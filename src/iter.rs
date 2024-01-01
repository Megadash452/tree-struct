use super::*;
use std::collections::VecDeque;

/// An [`Iterator`] over all the [`Node`]s of a [`Tree`] (or subtree) using a **Breadth-First Search** algorithm.
///
/// Obtained by calling [`Tree::iter_bfs()`] or [`Node::iter_bfs()`].
///
/// There is also [`IterDFS`], which uses *Depth-First search*, but **BFS** is usually *faster* in most scenarios.
pub struct IterBFS<'a> {
    /* Apparently a Vec would perform better than a LinkedList in this case.
    https://stackoverflow.com/questions/40848918/are-there-queue-and-stack-collections-in-rust */
    queue: VecDeque<&'a dyn Node>
}
impl<'a> IterBFS<'a> {
    pub fn new(node: &'a dyn Node) -> Self {
        let mut queue = VecDeque::new();
        // Step 1: Enqueue the root.
        queue.push_back(node);
        Self { queue }
    }
}
impl<'a> Iterator for IterBFS<'a> {
    type Item = &'a dyn Node;

    fn next(&mut self) -> Option<Self::Item> {
        // Step 2: Get next from queue.
        let popped = self.queue.pop_front();
        if let Some(popped) = popped {
            // Step 3: Enqueue its children.
            self.queue.extend(popped.children().iter());
        }
        popped
    }
}

/// An [`Iterator`] over all the [`Node`]s of a [`Tree`] (or subtree) using a **non-recursive**, **Depth-First Search** algorithm.
///
/// Obtained by calling [`Tree::iter_dfs()`] or [`Node::iter_dfs()`].
///
/// You should most likely use [`IterBFS`], which uses *Breadth-First search*, becase it is usually *faster* in most scenarios.
pub struct IterDFS<'a> {
    /* Apparently a Vec would perform better than a LinkedList in this case.
    https://stackoverflow.com/questions/40848918/are-there-queue-and-stack-collections-in-rust */
    stack: Vec<&'a dyn Node>
}
impl<'a> IterDFS<'a> {
    pub fn new(node: &'a dyn Node) -> Self {
        // Step 1: Push the root.
        Self { stack: vec![node] }
    }
}
impl<'a> Iterator for IterDFS<'a> {
    type Item = &'a dyn Node;

    fn next(&mut self) -> Option<Self::Item> {
        // Step 2: Get next from stack.
        let popped = self.stack.pop();
        if let Some(popped) = popped {
            // Step 3: Push its children.
            // Reverse because the first child should be popped next from the stack, so it must go last in the stack.
            self.stack.extend(popped.children().iter().rev());
        }
        popped
    }
}
