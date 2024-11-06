use std::borrow::Cow;

use libafl::{
    mutators::{MutationResult, Mutator},
    state::HasRand,
};
use libafl_bolts::{rands::Rand, Named};

use super::{grammars::tree::NodeIter, GrammarContextLookup, TextDocument};

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
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.root_node().iter_depth_first();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let fragments = grammar_ctx.derivation_fragment(selected_node.kind());
        let Some(selected_fragment) = state.rand_mut().choose(fragments) else {
            return Ok(MutationResult::Skipped);
        };
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

impl<S> Mutator<TextDocument, S> for RemoveErrorNode<'_>
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
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.root_node().iter_breadth_first();
        let error_nodes = nodes.filter(|node| node.is_error());
        let Some(selected_node) = state.rand_mut().choose(error_nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_range = selected_node.range();
        input.splice(node_range, Vec::new(), grammar_ctx);
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

impl<S> Mutator<TextDocument, S> for GenerateMissingNode<'_>
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
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.root_node().iter_breadth_first();
        let missing_nodes: Vec<_> = nodes.filter(|node| node.is_missing()).collect();
        let Some(selected_node) = state.rand_mut().choose(&missing_nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_kind = selected_node.kind();
        let fragment = match grammar_ctx.generate_node(node_kind, state.rand_mut(), Some(5)) {
            Ok(fragments) => fragments,
            _ => return Ok(MutationResult::Skipped),
            // Err(DerivationError::DepthLimitReached) => return Ok(MutationResult::Skipped),
            // Err(DerivationError::InvalidGrammar) => {
            //     return Err(libafl::Error::illegal_state("Invalid grammar"))
            // }
        };
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

impl<'a, S> Mutator<TextDocument, S> for ReplaceNodeWithGenerated<'a>
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
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.root_node().iter_depth_first();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_kind = selected_node.kind();
        let fragment = match grammar_ctx.generate_node(node_kind, state.rand_mut(), Some(5)) {
            Ok(fragments) => fragments,
            _ => return Ok(MutationResult::Skipped),
        };
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

impl<S> Mutator<TextDocument, S> for DropRandomNode<'_>
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
        let parse_tree = input.parse_tree(grammar_ctx);
        let nodes = parse_tree.root_node().iter_depth_first();
        let Some(selected_node) = state.rand_mut().choose(nodes) else {
            return Ok(MutationResult::Skipped);
        };
        let node_range = selected_node.range();
        input.splice(node_range, Vec::new(), grammar_ctx);
        Ok(MutationResult::Mutated)
    }
}
