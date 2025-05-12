use std::{option::Option, vec::Vec};

use libafl::{HasMetadata, state::HasRand};
use libafl_bolts::rands::Rand;

use super::NodeGenerator;
use crate::text_document::generation::{
    GrammarContext, NamedNodeGenerator, RandomRuleSelectionStrategy,
};

#[derive(Debug, Clone, Copy)]
pub struct EmptyNode;

impl<State> NodeGenerator<State> for EmptyNode {
    const NAME: &'static str = "AnEmptyNode";
    fn generate_node(
        &self,
        _node: tree_sitter::Node<'_>,
        _grammar_context: &GrammarContext,
        _state: &mut State,
    ) -> Option<Vec<u8>> {
        Some(Vec::new())
    }
}

#[derive(Debug)]
pub struct ChooseFromDerivations;

impl<State> NodeGenerator<State> for ChooseFromDerivations
where
    State: HasRand,
{
    const NAME: &'static str = "RandomDerivation";
    fn generate_node(
        &self,
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<Vec<u8>> {
        let fragments = grammar_context.node_fragments(node.kind());
        state.rand_mut().choose(fragments).map(|it| it.to_vec())
    }
}

#[derive(Debug)]
pub struct ExpandGrammar;

impl<State> NodeGenerator<State> for ExpandGrammar
where
    State: HasRand + HasMetadata,
{
    const NAME: &'static str = "RandomGeneration";
    fn generate_node(
        &self,
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<Vec<u8>> {
        let selection_strategy = RandomRuleSelectionStrategy;
        let generator = NamedNodeGenerator::new(grammar_context, selection_strategy);
        let fragment = generator.generate(node.kind(), state).ok()?;
        Some(fragment)
    }
}

#[derive(Debug)]
pub struct MismatchedNode;

impl<State> NodeGenerator<State> for MismatchedNode
where
    State: HasRand + HasMetadata,
{
    const NAME: &'static str = "MismatchedNode";

    fn generate_node(
        &self,
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<Vec<u8>> {
        let mismatched_rules = grammar_context
            .grammar
            .derivation_rules()
            .keys()
            .filter(|&it| it != node.kind());
        let node_kind = state.rand_mut().choose(mismatched_rules)?;
        let selection_strategy = RandomRuleSelectionStrategy;
        let generator = NamedNodeGenerator::new(grammar_context, selection_strategy);
        let fragment = generator.generate(node_kind, state).ok()?;
        Some(fragment)
    }
}
