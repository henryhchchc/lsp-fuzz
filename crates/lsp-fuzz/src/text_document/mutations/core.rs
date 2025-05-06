use std::{option::Option, vec::Vec};

use lsp_types::Uri;

use crate::{
    lsp_input::LspInput,
    text_document::{TextDocument, generation::GrammarContext},
};

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
