/*!
# Tree Struct

A general-purpose Tree implementation in Rust.

## Dynamic Dispatch
TODO: Explain when to use static dispatch Node (main branch) and when to use dynamic dispatch (this branch).

## Trees and Nodes

A Tree is essentially an `owned` Node with **content**, **children**, and no **parent**.
Most of the time, you will be dealing with `mutably and immutably borrowed` Nodes. Create a Tree with [`NodeBuilder`].

Nodes can be [`mutably borrowed`](Tree::borrow_descendant) from their Tree, then you can change the **content** of the Node, [append children](Node::append_child), or mutably borrow its [children](Node::children_mut). Nodes can also be [detached](Tree::detach_descendant) from the Tree, but that doesn't require a mutable reference to the Node.

## Iterators

You can iterate over all the Nodes of a Tree or a subtree (borrowed Node) using **Breadth-first** or **Depth-first Search** algorithms.

The iterators can be used to [find](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.find) a Node that you want to *detach* or *append* to another Node.

### Iterators for mutable Nodes

Mutable iterators (`Iterator<Item = &mut Node>`) are unsafe due to the fact that they yield mutable references to every Node. This allows borrowing a child with [`Node::children_mut()`], but the same child *will* be yielded in a future iteration. Now there are 2 mutable references to the *same* Node, which is **unsafe**.

A better (and *safe*) alternative to *mutable iterators* is using the `immutable iterators` ([`IterBFS`] and [`IterDFS`]) and [**mutably borrowing** a descendant](Tree::borrow_descendant) from the [`Tree`].
 */
mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
pub use node::{Node, BaseNode, NodeBuilder};
use std::{fmt::Debug, pin::Pin, ptr::NonNull};

type Owned<T> = Pin<Box<T>>;
type Parent<T> = NonNull<T>;

/// A Tree of [`Node`]s.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
pub struct Tree {
    root: Owned<dyn Node>,
}
impl Tree {
    pub fn root(&self) -> &dyn Node {
        self.root.as_ref().get_ref()
    }
    pub fn root_mut(&mut self) -> Pin<&mut dyn Node> {
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
    pub fn detach_descendant(&mut self, descendant: NonNull<dyn Node>) -> Option<Self> {
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
    pub fn borrow_descendant(&mut self, descendant: NonNull<dyn Node>) -> Option<Pin<&mut dyn Node>> {
        self.root_mut().borrow_descendant(descendant)
    }

    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS {
        IterBFS::new(self.root())
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the [`Tree`] using **Depth-First Search**.
    pub fn iter_dfs(&self) -> IterDFS {
        IterDFS::new(self.root())
    }
}
impl Tree {
    /// Calls [`Node::clone_deep()`] on the root of the [`Tree`].
    pub fn clone_deep(&self) -> Tree {
        self.root().clone_deep()
    }
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<'a> IntoIterator for &'a Tree {
    type Item = &'a dyn Node;
    type IntoIter = IterBFS<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_bfs()
    }
}

impl From<NodeBuilder> for Tree {
    #[inline]
    fn from(builder: NodeBuilder) -> Self {
        builder.build()
    }
}
impl From<Owned<dyn Node>> for Tree {
    #[inline]
    fn from(root: Owned<dyn Node>) -> Self {
        Tree { root }
    }
}
// impl PartialEq for Tree {
//     fn eq(&self, other: &Self) -> bool {
//         self.root().eq(other.root())
//     }
// }
// impl Eq for Tree {}
// impl Default for Tree {
//     fn default() -> Self {
//         NodeBuilder::default().build()
//     }
// }
// impl Debug for Tree {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         // f.debug_struct("Tree").field("root", self.root()).finish()
//         todo!()
//     }
// }
