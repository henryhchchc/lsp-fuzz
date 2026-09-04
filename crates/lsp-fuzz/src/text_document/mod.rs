use std::{borrow::Cow, hash::Hash, ops::Range};

use ahash::{HashMap, HashSet};
use generation::GrammarContext;
use grammar::tree_sitter::TreeIter;
use itertools::Itertools;
use libafl::inputs::HasTargetBytes;
use libafl_bolts::{HasLen, ownedref::OwnedSlice};
use lsp_fuzz_grammars::Language;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

pub(crate) mod conversions;
pub mod generation;
pub mod grammar;
pub mod mutations;

pub const LINE_SEP: u8 = b'\n';

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "TextDocumentSerialized", into = "TextDocumentSerialized")]
pub struct TextDocument {
    language: Language,
    content: Vec<u8>,
    // Skipped for serialization
    metadata: Metadata,
}

const SIGNATURE_LEVEL: usize = 3;

#[derive(Debug, Clone)]
pub struct Metadata {
    pub parse_tree: tree_sitter::Tree,
    pub node_type_ranges: HashMap<u16, HashSet<tree_sitter::Range>>,
    pub node_signatures: HashMap<SmallVec<[u16; SIGNATURE_LEVEL]>, HashSet<tree_sitter::Point>>,
}

impl Metadata {
    /// Builds parse metadata for a document from its language and current content.
    ///
    /// # Panics
    ///
    /// Panics if tree-sitter fails to produce a parse tree for `content`.
    #[must_use]
    pub fn generate(language: Language, content: &[u8]) -> Self {
        let mut parser = language.tree_sitter_parser();
        let parse_tree = parser
            .parse(content, None)
            .expect("Cannot parse input content");
        let mut result = Self {
            parse_tree,
            node_type_ranges: HashMap::default(),
            node_signatures: HashMap::default(),
        };
        result.update_node_info();
        result
    }

    fn update_node_info(&mut self) {
        self.node_type_ranges = self
            .parse_tree
            .root_node()
            .iter()
            .into_group_map_by(tree_sitter::Node::kind_id)
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|it| it.range()).collect()))
            .collect();
        self.node_signatures = self
            .parse_tree
            .root_node()
            .iter()
            .filter(|it| it.child_count() == 0)
            .into_group_map_by(|&it| {
                std::iter::successors(Some(it), tree_sitter::Node::parent)
                    .take(SIGNATURE_LEVEL)
                    .map(|it| it.kind_id())
                    .collect()
            })
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|it| it.start_position()).collect()))
            .collect();
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
    /// Creates a text document and eagerly computes parse-derived metadata.
    ///
    /// # Panics
    ///
    /// Panics if tree-sitter fails to produce an initial parse tree for `content`.
    #[must_use]
    pub fn new(language: Language, content: Vec<u8>) -> Self {
        let metadata = Metadata::generate(language, &content);
        Self {
            language,
            content,
            metadata,
        }
    }

    #[must_use]
    pub const fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Reparses the current content and refreshes cached node indexes.
    ///
    /// # Panics
    ///
    /// Panics if incremental reparsing fails.
    pub fn update_metadata(&mut self) {
        let mut parser = self.language.tree_sitter_parser();
        self.metadata.parse_tree = parser
            .parse(&self.content, Some(&self.metadata.parse_tree))
            .expect("Parsing should not fail");
        self.metadata.update_node_info();
    }

    #[must_use]
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    #[must_use]
    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &[u8]> {
        self.content.as_slice().split(|&it| it == LINE_SEP)
    }

    #[must_use]
    pub const fn content(&self) -> &[u8] {
        self.content.as_slice()
    }

    #[must_use]
    pub fn node_starts_in_range(&self, range: lsp_types::Range) -> Vec<tree_sitter::Point> {
        let start_point = tree_sitter::Point {
            row: range.start.line as usize,
            column: range.start.character as usize,
        };
        let end_point = tree_sitter::Point {
            row: range.end.line as usize,
            column: range.end.character as usize,
        };
        let Some(node) = self
            .parse_tree()
            .root_node()
            .descendant_for_point_range(start_point, end_point)
        else {
            return Vec::new();
        };
        node.iter().map(|it| it.start_position()).collect()
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

#[must_use]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "TextDocument")]
struct TextDocumentSerialized {
    language: Language,
    content: Vec<u8>,
}

impl From<TextDocument> for TextDocumentSerialized {
    fn from(document: TextDocument) -> Self {
        Self {
            language: document.language,
            content: document.content,
        }
    }
}

impl From<TextDocumentSerialized> for TextDocument {
    fn from(serialized: TextDocumentSerialized) -> Self {
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
