#![feature(arbitrary_self_types)]
mod node;
mod util;

use std::{
    rc::Rc,
    cell::{RefCell, Ref, RefMut},
    fmt::Debug
};
pub use node::{Node, NodeBuilder};
use node::*;


/// A Tree of [`Node`]s.
/// 
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
/// 
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
#[derive(Debug, PartialEq, Eq)]
pub struct Tree<T> {
    pub root: StrongNode<T>
}
impl<T> Tree<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn root(&self) -> Ref<Node<T>> {
        self.root.borrow()
    }
    pub fn root_mut(&mut self) -> RefMut<Node<T>> {
        self.root.borrow_mut()
    }

    /// [`Appends child`](Node::append_child()) to **root**.
    pub fn append_child(&mut self, child: impl Into<Tree<T>>) {
        Node::append_child(Rc::clone(&self.root), child)
    }
}
impl<T> From<NodeBuilder<T>> for Tree<T> {
    #[inline]
    fn from(builder: NodeBuilder<T>) -> Self {
        builder.build()
    }
}
impl<T> From<StrongNode<T>> for Tree<T> {
    /// Get a subtree, and the [`Node`] will have 1 more owner.
    fn from(root: StrongNode<T>) -> Self {
        Tree { root }
    }
}
impl<T> From<&StrongNode<T>> for Tree<T> {
    /// Create a subtree, and the [`Node`] will have 1 more owner.
    fn from(root: &StrongNode<T>) -> Self {
        Tree { root: Rc::clone(root) }
    }
}
impl <T: Clone> Tree<T> {
    /// See [`Node::clone_deep()`].
    pub fn clone_deep(&self) -> Tree<T> {
        self.root.borrow().clone_deep()
    }
}
