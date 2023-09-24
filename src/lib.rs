#![doc = include_str!("../README.md")]
mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
pub use node::{Node, NodeBuilder};
use std::{
    fmt::Debug,
    pin::Pin,
    rc::{Rc, Weak as WeakRc},
    cell::RefCell,
};

pub type Strong<T> = Pin<Rc<RefCell<T>>>;
/// TODO: pin?
type Weak<T> = WeakRc<RefCell<T>>;

/// A Tree of [`Node`]s.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
/// 
/// Although [`Node`]s use shared ownership though [`Reference Counting`](Rc), a [`Tree`] implies more explicitly that the specific [`Node`] is owned.
pub struct Tree<T> {
    root: Strong<Node<T>>,
}
impl<T> Tree<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    #[inline]
    pub fn root(&self) -> Strong<Node<T>> {
        Pin::clone(&self.root)
    }

    /// Iterate over all the [`Node`]s of the [`Tree`] using **Breadth-First Search**.
    /// Call [`IterBFS::new()`] to iterate from any arbitrary [`Node`].
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self.root())
    }
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Depth-First Search**.
    /// /// Call [`IterDFS::new()`] to iterate from any arbitrary [`Node`].
    pub fn iter_dfs(&self) -> IterDFS<T> {
        IterDFS::new(self.root())
    }
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<T> IntoIterator for &Tree<T> {
    type Item = Strong<Node<T>>;
    type IntoIter = IterBFS<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_bfs()
    }
}

impl<T> From<NodeBuilder<T>> for Tree<T> {
    #[inline]
    fn from(builder: NodeBuilder<T>) -> Self {
        builder.build()
    }
}
impl<T> From<Strong<Node<T>>> for Tree<T> {
    #[inline]
    fn from(root: Strong<Node<T>>) -> Self {
        Tree { root }
    }
}
impl<T: Clone> Clone for Tree<T> {
    /// Clones the entire [`Tree`] by calling [`Node::clone_deep()`] on the **root**.
    fn clone(&self) -> Self {
        self.root().borrow().clone_deep()
    }
}
impl<T: PartialEq> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root().borrow().eq(&*other.root().borrow())
    }
}
impl<T: Eq> Eq for Tree<T> {}
impl<T: Default> Default for Tree<T> {
    fn default() -> Self {
        NodeBuilder::default().build()
    }
}
impl<T: Debug> Debug for Tree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree").field("root", &*self.root().borrow()).finish()
    }
}
