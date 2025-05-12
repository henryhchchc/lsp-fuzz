use std::{borrow::Cow, hash::Hash, ops::Range};

use generation::{GrammarContext, GrammarContextLookup};
use libafl::{
    HasMetadata,
    inputs::HasTargetBytes,
    mutators::MutatorsTuple,
    state::{HasMaxSize, HasRand},
};
use libafl_bolts::{
    HasLen,
    ownedref::OwnedSlice,
    tuples::{Merge, NamedTuple},
};
use lsp_fuzz_grammars::Language;
use mutations::{
    NodeContentMutation, NodeTruncation, NodeUTF8Mutation, ReplaceNodeMutation,
    node_filters::HighlightedNodes,
    node_generators::{ChooseFromDerivations, EmptyNode, ExpandGrammar, MismatchedNode},
    text_document_selectors::RandomDoc,
};
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::{lsp::GeneratorsConfig, lsp_input::LspInput, utils::EitherTuple};

pub mod generation;
pub mod grammar;
pub mod mutations;

pub const LINE_SEP: u8 = b'\n';

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "_TextDocumentSerialized", into = "_TextDocumentSerialized")]
pub struct TextDocument {
    language: Language,
    content: Vec<u8>,
    // Skipped for serialization
    metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub parse_tree: tree_sitter::Tree,
}

impl Metadata {
    pub fn generate(language: Language, content: &[u8]) -> Self {
        let mut parser = language.tree_sitter_parser();
        let tree = parser
            .parse(content, None)
            .expect("Cannot parse input content");
        Self { parse_tree: tree }
    }
}

impl PartialEq for TextDocument {
    fn eq(&self, other: &Self) -> bool {
        self.language == other.language && self.content == other.content
    }
}

impl Eq for TextDocument {}

impl Hash for TextDocument {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.language.hash(state);
        self.content.hash(state);
    }
}

impl TextDocument {
    pub fn new(language: Language, content: Vec<u8>) -> Self {
        let metadata = Metadata::generate(language, &content);
        Self {
            content,
            language,
            metadata,
        }
    }

    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn update_metadata(&mut self) {
        let mut parser = self.language.tree_sitter_parser();
        self.metadata.parse_tree = parser
            .parse(&self.content, Some(&self.metadata.parse_tree))
            .expect("Parsing should not fail");
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &[u8]> {
        self.content.as_slice().split(|&it| it == LINE_SEP)
    }

    pub const fn content(&self) -> &[u8] {
        self.content.as_slice()
    }
}

pub trait GrammarBasedMutation {
    fn language(&self) -> Language;
    fn parse_tree(&self) -> &tree_sitter::Tree;
    fn fragment(&self, range: Range<usize>) -> &[u8];
    fn edit<E>(&mut self, edit: E) -> tree_sitter::InputEdit
    where
        E: FnOnce(&mut Vec<u8>) -> tree_sitter::InputEdit;

    fn splice(
        &mut self,
        range: tree_sitter::Range,
        new_content: Vec<u8>,
    ) -> tree_sitter::InputEdit {
        self.edit(|content| {
            let byte_range = range.start_byte..range.end_byte;
            let new_content_len = new_content.len();
            // Update the content
            let _ = content.splice(byte_range, new_content);
            let replacement = &content[range.start_byte..][..new_content_len];

            edit_for_node_replacement(range, replacement)
        })
    }
}

impl GrammarBasedMutation for TextDocument {
    fn edit<E>(&mut self, edit: E) -> tree_sitter::InputEdit
    where
        E: FnOnce(&mut Vec<u8>) -> tree_sitter::InputEdit,
    {
        let input_edit = edit(&mut self.content);
        self.metadata.parse_tree.edit(&input_edit);
        self.update_metadata();
        input_edit
    }

    fn language(&self) -> Language {
        self.language
    }

    fn fragment(&self, range: Range<usize>) -> &[u8] {
        &self.content[range]
    }

    fn parse_tree(&self) -> &tree_sitter::Tree {
        &self.metadata.parse_tree
    }
}

fn edit_for_node_replacement(
    range: tree_sitter::Range,
    replacement: &[u8],
) -> tree_sitter::InputEdit {
    let (delta_rows, delta_cols) = measure_fragment::<LINE_SEP>(replacement);
    let (start_position, old_end_position) = (range.start_point, range.end_point);
    let new_end_position = tree_sitter::Point {
        row: old_end_position.row + delta_rows,
        column: if delta_rows == 0 {
            old_end_position.column + delta_cols
        } else {
            delta_cols
        },
    };
    tree_sitter::InputEdit {
        start_byte: range.start_byte,
        old_end_byte: range.end_byte,
        new_end_byte: range.start_byte + replacement.len(),
        start_position,
        old_end_position,
        new_end_position,
    }
}

pub fn measure_fragment<const LINE_SEP: u8>(fragment: &[u8]) -> (usize, usize) {
    let mut rows = 0;
    let mut cols = 0;
    for &byte in fragment.iter().rev() {
        if byte == LINE_SEP {
            rows += 1;
        }
        if rows == 0 {
            cols += 1;
        }
    }
    (rows, cols)
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

type ReplaceNodeInRandomRoc<'a, NodeSel, NodeGen> =
    ReplaceNodeMutation<'a, RandomDoc, NodeSel, NodeGen>;
type NodeMutationInRandomDoc<'a, Mut, NodeSel> = NodeContentMutation<'a, Mut, RandomDoc, NodeSel>;

pub fn text_document_mutations<'g, State>(
    grammar_lookup: &'g GrammarContextLookup,
    genearators_config: &GeneratorsConfig,
) -> impl MutatorsTuple<LspInput, State> + NamedTuple + use<'g, State>
where
    State: HasRand + HasMaxSize + HasMetadata,
{
    use mutations::node_filters::NodesThat;

    let any_node = NodesThat::new(|_: &tree_sitter::Node<'_>| true);
    let terminal_node = NodesThat::new(|it: &tree_sitter::Node<'_>| it.child_count() == 0);
    let remove_comment = ReplaceNodeInRandomRoc::new(
        grammar_lookup,
        HighlightedNodes::new("comment".to_owned()),
        EmptyNode,
    );
    let correct_code_mutations = tuple_list![
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ChooseFromDerivations),
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ChooseFromDerivations),
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ExpandGrammar),
        ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, ExpandGrammar),
        remove_comment.clone(),
        remove_comment.clone(),
        remove_comment,
    ];
    if genearators_config.invalid_code {
        let recover_from_error = ReplaceNodeInRandomRoc::new(
            grammar_lookup,
            NodesThat::new(|it: &tree_sitter::Node<'_>| it.is_error()),
            ChooseFromDerivations,
        );
        let produce_missing_node = ReplaceNodeInRandomRoc::new(
            grammar_lookup,
            NodesThat::new(|it: &tree_sitter::Node<'_>| it.is_missing()),
            ChooseFromDerivations,
        );
        let generate_mismatched =
            ReplaceNodeInRandomRoc::new(grammar_lookup, any_node, MismatchedNode);
        let terminal_truncation =
            NodeMutationInRandomDoc::new(NodeTruncation, grammar_lookup, terminal_node);
        let utf8_mutation =
            NodeMutationInRandomDoc::new(NodeUTF8Mutation, grammar_lookup, terminal_node);
        let incorrect_code_mutations = tuple_list![
            recover_from_error,
            produce_missing_node,
            generate_mismatched,
            terminal_truncation,
            utf8_mutation,
            ReplaceNodeInRandomRoc::new(grammar_lookup, terminal_node, EmptyNode),
        ];
        EitherTuple::Left(correct_code_mutations.merge(incorrect_code_mutations))
    } else {
        EitherTuple::Right(correct_code_mutations)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "TextDocument")]
struct _TextDocumentSerialized {
    language: Language,
    content: Vec<u8>,
}

impl From<TextDocument> for _TextDocumentSerialized {
    fn from(document: TextDocument) -> Self {
        _TextDocumentSerialized {
            language: document.language,
            content: document.content,
        }
    }
}

impl From<_TextDocumentSerialized> for TextDocument {
    fn from(serialized: _TextDocumentSerialized) -> Self {
        Self::new(serialized.language, serialized.content)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_measure_fragment() {
        // Test case 1: Single line, no separators
        let fragment = b"hello";
        let (rows, cols) = measure_fragment::<LINE_SEP>(fragment);
        assert_eq!(rows, 0);
        assert_eq!(cols, 5);

        // Test case 2: Two lines
        let fragment = b"hello\nworld";
        let (rows, cols) = measure_fragment::<LINE_SEP>(fragment);
        assert_eq!(rows, 1);
        assert_eq!(cols, 5);

        // Test case 3: Ends with separator
        let fragment = b"hello\nworld\n";
        let (rows, cols) = measure_fragment::<LINE_SEP>(fragment);
        assert_eq!(rows, 2);
        assert_eq!(cols, 0);

        // Test case 4: Empty fragment
        let fragment = b"";
        let (rows, cols) = measure_fragment::<LINE_SEP>(fragment);
        assert_eq!(rows, 0);
        assert_eq!(cols, 0);

        // Test case 5: Three lines
        let fragment = b"hello\nworld\nrust";
        let (rows, cols) = measure_fragment::<LINE_SEP>(fragment);
        assert_eq!(rows, 2);
        assert_eq!(cols, 4);
    }

    #[test]
    fn text_doc_lines() {
        let content = b"hello\nworld\nrust";
        let doc = TextDocument::new(Language::Rust, content.to_vec());
        let mut lines = doc.lines();
        assert_eq!(lines.next(), Some(b"hello".as_slice()));
        assert_eq!(lines.next(), Some(b"world".as_slice()));
        assert_eq!(lines.next(), Some(b"rust".as_slice()));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn text_doc_lines_tailing_empty() {
        let content = b"hello\nworld\nrust\n";
        let doc = TextDocument::new(Language::Rust, content.to_vec());
        let mut lines = doc.lines();
        assert_eq!(lines.next(), Some(b"hello".as_slice()));
        assert_eq!(lines.next(), Some(b"world".as_slice()));
        assert_eq!(lines.next(), Some(b"rust".as_slice()));
        assert_eq!(lines.next(), Some(b"".as_slice()));
        assert_eq!(lines.next(), None);
    }
}
