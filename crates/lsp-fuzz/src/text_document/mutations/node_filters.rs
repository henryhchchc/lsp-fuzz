use derive_new::new as New;
use libafl::state::HasRand;
use libafl_bolts::rands::Rand;

use super::NodeSelector;
use crate::text_document::{
    GrammarBasedMutation, GrammarContext, TextDocument,
    grammar::tree_sitter::{CapturesIterator, TreeIter},
};

#[derive(Debug, Clone, Copy, New)]
pub struct NodesThat<Predicate> {
    predicate: Predicate,
}

impl<State, Predicate> NodeSelector<State> for NodesThat<Predicate>
where
    State: HasRand,
    Predicate: for<'t> Fn(&tree_sitter::Node<'t>) -> bool,
{
    const NAME: &'static str = "NotesThat<Pred>";

    fn select_node<'t>(
        &self,
        doc: &'t mut TextDocument,
        _grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<tree_sitter::Node<'t>> {
        let parse_tree = doc.parse_tree();
        let candidate_nodes = parse_tree.iter().filter(&self.predicate);
        state.rand_mut().choose(candidate_nodes)
    }
}

#[derive(Debug, Clone, New)]
pub struct HighlightedNodes {
    capture_group_name: String,
}

impl<State> NodeSelector<State> for HighlightedNodes
where
    State: HasRand,
{
    const NAME: &'static str = "Highlighted";

    fn select_node<'t>(
        &self,
        doc: &'t mut TextDocument,
        _grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<tree_sitter::Node<'t>> {
        let captured_nodes = CapturesIterator::new(doc, &self.capture_group_name)?;
        state.rand_mut().choose(captured_nodes)
    }
}
