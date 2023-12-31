#![doc = include_str!("./README.md")]
mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
pub use node::{Node, NodeBuilder};
use node::InnerNode;
use std::fmt::Debug;
use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "arc")] {
        use std::sync::{Arc as Rc, Weak as WeakRc};
        use parking_lot::{
            RwLock,
            RwLockReadGuard as ReadLock, RwLockWriteGuard as WriteLock,
            MappedRwLockReadGuard as ContentReadLock, MappedRwLockWriteGuard as ContentWriteLock
        };
    } else if #[cfg(feature = "rc")] {
        use std::{
            rc::{Rc, Weak as WeakRc},
            cell::{RefCell as RwLock, Ref as ReadLock, RefMut as WriteLock},
            // Must have separate type (names) for referencing a Node borrow and one that derives from a Node borrow (see parking_lot import above).
            cell::{Ref as ContentReadLock, RefMut as ContentWriteLock }
        };
    }
}

/// A Tree of [`Node`]s.
/// The root of the Tree has *no parent*.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
/// 
/// Although [`Node`]s use shared ownership though [`Reference Counting`](Rc), a [`Tree`] implies more explicitly that the specific [`Node`] is owned.
#[derive(Default, PartialEq, Eq)]
pub struct Tree<T> {
    root: Node<T>,
}
impl<T> Tree<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    #[inline]
    pub fn root(&self) -> Node<T> {
        self.root.ref_clone()
    }

    /// Iterate over all the [`Node`]s of the [`Tree`] using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self.root())
    }
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Depth-First Search**.
    pub fn iter_dfs(&self) -> IterDFS<T> {
        IterDFS::new(self.root())
    }
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<T> IntoIterator for &Tree<T> {
    type Item = Node<T>;
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
impl<T> Clone for Tree<T>
where T: Clone {
    /// Clones the entire [`Tree`] by calling [`Node::clone_deep()`] on the **root**.
    fn clone(&self) -> Self {
        self.root.clone_deep()
    }
}
impl<T> Debug for Tree<T>
where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("root", &self.root().debug_tree())
            .finish()
    }
}

/// Obtained by calling [`Node::debug_tree()`].
pub struct DebugTree<'a, T>
where T: Debug {
    root: ReadLock<'a, InnerNode<T>>,
}
impl<'a, T> Debug for DebugTree<'a, T>
where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.root.content)
            .field("children", &self.root
                .children
                .iter()
                .map(|c| c.debug_tree())
                .collect::<Box<_>>()
            )
            .finish()
    }
}
