use std::{borrow::Cow, marker::PhantomData};

use itertools::Itertools;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, HasLen, Named};
use node_filters::{AnyNode, ErrorNode, MissingNode};
use node_generators::{ChooseFromDerivations, EmptyNode, GenerateNodeWithGrammar};

use super::{
    grammars::{tree::TreeIter, GrammarContext},
    GrammarBasedMutation, GrammarContextLookup,
};

const MAX_DOCUMENT_SIZE: usize = libafl::state::DEFAULT_MAX_SIZE;

#[derive(Debug)]
pub struct ReplaceNodeMutation<'a, NF, GEN> {
    grammar_lookup: &'a GrammarContextLookup,
    name: Cow<'static, str>,
    _node_filter: PhantomData<NF>,
    _generator: PhantomData<GEN>,
}

impl<'a, NF, GEN> ReplaceNodeMutation<'a, NF, GEN>
where
    NF: NodeFilter,
    GEN: NodeGenerator,
{
    pub fn new(grammar_lookup: &'a GrammarContextLookup) -> Self {
        let name = Cow::Owned("Replace".to_owned() + NF::NAME + "With" + GEN::NAME);
        Self {
            grammar_lookup,
            name,
            _node_filter: PhantomData,
            _generator: PhantomData,
        }
    }
}

impl<NF, GEN> Named for ReplaceNodeMutation<'_, NF, GEN> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        &self.name
    }
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

impl<S, I, NF, GEN> Mutator<I, S> for ReplaceNodeMutation<'_, NF, GEN>
where
    NF: NodeFilter,
    GEN: NodeGenerator,
    I: GrammarBasedMutation + HasLen,
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let input_len = input.len();
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree
            .iter()
            .filter(|&it| NF::filter_node(it, grammar_ctx));
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let Some(new_fragement) = GEN::generate_node(selected_node, grammar_ctx, state.rand_mut())
        else {
            return Ok(MutationResult::Skipped);
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if input_len - node_len + new_fragement.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        input.splice(node_range, new_fragement.to_vec(), grammar_ctx);
        Ok(MutationResult::Mutated)
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
    pub struct GenerateNodeWithGrammar;

    impl NodeGenerator for GenerateNodeWithGrammar {
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

pub type ReplaceSubTreeWithDerivation<'a> = ReplaceNodeMutation<'a, AnyNode, ChooseFromDerivations>;
pub type ReplaceNodeWithGenerated<'a> = ReplaceNodeMutation<'a, AnyNode, GenerateNodeWithGrammar>;
pub type GenerateMissingNode<'a> = ReplaceNodeMutation<'a, MissingNode, GenerateNodeWithGrammar>;

pub type RemoveErrorNode<'a> = ReplaceNodeMutation<'a, ErrorNode, EmptyNode>;
pub type DropRandomNode<'a> = ReplaceNodeMutation<'a, AnyNode, EmptyNode>;

#[derive(Debug, derive_more::Constructor)]
pub struct DropUncoveredArea<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for DropUncoveredArea<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropUncoveredArea");
        &NAME
    }
}

impl<S, I> Mutator<I, S> for DropUncoveredArea<'_>
where
    S: HasRand,
    I: GrammarBasedMutation,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let parse_tree = input.parse_tree(grammar_ctx);
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

        input.edit(grammar_ctx, |content| {
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
