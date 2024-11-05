use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use libafl::{inputs::HasTargetBytes, mutators::MutatorsTuple, state::HasRand, SerdeAny};
use libafl_bolts::{ownedref::OwnedSlice, tuples::NamedTuple, HasLen};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

pub mod grammars;
pub mod mutations;

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

pub const fn text_document_mutations<S>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<TextDocument, S> + NamedTuple + use<'_, S>
where
    S: HasRand,
{
    use mutations::*;
    tuple_list![
        ReplaceSubTreeWithDerivation::new(grammar_lookup),
        RemoveErrorNode::new(grammar_lookup),
        GenerateMissingNode::new(grammar_lookup),
        ReplaceNodeWithGenerated::new(grammar_lookup),
        DropRandomNode::new(grammar_lookup),
    ]
}
