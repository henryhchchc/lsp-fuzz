use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use grammars::tree::NodeIter;
use libafl::{
    inputs::{HasTargetBytes, MutVecInput},
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl::{mutators::MutatorsTuple, SerdeAny};
use libafl_bolts::{ownedref::OwnedSlice, rands::Rand, tuples::NamedTuple, HasLen, Named};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

pub mod grammars;

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Hash,
    derive_more::Display,
    derive_more::FromStr,
)]
pub enum Language {
    C,
    Rust,
}

impl Language {
    pub fn file_extensions<'a>(&self) -> HashSet<&'a str> {
        match self {
            Self::C => HashSet::from(["c", "cc", "h"]),
            Self::Rust => HashSet::from(["rs"]),
        }
    }

    pub fn tree_sitter_parser(&self) -> tree_sitter::Parser {
        let language = match self {
            Self::C => tree_sitter_c::LANGUAGE,
            Self::Rust => tree_sitter_rust::LANGUAGE,
        };
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language.into())
            .expect("Fail to initialize parser");
        parser
    }

    fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::C => tree_sitter::Language::new(tree_sitter_c::LANGUAGE),
            Self::Rust => tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, SerdeAny, derive_more::Deref)]
pub struct GrammarContextLookup {
    #[deref]
    inner: HashMap<Language, grammars::GrammarContext>,
}

impl FromIterator<(Language, grammars::GrammarContext)> for GrammarContextLookup {
    fn from_iter<T: IntoIterator<Item = (Language, grammars::GrammarContext)>>(iter: T) -> Self {
        Self {
            inner: HashMap::from_iter(iter),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocument {
    content: Vec<u8>,
    language: Language,
}

impl TextDocument {
    pub fn new(content: Vec<u8>, language: Language) -> Self {
        Self { content, language }
    }

    pub fn content_bytes_mut(&mut self) -> MutVecInput<'_> {
        MutVecInput::from(&mut self.content)
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }
}

impl HasTargetBytes for TextDocument {
    fn target_bytes(&self) -> OwnedSlice<'_, u8> {
        OwnedSlice::from(&self.content)
    }
}

impl HasLen for TextDocument {
    fn len(&self) -> usize {
        self.content.len()
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct ReplaceSubTreeWithDerivation<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for ReplaceSubTreeWithDerivation<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ReplaceSubTreeWithDerivation");
        &NAME
    }
}

impl<S> Mutator<TextDocument, S> for ReplaceSubTreeWithDerivation<'_>
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut TextDocument,
    ) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language) else {
            return Ok(MutationResult::Skipped);
        };
        let parse_tree = grammar_ctx
            .parse_source_code(&input.content)
            .map_err(|_| libafl::Error::unknown("Fail to parse input"))?;
        let nodes = parse_tree.root_node().iter_depth_first();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let byte_range = selected_node.byte_range();
        let node_kind = selected_node.kind();
        let fragments = grammar_ctx.derivation_fragment(node_kind);
        let Some(selected_fragment) = state.rand_mut().choose(fragments) else {
            return Ok(MutationResult::Skipped);
        };
        let _ = input
            .content
            .splice(byte_range, selected_fragment.iter().copied());
        Ok(MutationResult::Mutated)
    }
}

pub const fn text_document_mutations<S>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<TextDocument, S> + NamedTuple + use<'_, S>
where
    S: HasRand,
{
    tuple_list![ReplaceSubTreeWithDerivation::new(grammar_lookup)]
}
