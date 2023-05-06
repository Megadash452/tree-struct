mod node;

pub use node::{Node, NodeBuilder};
use std::{cell::UnsafeCell, fmt::Debug, pin::Pin};

type Owned<T> = Pin<Box<UnsafeCell<T>>>;
type Parent<T> = *const UnsafeCell<T>;

/// A Tree of [`Node`]s.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
#[derive(Debug)]
pub struct Tree<T> {
    pub(crate) root: Owned<Node<T>>,
}
impl<T> Tree<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn root(&self) -> &Node<T> {
        unsafe { &*self.root.get() }
    }
    pub fn root_mut(&mut self) -> &mut Node<T> {
        unsafe { &mut *self.root.get() }
    }
}
impl<T: Clone> Tree<T> {
    /// Calls [`Node::clone_deep()`] on the root of the [`Tree`].
    pub fn clone_deep(&self) -> Tree<T> {
        self.root().clone_deep()
    }
}

impl<T> From<NodeBuilder<T>> for Tree<T> {
    #[inline]
    fn from(builder: NodeBuilder<T>) -> Self {
        builder.build()
    }
}
impl<T> From<Owned<Node<T>>> for Tree<T> {
    fn from(root: Owned<Node<T>>) -> Self {
        Tree { root }
    }
}
impl<T: PartialEq> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root().eq(other.root())
    }
}
impl<T: Eq> Eq for Tree<T> {}
