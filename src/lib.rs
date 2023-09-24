#![doc = include_str!("../README.md")]
mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
pub use node::{Node, NodeBuilder};
use std::{fmt::Debug, pin::Pin, ptr::NonNull};

type Owned<T> = Pin<Box<T>>;
type Parent<T> = NonNull<T>;

/// A Tree of [`Node`]s.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
pub struct Tree<T> {
    root: Owned<Node<T>>,
}
impl<T> Tree<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn root(&self) -> &Node<T> {
        self.root.as_ref().get_ref()
    }
    pub fn root_mut(&mut self) -> Pin<&mut Node<T>> {
        self.root.as_mut()
    }

    /// Removes the **descendant** of the **root [`Node`]** from the [`Tree`], and returns the *detached [`Node`]* with ownership (aka a [`Tree`]).
    ///
    /// Returns [`None`] if it is not a **descendant** of the **root**, or **root** [`is_same_as`](Node::is_same_as()) **descendant**.
    ///
    /// This function can only be called from the **root [`Node`]**.
    ///
    /// **descendant** must be a *NonNull pointer* (obtained from [`Node::ptr`]) because, if it was a reference,
    /// the borrow checker will consider the entire [`Tree`] to be *immutably borrowed* (including *self*).
    /// The **descendant** pointer passed to this function will remain valid because it is [`Pin`]ned.
    ///
    /// # Example
    /// ```
    /// # use tree_struct::Node;
    /// # let mut tree = Node::builder(0).child(Node::builder(1)).child(Node::builder(2)).build();
    /// let target = tree.root().children()[1].ptr();
    /// let detached = tree.detach_descendant(target).unwrap();
    /// assert!(detached.root().is_same_as(target));
    /// ```
    #[inline]
    pub fn detach_descendant(&mut self, descendant: NonNull<Node<T>>) -> Option<Self> {
        self.root_mut().detach_descendant(descendant)
    }

    /// Mutably borrows a **descendant** of the [`Tree`]'s **root [`Node`]** as `mutable`.
    /// See [Mutable Iterators section](self#iterators-for-mutable-nodes) for why obtaining a `&mut Node` was implemented this way.
    ///
    /// Returns [`None`] if it is not a **descendant** of the **root**, or **root** [`is_same_as`](Node::is_same_as()) **descendant**.
    ///
    /// This function can be used to *mutably borrow* a [`Node`] obtained by a [`Node iterator`](IterBFS).
    ///
    /// **descendant** must be a *NonNull pointer* (obtained from [`Node::ptr`]) because, if it was a reference,
    /// the borrow checker will consider the entire [`Tree`] to be *immutably borrowed* (including *self*).
    /// The **descendant** pointer passed to this function will remain valid because it is [`Pin`]ned.
    ///
    /// # Example
    /// ```
    /// # use tree_struct::Node;
    /// # let mut tree = Node::builder('a').child(Node::builder('b').child(Node::builder('c'))).build();
    /// let target = tree.iter_bfs().find(|n| n.content == 'c').unwrap().ptr();
    /// let borrowed = tree.borrow_descendant(target).unwrap();
    /// assert!(borrowed.is_same_as(target));
    /// ```
    ///
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    #[inline]
    pub fn borrow_descendant(&mut self, descendant: NonNull<Node<T>>) -> Option<Pin<&mut Node<T>>> {
        self.root_mut().borrow_descendant(descendant)
    }

    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self.root())
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Depth-First Search**.
    pub fn iter_dfs(&self) -> IterDFS<T> {
        IterDFS::new(self.root())
    }
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<'a, T> IntoIterator for &'a Tree<T> {
    type Item = &'a Node<T>;
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
    #[inline]
    fn from(root: Owned<Node<T>>) -> Self {
        Tree { root }
    }
}
impl<T: Clone> Clone for Tree<T> {
    /// Clones the entire [`Tree`] by calling [`Node::clone_deep()`] on the **root**.
    fn clone(&self) -> Self {
        self.root().clone_deep()
    }
}
impl<T: PartialEq> PartialEq for Tree<T> {
    fn eq(&self, other: &Self) -> bool {
        self.root().eq(other.root())
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
        f.debug_struct("Tree")
            .field("root", &self.root().debug_tree())
            .finish()
    }
}

/// Obtained by calling [`Node::debug_tree()`].
pub struct DebugTree<'a, T: Debug> {
    root: &'a Node<T>,
}
impl<'a, T: Debug> Debug for DebugTree<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.root.content)
            .field("children", &self.root.children().iter().map(|c| c.debug_tree()).collect::<Box<_>>())
            .finish()
    }
}