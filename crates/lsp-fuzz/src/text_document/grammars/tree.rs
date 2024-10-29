use std::collections::VecDeque;

pub type TraversalOrder = bool;

pub const DEPTH_FIRST_TRAVERSAL: bool = true;
pub const BREADTH_FIRST_TRAVERSAL: bool = false;

/// A trait for iterating over nodes in a tree structure.
///
/// This trait provides methods for creating iterators that traverse the tree
/// in either depth-first or breadth-first order.
pub trait NodeIter<'t> {
    /// Returns an iterator over the nodes in the tree in depth-first order.
    fn iter_depth_first(self) -> TreeIterator<'t, DEPTH_FIRST_TRAVERSAL>;
    /// Returns an iterator over the nodes in the tree in breadth-first order.
    fn iter_breadth_first(self) -> TreeIterator<'t, BREADTH_FIRST_TRAVERSAL>;
}

impl<'t> NodeIter<'t> for tree_sitter::Node<'t> {
    fn iter_depth_first(self) -> TreeIterator<'t, DEPTH_FIRST_TRAVERSAL> {
        TreeIterator::start_from(self)
    }

    fn iter_breadth_first(self) -> TreeIterator<'t, BREADTH_FIRST_TRAVERSAL> {
        TreeIterator::start_from(self)
    }
}

#[derive(Debug)]
pub struct TreeIterator<'t, const ORDER: TraversalOrder> {
    queue: VecDeque<tree_sitter::Node<'t>>,
}

impl<'t, const ORDER: TraversalOrder> TreeIterator<'t, ORDER> {
    pub fn start_from(root: tree_sitter::Node<'t>) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(root);
        Self { queue }
    }
}

impl<'t> Iterator for TreeIterator<'t, DEPTH_FIRST_TRAVERSAL> {
    type Item = tree_sitter::Node<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_back()?;
        self.queue.reserve(node.child_count());
        for i in 0..node.child_count() {
            self.queue
                .push_back(node.child(i).expect("We make sure the index is in range"));
        }
        Some(node)
    }
}

impl<'t> Iterator for TreeIterator<'t, BREADTH_FIRST_TRAVERSAL> {
    type Item = tree_sitter::Node<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_front()?;
        self.queue.reserve(node.child_count());
        for i in 0..node.child_count() {
            self.queue
                .push_back(node.child(i).expect("We make sure the index is in range"));
        }
        Some(node)
    }
}
