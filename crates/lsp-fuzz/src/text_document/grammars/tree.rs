use std::{
    fmt::{self, Debug},
    ops::Range,
};

/// A trait for iterating over nodes in a [`tree_sitter::Tree`].
pub trait TreeIter {
    /// Returns an iterator over the nodes in the tree.
    fn iter<'t>(&'t self) -> TreeIterator<'t>;
}

impl TreeIter for tree_sitter::Tree {
    fn iter<'t>(&'t self) -> TreeIterator<'t> {
        let available_descendants = (0..self.root_node().descendant_count()).into_iter();
        let cursor = self.walk();
        TreeIterator {
            descendant_indices: available_descendants,
            cursor,
        }
    }
}

pub struct TreeIterator<'t> {
    descendant_indices: <Range<usize> as IntoIterator>::IntoIter,
    cursor: tree_sitter::TreeCursor<'t>,
}

impl Debug for TreeIterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeIterator")
            .field("indices", &self.descendant_indices)
            .finish()
    }
}

impl<'t> Iterator for TreeIterator<'t> {
    type Item = tree_sitter::Node<'t>;

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
