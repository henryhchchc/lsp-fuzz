use std::{borrow::Cow, marker::PhantomData};

use derive_new::new as New;
use itertools::Itertools;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{HasLen, Named, rands::Rand};
use lsp_types::Uri;
use node_filters::{AnyNode, ErrorNode, MissingNode};
use node_generators::{ChooseFromDerivations, EmptyNode, ExpandGrammar};
use text_document_selectors::RandomDoc;

use crate::lsp_input::LspInput;

use super::{
    GrammarBasedMutation, GrammarContextLookup, TextDocument,
    grammars::{GrammarContext, tree::TreeIter},
};

const MAX_DOCUMENT_SIZE: usize = libafl::state::DEFAULT_MAX_SIZE;

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, TS, NF, GEN> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    _text_doc_selector: PhantomData<TS>,
    _node_filter: PhantomData<NF>,
    _generator: PhantomData<GEN>,
}

impl<'a, TS, NF, GEN> ReplaceNodeMutation<'a, TS, NF, GEN>
where
    NF: NodeFilter,
    GEN: NodeGenerator,
{
    pub fn new(grammar_lookup: &'a GrammarContextLookup) -> Self {
        let name = Cow::Owned("Replace".to_owned() + NF::NAME + "With" + GEN::NAME);
        Self {
            grammar_lookup,
            name,
            _text_doc_selector: PhantomData,
            _node_filter: PhantomData,
            _generator: PhantomData,
        }
    }
}

impl<TS, NF, GEN> Named for ReplaceNodeMutation<'_, TS, NF, GEN> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &self.name
    }
}

pub trait TextDocumentSelector<State> {
    fn select_document<'i>(
        state: &mut State,
        input: &'i LspInput,
    ) -> Option<(Uri, &'i TextDocument)>;

    fn select_document_mut<'i>(
        state: &mut State,
        input: &'i mut LspInput,
    ) -> Option<(Uri, &'i mut TextDocument)>;
}

pub trait NodeFilter {
    const NAME: &'static str;
    fn filter_node(node: tree_sitter::Node<'_>, grammar_context: &GrammarContext) -> bool;
}

pub trait NodeGenerator {
    const NAME: &'static str;
    fn generate_node<R>(
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        rand: &mut R,
    ) -> Option<Vec<u8>>
    where
        R: Rand;
}

impl<State, TS, NF, GEN> Mutator<LspInput, State> for ReplaceNodeMutation<'_, TS, NF, GEN>
where
    TS: TextDocumentSelector<State>,
    NF: NodeFilter,
    GEN: NodeGenerator,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((_, doc)) = TS::select_document_mut(state, input) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(grammar_ctx) = self.grammar_lookup.get(doc.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let doc_len = doc.len();
        let parse_tree = doc.get_or_create_parse_tree(grammar_ctx);
        let nodes = parse_tree
            .iter()
            .filter(|&it| NF::filter_node(it, grammar_ctx));
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(new_fragment) = GEN::generate_node(selected_node, grammar_ctx, state.rand_mut())
        else {
            return Ok(MutationResult::Skipped);
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if doc_len - node_len + new_fragment.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        doc.splice(node_range, new_fragment.to_vec(), grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

pub mod text_document_selectors {
    use libafl::state::HasRand;
    use libafl_bolts::rands::Rand;
    use lsp_types::Uri;
    use std::{marker::PhantomData, option::Option};

    use crate::{lsp_input::LspInput, text_document::TextDocument};

    use super::TextDocumentSelector;

    #[derive(Debug)]
    pub struct RandomDoc<State>(PhantomData<State>);

    impl<State> TextDocumentSelector<State> for RandomDoc<State>
    where
        State: HasRand,
    {
        fn select_document<'i>(
            state: &mut State,
            input: &'i LspInput,
        ) -> Option<(Uri, &'i TextDocument)> {
            let iter = input.workspace.iter_files().filter_map(|(path, doc)| {
                doc.as_source_file().map(|doc| {
                    (
                        format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                        doc,
                    )
                })
            });
            state.rand_mut().choose(iter)
        }

        fn select_document_mut<'i>(
            state: &mut State,
            input: &'i mut LspInput,
        ) -> Option<(Uri, &'i mut TextDocument)> {
            let iter = input.workspace.iter_files_mut().filter_map(|(path, doc)| {
                doc.as_source_file_mut().map(|doc| {
                    (
                        format!("lsp-fuzz://{}", path.display()).parse().unwrap(),
                        doc,
                    )
                })
            });
            state.rand_mut().choose(iter)
        }
    }
}

pub mod node_filters {
    use crate::text_document::grammars::GrammarContext;

    use super::NodeFilter;

    #[derive(Debug)]
    pub struct ErrorNode;

    impl NodeFilter for ErrorNode {
        const NAME: &'static str = "ErrorNode";

        fn filter_node(node: tree_sitter::Node<'_>, _grammar_context: &GrammarContext) -> bool {
            node.is_error()
        }
    }

    #[derive(Debug)]
    pub struct AnyNode;

    impl NodeFilter for AnyNode {
        const NAME: &'static str = "AnyNode";
        fn filter_node(_node: tree_sitter::Node<'_>, _grammar_context: &GrammarContext) -> bool {
            true
        }
    }

    #[derive(Debug)]
    pub struct MissingNode;

    impl NodeFilter for MissingNode {
        const NAME: &'static str = "MissingNode";
        fn filter_node(node: tree_sitter::Node<'_>, _grammar_context: &GrammarContext) -> bool {
            node.is_missing()
        }
    }
}

pub mod node_generators {

    use std::{option::Option, vec::Vec};

    use libafl_bolts::rands::Rand;

    use crate::text_document::grammars::GrammarContext;

    use super::NodeGenerator;

    #[derive(Debug)]
    pub struct EmptyNode;

    impl NodeGenerator for EmptyNode {
        const NAME: &'static str = "AnEmptyNode";
        fn generate_node<R>(
            _node: tree_sitter::Node<'_>,
            _grammar_context: &GrammarContext,
            _rand: &mut R,
        ) -> Option<Vec<u8>>
        where
            R: Rand,
        {
            Some(Vec::new())
        }
    }

    #[derive(Debug)]
    pub struct ChooseFromDerivations;

    impl NodeGenerator for ChooseFromDerivations {
        const NAME: &'static str = "RandomDerivation";
        fn generate_node<R>(
            node: tree_sitter::Node<'_>,
            grammar_context: &GrammarContext,
            rand: &mut R,
        ) -> Option<Vec<u8>>
        where
            R: Rand,
        {
            let fragments = grammar_context.derivation_fragment(node.kind());
            rand.choose(fragments).map(|it| it.to_vec())
        }
    }

    #[derive(Debug)]
    pub struct ExpandGrammar;

    impl NodeGenerator for ExpandGrammar {
        const NAME: &'static str = "RandomGeneration";
        fn generate_node<R>(
            node: tree_sitter::Node<'_>,
            grammar_context: &GrammarContext,
            rand: &mut R,
        ) -> Option<Vec<u8>>
        where
            R: Rand,
        {
            let fragment = grammar_context
                .generate_node(node.kind(), rand, Some(5))
                .ok()?;
            Some(fragment)
        }
    }
}

pub type ReplaceSubTreeWithDerivation<'a, State> =
    ReplaceNodeMutation<'a, RandomDoc<State>, AnyNode, ChooseFromDerivations>;
pub type ReplaceNodeWithGenerated<'a, State> =
    ReplaceNodeMutation<'a, RandomDoc<State>, AnyNode, ExpandGrammar>;
pub type GenerateMissingNode<'a, State> =
    ReplaceNodeMutation<'a, RandomDoc<State>, MissingNode, ExpandGrammar>;

pub type RemoveErrorNode<'a, State> =
    ReplaceNodeMutation<'a, RandomDoc<State>, ErrorNode, EmptyNode>;
pub type DropRandomNode<'a, State> = ReplaceNodeMutation<'a, RandomDoc<State>, AnyNode, EmptyNode>;

#[derive(Debug, New)]
pub struct DropUncoveredArea<'a, TS> {
    grammar_lookup: &'a GrammarContextLookup,
    _doc_selector: PhantomData<TS>,
}

impl<TS> Named for DropUncoveredArea<'_, TS> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropUncoveredArea");
        &NAME
    }
}

impl<State, TS> Mutator<LspInput, State> for DropUncoveredArea<'_, TS>
where
    TS: TextDocumentSelector<State>,
    State: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut State,
        input: &mut LspInput,
    ) -> Result<MutationResult, libafl::Error> {
        let Some((_path, doc)) = TS::select_document_mut(state, input) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(grammar_ctx) = self.grammar_lookup.get(doc.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let parse_tree = doc.get_or_create_parse_tree(grammar_ctx);
        let covered_areas = parse_tree
            .iter()
            .filter(|it| it.child_count() > 0)
            .map(|it| it.range())
            .sorted_by_key(|it| it.start_byte)
            .tuple_windows()
            .filter(|(prev, curr)| prev.end_byte < curr.start_byte);

        let Some((prev, curr)) = state.rand_mut().choose(covered_areas) else {
            return Ok(MutationResult::Skipped);
        };

        doc.edit(grammar_ctx, |content| {
            let remove_range = prev.end_byte..curr.start_byte;
            let _ = content.drain(remove_range);
            tree_sitter::InputEdit {
                start_byte: prev.end_byte,
                old_end_byte: curr.start_byte,
                new_end_byte: prev.end_byte,
                start_position: prev.end_point,
                old_end_position: curr.start_point,
                new_end_position: prev.end_point,
            }
        });

        Ok(MutationResult::Mutated)
    }
}
