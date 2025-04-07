use std::{borrow::Cow, collections::HashMap, hash::Hash, ops::Range};

use grammars::{GrammarContext, tree::TreeIter};
use libafl::{
    SerdeAny,
    inputs::HasTargetBytes,
    mutators::MutatorsTuple,
    state::{HasMaxSize, HasRand},
};
use libafl_bolts::{HasLen, ownedref::OwnedSlice, tuples::NamedTuple};
use lsp_fuzz_grammars::Language;
use mutations::text_document_selectors::RandomDoc;
use serde::{Deserialize, Serialize};
use tuple_list::tuple_list;

use crate::lsp_input::LspInput;

pub mod grammars;
pub mod mutations;
pub mod token_novelty;

pub const LINE_SEP: u8 = b'\n';

#[derive(Debug, Serialize, Deserialize, SerdeAny)]
pub struct GrammarContextLookup {
    inner: HashMap<Language, grammars::GrammarContext>,
}

impl GrammarContextLookup {
    pub fn get(&self, language: Language) -> Option<&GrammarContext> {
        self.inner.get(&language)
    }
    
    pub fn iter(&self) -> impl Iterator<Item = &GrammarContext> {
        self.inner.values()
    }
}

impl FromIterator<grammars::GrammarContext> for GrammarContextLookup {
    fn from_iter<T: IntoIterator<Item = grammars::GrammarContext>>(iter: T) -> Self {
        let inner = iter.into_iter().map(|gc| (gc.language(), gc)).collect();
        Self { inner }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocument {
    language: Language,
    content: Vec<u8>,
    #[serde(skip)]
    parse_tree: Option<tree_sitter::Tree>,
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
    pub fn new(content: Vec<u8>, language: Language) -> Self {
        Self {
            content,
            language,
            parse_tree: None,
        }
    }

    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.content)
    }

    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &[u8]> {
        self.content.as_slice().split(|&it| it == LINE_SEP)
    }

    pub fn lsp_range(&self) -> lsp_types::Range {
        let start = lsp_types::Position::default();
        let end = self
            .lines()
            .enumerate()
            .last()
            .map(|(line_idx, line)| lsp_types::Position::new(line_idx as _, line.len() as _))
            .unwrap_or_default();
        lsp_types::Range::new(start, end)
    }

    pub fn terminal_ranges(&self) -> impl Iterator<Item = tree_sitter::Range> + '_ {
        self.parse_tree.iter().flat_map(|parse_tree| {
            parse_tree
                .iter()
                .filter(|it| it.child_count() == 0)
                .map(|it| it.range())
        })
    }

    pub fn generate_parse_tree(&mut self, grammar_context: &GrammarContext) {
        let mut parser = grammar_context.create_parser();
        self.parse_tree = Some(
            parser
                .parse(&self.content, None)
                .expect("Parsing should not fail"),
        );
    }

    fn update_parse_tree(
        &mut self,
        input_edit: tree_sitter::InputEdit,
        grammar_context: &GrammarContext,
    ) {
        let mut parser = grammar_context.create_parser();
        if let Some(ref mut parse_tree) = self.parse_tree {
            parse_tree.edit(&input_edit);
        }
        self.parse_tree = parser.parse(&self.content, self.parse_tree.as_ref());
    }

    const fn parse_tree(&self) -> Option<&tree_sitter::Tree> {
        self.parse_tree.as_ref()
    }
}

pub trait GrammarBasedMutation {
    fn language(&self) -> Language;
    fn get_or_create_parse_tree(&mut self, grammar_context: &GrammarContext) -> &tree_sitter::Tree;
    fn fragment(&self, range: Range<usize>) -> &[u8];
    fn edit<E>(&mut self, grammar_context: &GrammarContext, edit: E)
    where
        E: FnOnce(&mut Vec<u8>) -> tree_sitter::InputEdit;

    fn splice(
        &mut self,
        range: tree_sitter::Range,
        new_content: Vec<u8>,
        grammar_context: &GrammarContext,
    ) {
        self.edit(grammar_context, |content| {
            let byte_range = range.start_byte..range.end_byte;
            let replacement_range = range.start_byte..(range.start_byte + new_content.len());
            // Update the content
            let _ = content.splice(byte_range, new_content);
            let replacement = &content[replacement_range];

            edit_for_node_replacement(range, replacement)
        });
    }
}

impl GrammarBasedMutation for TextDocument {
    fn edit<E>(&mut self, grammar_context: &GrammarContext, edit: E)
    where
        E: FnOnce(&mut Vec<u8>) -> tree_sitter::InputEdit,
    {
        let input_edit = edit(&mut self.content);
        self.update_parse_tree(input_edit, grammar_context);
    }

    fn language(&self) -> Language {
        self.language
    }

    fn fragment(&self, range: Range<usize>) -> &[u8] {
        &self.content[range]
    }

    fn get_or_create_parse_tree(&mut self, grammar_context: &GrammarContext) -> &tree_sitter::Tree {
        self.parse_tree.get_or_insert_with(|| {
            let mut parser = grammar_context.create_parser();
            parser.parse(&self.content, None).unwrap()
        })
    }
}

fn edit_for_node_replacement(
    range: tree_sitter::Range,
    replacement: &[u8],
) -> tree_sitter::InputEdit {
    let (delta_rows, delta_cols) = measure_fragment::<b'\n'>(replacement);
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

pub fn text_document_mutations<State>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<LspInput, State> + NamedTuple + use<'_, State>
where
    State: HasRand + HasMaxSize,
{
    use mutations::*;
    tuple_list![
        ReplaceSubTreeWithDerivation::new(grammar_lookup),
        RemoveErrorNode::new(grammar_lookup),
        GenerateMissingNode::new(grammar_lookup),
        ReplaceNodeWithGenerated::new(grammar_lookup),
        DropRandomNode::new(grammar_lookup),
        DropUncoveredArea::<'_, RandomDoc<State>>::new(grammar_lookup),
    ]
}

pub fn text_document_reductions<State>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<LspInput, State> + NamedTuple + use<'_, State>
where
    State: HasRand + HasMaxSize,
{
    use mutations::*;
    tuple_list![
        RemoveErrorNode::new(grammar_lookup),
        DropRandomNode::new(grammar_lookup),
        DropUncoveredArea::<'_, RandomDoc<State>>::new(grammar_lookup),
    ]
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_measure_fragment_single_line() {
        // Test case 1: Single line, no separators
        let fragment = b"hello";
        let (rows, cols) = measure_fragment::<b'\n'>(fragment);
        assert_eq!(rows, 0);
        assert_eq!(cols, 5);
    }

    #[test]
    fn test_measure_fragment_two_lines() {
        // Test case 2: Two lines
        let fragment = b"hello\nworld";
        let (rows, cols) = measure_fragment::<b'\n'>(fragment);
        assert_eq!(rows, 1);
        assert_eq!(cols, 5);
    }

    #[test]
    fn test_measure_fragment_ends_with_separator() {
        // Test case 3: Ends with separator
        let fragment = b"hello\nworld\n";
        let (rows, cols) = measure_fragment::<b'\n'>(fragment);
        assert_eq!(rows, 2);
        assert_eq!(cols, 0);
    }

    #[test]
    fn test_measure_fragment_empty_fragment() {
        // Test case 4: Empty fragment
        let fragment = b"";
        let (rows, cols) = measure_fragment::<b'\n'>(fragment);
        assert_eq!(rows, 0);
        assert_eq!(cols, 0);
    }

    #[test]
    fn test_measure_fragment_three_lines() {
        // Test case 5: Three lines
        let fragment = b"hello\nworld\nrust";
        let (rows, cols) = measure_fragment::<b'\n'>(fragment);
        assert_eq!(rows, 2);
        assert_eq!(cols, 4);
    }

    #[test]
    fn text_doc_lines() {
        let content = b"hello\nworld\nrust";
        let doc = TextDocument::new(content.to_vec(), Language::Rust);
        let mut lines = doc.lines();
        assert_eq!(lines.next(), Some(b"hello".as_slice()));
        assert_eq!(lines.next(), Some(b"world".as_slice()));
        assert_eq!(lines.next(), Some(b"rust".as_slice()));
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn text_doc_lines_tailing_empty() {
        let content = b"hello\nworld\nrust\n";
        let doc = TextDocument::new(content.to_vec(), Language::Rust);
        let mut lines = doc.lines();
        assert_eq!(lines.next(), Some(b"hello".as_slice()));
        assert_eq!(lines.next(), Some(b"world".as_slice()));
        assert_eq!(lines.next(), Some(b"rust".as_slice()));
        assert_eq!(lines.next(), Some(b"".as_slice()));
        assert_eq!(lines.next(), None);
    }
}
