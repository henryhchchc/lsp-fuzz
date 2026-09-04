use std::{option::Option, vec::Vec};

use crate::text_document::{TextDocument, generation::GrammarContext};

pub trait NodeSelector<State> {
    const NAME: &'static str;
    fn select_node<'t>(
        &self,
        doc: &'t mut TextDocument,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<tree_sitter::Node<'t>>;
}

pub trait NodeGenerator<State> {
    const NAME: &'static str;
    fn generate_node(
        &self,
        node: tree_sitter::Node<'_>,
        grammar_context: &GrammarContext,
        state: &mut State,
    ) -> Option<Vec<u8>>;
}

pub trait NodeContentMutator<State> {
    fn mutate(&self, content: &mut Vec<u8>, state: &mut State);
}
