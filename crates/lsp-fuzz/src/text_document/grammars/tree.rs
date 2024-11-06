use std::collections::VecDeque;

pub type TraversalType = bool;

pub const DEPTH_FIRST_TRAVERSAL: bool = true;
pub const BREADTH_FIRST_TRAVERSAL: bool = false;

/// A trait for iterating over nodes in a tree structure.
///
/// This trait provides methods for creating iterators that traverse the tree
/// in either depth-first or breadth-first order.
pub trait NodeIter {
    /// Returns an iterator over the nodes in the tree in depth-first order.
    fn iter_depth_first<'t>(self) -> TreeIterator<'t, DEPTH_FIRST_TRAVERSAL>
    where
        Self: 't;
    /// Returns an iterator over the nodes in the tree in breadth-first order.
    fn iter_breadth_first<'t>(self) -> TreeIterator<'t, BREADTH_FIRST_TRAVERSAL>
    where
        Self: 't;
}

impl NodeIter for tree_sitter::Node<'_> {
    fn iter_depth_first<'t>(self) -> TreeIterator<'t, DEPTH_FIRST_TRAVERSAL>
    where
        Self: 't,
    {
        TreeIterator::depth_first_from(self)
    }

    fn iter_breadth_first<'t>(self) -> TreeIterator<'t, BREADTH_FIRST_TRAVERSAL>
    where
        Self: 't,
    {
        TreeIterator::breadth_first_from(self)
    }
}

#[derive(Debug)]
pub struct TreeIterator<'t, const TRV: TraversalType> {
    visited_counter: usize,
    total_nodes: usize,
    queue: VecDeque<tree_sitter::Node<'t>>,
}

impl<'t, const TRV: TraversalType> TreeIterator<'t, TRV> {
    pub fn new(root: tree_sitter::Node<'t>) -> Self {
        let mut queue = VecDeque::new();
        let total_nodes = root.descendant_count();
        queue.push_back(root);
        Self {
            queue,
            visited_counter: 0,
            total_nodes,
        }
    }
}

impl<'t> TreeIterator<'t, BREADTH_FIRST_TRAVERSAL> {
    pub fn breadth_first_from(root: tree_sitter::Node<'t>) -> Self {
        Self::new(root)
    }
}

impl<'t> TreeIterator<'t, DEPTH_FIRST_TRAVERSAL> {
    pub fn depth_first_from(root: tree_sitter::Node<'t>) -> Self {
        Self::new(root)
    }
}

impl<'t, const TRV: TraversalType> Iterator for TreeIterator<'t, TRV> {
    type Item = tree_sitter::Node<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = match TRV {
            DEPTH_FIRST_TRAVERSAL => self.queue.pop_back(),
            BREADTH_FIRST_TRAVERSAL => self.queue.pop_front(),
        }?;
        self.queue.reserve(node.child_count());
        for i in 0..node.child_count() {
            self.queue
                .push_back(node.child(i).expect("We make sure the index is in range"));
        }
        self.visited_counter += 1;
        Some(node)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_nodes = self.total_nodes - self.visited_counter;
        (remaining_nodes, Some(remaining_nodes))
    }
}

impl<const TRV: TraversalType> ExactSizeIterator for TreeIterator<'_, TRV> {
    fn len(&self) -> usize {
        self.total_nodes - self.visited_counter
    }
}
