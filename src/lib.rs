mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
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

    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self.root())
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Depth-First Search**.
    pub fn iter_dfs(&self) -> impl Iterator<Item = &Node<T>> {
        IterDFS::new(self.root())
    }
}
impl<T: Clone> Tree<T> {
    /// Calls [`Node::clone_deep()`] on the root of the [`Tree`].
    pub fn clone_deep(&self) -> Tree<T> {
        self.root().clone_deep()
    }
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<'a, T> IntoIterator for &'a Tree<T> {
    type Item = &'a Node<T>;
    /* TODO: Change to `impl Iterator<Item = Self::Item>` (and also in Tree::iter_bfs()) when `impl Trait associated types` becomes stable.
    See issue #63063: https://github.com/rust-lang/rust/issues/63063 */
    type IntoIter = IterBFS<'a, T>;

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
