#![doc = include_str!("../README.md")]
mod iter;
mod node;

pub use iter::{IterBFS, IterDFS};
pub use node::{Node, BaseNode, NodeBuilder};
pub use tree_struct_macros::full_node_impl;
use std::{
    pin::Pin,
    ptr::{NonNull, eq as ptr_eq},
    fmt::Debug,
    os::raw::c_void
};

type Owned<T> = Pin<Box<T>>;
type Parent<T> = NonNull<T>;

/// A Tree of [`Node`]s.
///
/// ### Ownership
/// When a [`Node`] method *returns* this type, it means it is **passing ownership** of the [`Node`]s.
///
/// When a [`Node`] method *asks* for this type as argument, it means it is **taking ownership** of the [`Node`]s.
pub struct Tree<T> {
    root: Owned<BaseNode<T>>,
}
impl<T> Tree<T> {
    pub fn root(&self) -> &BaseNode<T> {
        self.root.as_ref().get_ref()
    }
    pub fn root_mut(&mut self) -> Pin<&mut BaseNode<T>> {
        self.root.as_mut()
    }
}
impl<T> Tree<T>
where T: Debug + 'static {
    // /// Removes the **descendant** of the **root [`Node`]** from the [`Tree`], and returns the *detached [`Node`]* with ownership (aka a [`Tree`]).
    // ///
    // /// Returns [`None`] if it is not a **descendant** of the **root**, or **root** [`is_same_as`](Node::is_same_as()) **descendant**.
    // ///
    // /// This function can only be called from the **root [`Node`]**.
    // ///
    // /// **descendant** must be a *NonNull pointer* (obtained from [`Node::ptr`]) because, if it was a reference,
    // /// the borrow checker will consider the entire [`Tree`] to be *immutably borrowed* (including *self*).
    // /// The **descendant** pointer passed to this function will remain valid because it is [`Pin`]ned.
    // ///
    // /// # Example
    // /// ```
    // /// # use tree_struct::Node;
    // /// # let mut tree = Node::builder(0).child(Node::builder(1)).child(Node::builder(2)).build();
    // /// let target = tree.root().children()[1].ptr();
    // /// let detached = tree.detach_descendant(target).unwrap();
    // /// assert!(detached.root().is_same_as(target));
    // /// ```
    pub fn detach_descendant(&mut self, descendant: NonNull<dyn Node>) -> Option<Self> {
        if !is_descendant(self.root() as *const _ as *const c_void, descendant) {
            return None;
        }

        // if it is a descendant, it is guaranteed to be Self
        let parent = unsafe { descendant.as_ref().downcast_ref::<BaseNode<T>>().unwrap().parent.unwrap().as_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child|
                ptr_eq(descendant.as_ptr() as *const c_void, child.as_ref().get_ref() as *const _ as *const c_void)
            )
            .expect("Node is not found in its parent");
        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        // todo!()
        let mut root = parent.children.remove(index);
        unsafe { root.as_mut().get_unchecked_mut() }.parent = None;
        Some(Tree { root })
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
    /// # use tree_struct::{BaseNode, Node};
    /// # let mut tree = BaseNode::builder('a').child(BaseNode::builder('b').child(BaseNode::builder('c'))).build();
    /// let target = tree.iter_bfs().find(|n| n.downcast_ref::<BaseNode<char>>().unwrap().content == 'c').unwrap().ptr();
    /// let borrowed = tree.borrow_descendant(target).unwrap();
    /// assert!(borrowed.is_same_as(target.as_ptr()));
    /// ```
    ///
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    pub fn borrow_descendant(&mut self, mut descendant: NonNull<dyn Node>) -> Option<Pin<&mut BaseNode<T>>> {
        if is_descendant(self.root() as *const _ as *const c_void, descendant) {
            Some(unsafe { Pin::new_unchecked(descendant.as_mut().downcast_mut().unwrap()) })
        } else {
            None
        }
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

/// A [`Node`] is a **descendant** of another [`Node`] if:
/// 1. The two [`Node`]s are not the same ([`std::ptr::eq()`]).
/// 2. Looking up the [`Tree`] from `other`,  `self` is found to be one of `other`'s ancestors. (Not recursive).
fn is_descendant(this: *const c_void, other: NonNull<dyn Node>) -> bool {
    if ptr_eq(this, other.as_ptr() as *const dyn Node as *const c_void) {
        return false;
    }

    let mut ancestor = unsafe { other.as_ref() }.parent();

    while let Some(node) = ancestor {
        if ptr_eq(this, node as *const dyn Node as *const c_void) {
            return true;
        }
        ancestor = node.parent();
    }

    false
}

/* Only Tree should implement IntoIter because , semantically, it makes sense to iterate through a Tree, but doesn't make sense to iterate through a Node.
Node still has iter_bfs() and iter_dfs() in case the user wants to use it that way. */
impl<'a, T> IntoIterator for &'a Tree<T>
where T: Debug + 'static {
    type Item = &'a dyn Node;
    type IntoIter = IterBFS<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_bfs()
    }
}

// impl From<NodeBuilder> for Tree {
//     #[inline]
//     fn from(builder: NodeBuilder) -> Self {
//         builder.build()
//     }
// }
impl<T> Clone for Tree<T>
where T: Clone {
    /// Clones the entire [`Tree`] by calling [`Node::clone_deep()`] on the **root**.
    fn clone(&self) -> Self {
        self.root().clone_deep()
    }
}
impl<T> PartialEq for Tree<T>
where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.root().eq(other.root())
    }
}
impl<T> Eq for Tree<T>
where T: Eq {}
impl<T> Default for Tree<T>
where T: Default {
    fn default() -> Self {
        NodeBuilder::default().build()
    }
}
impl<T> Debug for Tree<T>
where T: Debug + 'static {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("root", &(self.root() as &dyn Node).debug_tree())
            .finish()
    }
}

/// Obtained by calling [`Node::debug_tree()`].
pub struct DebugTree<'a> {
    pub root: &'a dyn Node,
}
impl<'a> Debug for DebugTree<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.root.debug_content())
            .field("children", &self.root.children().iter().map(|c| c.debug_tree()).collect::<Box<_>>())
            .finish()
    }
}
