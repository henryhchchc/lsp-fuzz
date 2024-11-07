use std::borrow::Cow;

use itertools::Itertools;
use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, HasLen, Named};

use super::{grammars::tree::TreeIter, GrammarBasedMutation, GrammarContextLookup};

const MAX_DOCUMENT_SIZE: usize = 100_000;

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

impl<S, I> Mutator<I, S> for ReplaceSubTreeWithDerivation<'_>
where
    S: HasRand,
    I: GrammarBasedMutation + HasLen,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let input_len = input.len();
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.iter();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let fragments = grammar_ctx.derivation_fragment(selected_node.kind());
        let Some(selected_fragment) = state.rand_mut().choose(fragments) else {
            return Ok(MutationResult::Skipped);
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if input_len - node_len + selected_fragment.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        input.splice(node_range, selected_fragment.to_vec(), grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct RemoveErrorNode<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for RemoveErrorNode<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("RemoveErrorNode");
        &NAME
    }
}

impl<S, I> Mutator<I, S> for RemoveErrorNode<'_>
where
    S: HasRand,
    I: GrammarBasedMutation,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.iter();
        let error_nodes = nodes.filter(|node| node.is_error());
        let Some(selected_node) = state.rand_mut().choose(error_nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_range = selected_node.range();
        input.drain(node_range, grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct GenerateMissingNode<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for GenerateMissingNode<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("GenerateMissingNode");
        &NAME
    }
}

impl<I, S> Mutator<I, S> for GenerateMissingNode<'_>
where
    S: HasRand,
    I: GrammarBasedMutation + HasLen,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let input_len = input.len();
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.iter();
        let missing_nodes: Vec<_> = nodes.filter(|node| node.is_missing()).collect();
        let Some(selected_node) = state.rand_mut().choose(&missing_nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_kind = selected_node.kind();
        let fragment = match grammar_ctx.generate_node(node_kind, state.rand_mut(), Some(5)) {
            Ok(fragments) => fragments,
            _ => return Ok(MutationResult::Skipped),
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if input_len - node_len + fragment.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        input.splice(node_range, fragment, grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct ReplaceNodeWithGenerated<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for ReplaceNodeWithGenerated<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ReplaceNodeWithGenerated");
        &NAME
    }
}

impl<'a, I, S> Mutator<I, S> for ReplaceNodeWithGenerated<'a>
where
    S: HasRand,
    I: GrammarBasedMutation + HasLen,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let input_len = input.len();
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.iter();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_kind = selected_node.kind();
        let fragment = match grammar_ctx.generate_node(node_kind, state.rand_mut(), Some(5)) {
            Ok(fragments) => fragments,
            _ => return Ok(MutationResult::Skipped),
        };
        let node_len = selected_node.end_byte() - selected_node.start_byte();
        if input_len - node_len + fragment.len() > MAX_DOCUMENT_SIZE {
            return Ok(MutationResult::Skipped);
        }
        let node_range = selected_node.range();
        input.splice(node_range, fragment, grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

#[derive(Debug, derive_more::Constructor)]
pub struct DropRandomNode<'a> {
    grammar_lookup: &'a GrammarContextLookup,
}

impl Named for DropRandomNode<'_> {
    fn name(&self) -> &std::borrow::Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("DropRandomNode");
        &NAME
    }
}

impl<S, I> Mutator<I, S> for DropRandomNode<'_>
where
    S: HasRand,
    I: GrammarBasedMutation,
{
    fn mutate(&mut self, state: &mut S, input: &mut I) -> Result<MutationResult, libafl::Error> {
        let Some(grammar_ctx) = self.grammar_lookup.get(&input.language()) else {
            return Ok(MutationResult::Skipped);
        };
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.iter();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_range = selected_node.range();
        input.drain(node_range, grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}

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
