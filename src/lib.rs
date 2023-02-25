mod node;

use std::{fmt::Debug, cell::UnsafeCell};
pub use node::{Node, NodeBuilder};

type RootNode<T> = Box<UnsafeCell<Node<T>>>; // TODO: Pin
type ChildNode<T> = Box<Node<T>>;


/// A Tree of [`Node`]s.
/// 
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
/// 
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
#[derive(Debug)]
pub struct Tree<T> {
    pub(crate) root: RootNode<T>
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
        self.root.as_mut().get_mut()
    }

    // /// [`Appends child`](Node::append_child()) to **root**.
    // pub fn append_child(&mut self, child: impl Into<Tree<T>>) {
    //     Node::append_child(Rc::clone(&self.root), child)
    // }
}
impl <T: Clone> Tree<T> {
    /// See [`Node::clone_deep()`].
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
impl<T> From<RootNode<T>> for Tree<T> {
    /// Get a subtree, and the [`Node`] will have 1 more owner.
    fn from(root: RootNode<T>) -> Self {
        Tree { root }
    }
}
impl<T: PartialEq> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root().eq(other.root())
    }
}
impl<T: Eq> Eq for Tree<T> {
}
