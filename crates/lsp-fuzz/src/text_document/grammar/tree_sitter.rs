use std::{
    fmt::{self, Debug},
    iter::FusedIterator,
    ops::Range,
};

use tree_sitter::{QueryCaptures, QueryCursor, StreamingIterator, TextProvider};

use crate::text_document::{GrammarBasedMutation, TextDocument};

/// A trait for iterating over nodes in a [`tree_sitter::Tree`].
///
/// This allows easy traversal of all nodes in a tree-sitter syntax tree.
pub trait TreeIter {
    /// Returns an iterator over all nodes in the tree in pre-order traversal.
    ///
    /// # Returns
    ///
    /// An iterator yielding each node in the tree.
    fn iter(&self) -> TreeIterator<'_>;
}

impl TreeIter for tree_sitter::Tree {
    fn iter(&self) -> TreeIterator<'_> {
        let node_count = self.root_node().descendant_count();
        let available_descendants = 0..node_count;
        let cursor = self.walk();

        TreeIterator {
            descendant_indices: available_descendants.into_iter(),
            cursor,
        }
    }
}

/// An iterator over all nodes in a tree-sitter syntax tree.
///
/// This iterator provides efficient access to all nodes in a pre-order traversal.
pub struct TreeIterator<'tree> {
    /// Iterator over the indices of descendants to visit
    descendant_indices: <Range<usize> as IntoIterator>::IntoIter,
    /// Cursor used to navigate the tree
    cursor: tree_sitter::TreeCursor<'tree>,
}

impl Debug for TreeIterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeIterator")
            .field("indices", &self.descendant_indices)
            .finish()
    }
}

impl<'tree> Iterator for TreeIterator<'tree> {
    type Item = tree_sitter::Node<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        self.descendant_indices.next().map(|idx| {
            self.cursor.goto_descendant(idx);
            self.cursor.node()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.descendant_indices.size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.descendant_indices.nth(n).map(|idx| {
            self.cursor.goto_descendant(idx);
            self.cursor.node()
        })
    }
}

impl ExactSizeIterator for TreeIterator<'_> {
    fn len(&self) -> usize {
        self.descendant_indices.len()
    }
}

impl FusedIterator for TreeIterator<'_> {}

impl<'a> TextProvider<&'a [u8]> for &'a TextDocument {
    type I = std::iter::Once<&'a [u8]>;

    fn text(&mut self, node: tree_sitter::Node<'_>) -> Self::I {
        std::iter::once(&self.content[node.byte_range()])
    }
}

pub struct CapturesIterator<'doc, 'cursor> {
    captures: QueryCaptures<'cursor, 'doc, &'doc TextDocument, &'doc [u8]>,
    capture_index: u32,
}

impl Debug for CapturesIterator<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturesIterator")
            .field("captures", &(&self.captures as *const _))
            .field("capture_index", &self.capture_index)
            .finish()
    }
}

impl<'doc, 'cursor> CapturesIterator<'doc, 'cursor> {
    pub fn new(
        doc: &'doc TextDocument,
        group_name: &str,
        cursor: &'cursor mut QueryCursor,
    ) -> Option<Self> {
        let parse_tree = doc.parse_tree();
        let query = doc.language().ts_highlight_query();
        let capture_index = query.capture_index_for_name(group_name)?;
        let captures = cursor.captures(query, parse_tree.root_node(), doc);

        Some(Self {
            captures,
            capture_index,
        })
    }
}

impl<'doc> Iterator for CapturesIterator<'doc, '_> {
    type Item = tree_sitter::Node<'doc>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((query_match, index)) = self.captures.next() {
            let capture = query_match.captures[*index];
            if capture.index == self.capture_index {
                return Some(capture.node);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_lower, upper) = self.captures.size_hint();
        (0, upper)
    }
}
