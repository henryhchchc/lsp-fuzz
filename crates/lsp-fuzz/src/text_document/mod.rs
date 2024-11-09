use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    hash::Hash,
    ops::Range,
};

use derive_more::derive::{Display, FromStr};
use grammars::GrammarContext;
use libafl::{
    inputs::HasTargetBytes,
    mutators::MutatorsTuple,
    state::{HasMaxSize, HasRand},
    SerdeAny,
};
use libafl_bolts::{ownedref::OwnedSlice, tuples::NamedTuple, HasLen};
use serde::{Deserialize, Serialize};
use tree_sitter::InputEdit;
use tuple_list::tuple_list;

pub mod grammars;
pub mod mutations;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Display, FromStr)]
#[non_exhaustive]
pub enum Language {
    C,
    CPlusPlus,
    Rust,
}

impl Language {
    pub fn file_extensions<'a>(&self) -> HashSet<&'a str> {
        match self {
            Self::C => HashSet::from(["c", "cc", "h"]),
            Self::CPlusPlus => HashSet::from(["cpp", "cxx", "hpp"]),
            Self::Rust => HashSet::from(["rs"]),
        }
    }

    pub fn tree_sitter_parser(&self) -> tree_sitter::Parser {
        let language = match self {
            Self::C => tree_sitter_c::LANGUAGE,
            Self::CPlusPlus => tree_sitter_cpp::LANGUAGE,
            Self::Rust => tree_sitter_rust::LANGUAGE,
        };
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language.into())
            .expect("Fail to initialize parser");
        parser
    }

    fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Self::C => tree_sitter::Language::new(tree_sitter_c::LANGUAGE),
            Self::CPlusPlus => tree_sitter::Language::new(tree_sitter_cpp::LANGUAGE),
            Self::Rust => tree_sitter::Language::new(tree_sitter_rust::LANGUAGE),
        }
    }

    pub const fn lsp_language_id<'a>(&self) -> &'a str {
        match self {
            Self::C => "c",
            Self::CPlusPlus => "cpp",
            Self::Rust => "rust",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, SerdeAny, derive_more::Deref)]
pub struct GrammarContextLookup {
    #[deref]
    inner: HashMap<Language, grammars::GrammarContext>,
}

impl FromIterator<(Language, grammars::GrammarContext)> for GrammarContextLookup {
    fn from_iter<T: IntoIterator<Item = (Language, grammars::GrammarContext)>>(iter: T) -> Self {
        Self {
            inner: HashMap::from_iter(iter),
        }
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

pub trait GrammarBasedMutation {
    fn language(&self) -> Language;
    fn parse_tree(&mut self, grammar_context: &GrammarContext) -> &tree_sitter::Tree;
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

    fn drain(&mut self, range: tree_sitter::Range, grammar_context: &GrammarContext) {
        self.edit(grammar_context, |content| {
            let byte_range = range.start_byte..range.end_byte;
            let _ = content.drain(byte_range);

            InputEdit {
                start_byte: range.start_byte,
                old_end_byte: range.end_byte,
                new_end_byte: range.start_byte,
                start_position: range.start_point,
                old_end_position: range.end_point,
                new_end_position: range.start_point,
            }
        });
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

    fn update_parse_tree(
        &mut self,
        input_edit: tree_sitter::InputEdit,
        grammar_context: &GrammarContext,
    ) {
        let mut parser = grammar_context.create_parser();
        let old_tree = self
            .parse_tree
            .get_or_insert_with(|| parser.parse(&self.content, None).unwrap());
        old_tree.edit(&input_edit);
        let mut parser = grammar_context.create_parser();
        self.parse_tree = parser.parse(&self.content, self.parse_tree.as_ref());
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

    fn parse_tree(&mut self, grammar_context: &GrammarContext) -> &tree_sitter::Tree {
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

pub const fn text_document_mutations<S>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<TextDocument, S> + NamedTuple + use<'_, S>
where
    S: HasRand + HasMaxSize,
{
    use mutations::*;
    tuple_list![
        ReplaceSubTreeWithDerivation::new(grammar_lookup),
        RemoveErrorNode::new(grammar_lookup),
        GenerateMissingNode::new(grammar_lookup),
        ReplaceNodeWithGenerated::new(grammar_lookup),
        DropRandomNode::new(grammar_lookup),
        DropUncoveredArea::new(grammar_lookup),
    ]
}

pub const fn text_document_reductions<S>(
    grammar_lookup: &GrammarContextLookup,
) -> impl MutatorsTuple<TextDocument, S> + NamedTuple + use<'_, S>
where
    S: HasRand + HasMaxSize,
{
    use mutations::*;
    tuple_list![
        RemoveErrorNode::new(grammar_lookup),
        DropRandomNode::new(grammar_lookup),
        DropUncoveredArea::new(grammar_lookup),
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
}
