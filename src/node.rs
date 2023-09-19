use super::*;
use ptrplus::AsPtr;
use std::{ptr::NonNull, marker::PhantomPinned};

/// Helper struct to build a [`Tree`] of [`Node`]s.
///
/// ### Examples
/// Can be used as a [Builder Pattern](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html),
/// or something similar, but by assigning the fields.
///
/// ```
/// # use tree_struct::{Node, NodeBuilder};
/// let tree1 = Node::builder("parent")
///     .child(Node::builder("child a"))
///     .child(Node::builder("child b")
///         .child(Node::builder("child c")))
///     .build();
///
/// // Or:
///
/// let tree2 = NodeBuilder {
///     content: "parent",
///     children: vec![
///         NodeBuilder {
///             content: "child a",
///             children: vec![]
///         },
///         NodeBuilder {
///             content: "child b",
///             children: vec![
///                 NodeBuilder {
///                     content: "child c",
///                     children: vec![]
///                 }
///             ]
///         },
///     ],
/// }.build();
///
/// assert_eq!(tree1, tree2);
/// ```
#[derive(Debug, Default)]
pub struct NodeBuilder<T> {
    pub content: T,
    pub children: Vec<Self>,
}
impl<T> NodeBuilder<T> {
    /// New [`NodeBuilder`] using [Builder Pattern](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html).
    pub fn new(content: T) -> Self {
        NodeBuilder {
            content,
            children: vec![],
        }
    }
    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// Create a new [`Tree`] from nodes with **children** and **content**.
    /// The children will be made into [`Pin`]ned [`Node`]s with the proper **parent**.
    pub fn build(self) -> Tree<T> {
        let mut tree = Tree::from(Box::pin(Node {
            content: self.content,
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }));

        unsafe { tree.root_mut().get_unchecked_mut() }.children = Self::build_children(
            tree.root.ptr(),
            self.children
        );

        tree
    }
    fn build_children(parent: Parent<Node<T>>, children: Vec<Self>) -> Vec<Owned<Node<T>>> {
        children
            .into_iter()
            .map(|builder| {
                let mut child = Box::pin(Node {
                    content: builder.content,
                    parent: Some(parent),
                    children: vec![],
                    _pin: PhantomPinned,
                });

                unsafe { child.as_mut().get_unchecked_mut() }.children = Self::build_children(
                    child.ptr(),
                    builder.children
                );

                child
            })
            .collect()
    }
}

pub trait Node: Sized + Debug {
    // Can't have a `Node::content()` because the content would be the struct itself.

    /// Get an *immutable reference* to the `parent` [`Node`] of `self`.
    /// To get a *mutable reference*,
    /// call [`crate::Tree::borrow_descendant()`] from the owner [`Tree`] with `self.parent().ptr()`.
    fn parent<'a>(self: &'a dyn Node) -> Option<&'a dyn Node>;
    /// Holds references to each **child**.
    fn children<'a>(self: &'a dyn Node) -> Box<[&'a dyn Node]>;
    /// Holds mutable references to each **child**.
    fn children_mut<'a>(self: Pin<&'a mut dyn Node>) -> Box<[&'a mut dyn Node]>;

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children()).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    fn next_sibling<'a>(self: &'a dyn Node) -> Option<&'a dyn Node> {
        find_self_next(self, self.parent()?.children().iter())
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children()).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    fn prev_sibling<'a>(self: &'a dyn Node) -> Option<&'a dyn Node> {
        find_self_next(self, self.parent()?.children().iter().rev())
    }

    /// Pushes the **child** to the end of **self**'s *children*.
    fn append_child(self: Pin<&mut dyn Node>, child: /*Tree*/String);
    // {
    //     // Compiler ensures `self != child.root`.
    //     unsafe {
    //         let this = self.get_unchecked_mut();
    //         child.root_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
    //         this.children.push(child.root)
    //     }
    // }
    /// Inserts the **child** to **self**'s *children* at some index.
    fn insert_child(self: Pin<&mut dyn Node>, child: /*Tree*/String, index: usize);
    // {
    //    // Compiler ensures `self != child.root`.
    //     unsafe {
    //         let this = self.get_unchecked_mut() ;
    //         child.root_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
    //         this.children.insert(index, child.root)
    //     }
    // }

    /// See [`crate::Tree::detach_descendant()`].
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    fn detach_descendant(self: Pin<&mut dyn Node>, descendant: NonNull<dyn Node>) -> Option</*Tree*/String> {
        if self.is_same_as(descendant)
        || !has_descendant(self.as_ref().get_ref(), descendant) {
            return None;
        }

        let parent = unsafe { descendant.as_ref().parent.unwrap().as_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| descendant.as_ptr() == child.ptr().as_ptr())
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let mut tree = Tree::from(parent.children.remove(index));
        unsafe { tree.root_mut().get_unchecked_mut() }.parent = None;
        Some(tree)
    }
    /// See [`crate::Tree::borrow_descendant()`].
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    fn borrow_descendant<'a>(self: Pin<&'a mut dyn Node>, descendant: NonNull<dyn Node>) -> Option<Pin<&'a mut dyn Node>> {
        if self.is_same_as(descendant)
        || !has_descendant(self.as_ref().get_ref(), descendant) {
            None
        } else {
            Some(unsafe { Pin::new_unchecked(&mut *descendant.as_ptr()) })
        }
    }

    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Breadth-First Search**.
    fn iter_bfs<'a>(self: &dyn Node) -> IterBFS<'a, T> {
        IterBFS::new(self)
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Depth-First Search**.
    fn iter_dfs<'a>(self: &dyn Node) -> IterDFS<'a, T> {
        IterDFS::new(self)
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    fn is_same_as(self: &dyn Node, other: impl AsPtr<Raw = dyn Node>) -> bool {
        std::ptr::eq(self, other.as_ptr())
    }
    #[inline]
    /// Get a *[`NonNull`] pointer* for **self**, which should only be treated as a `*const Self`.
    /// Useful for [`Tree::detach_descendant`] and [`Tree::borrow_descendant`].
    /// To get a *raw pointer* (*const Self) do `.ptr().as_ptr()`.
    fn ptr(self: &dyn Node) -> NonNull<dyn Node> {
        NonNull::from(self)
    }
}

/// Look at every ancestor of **other** until **self** is found. (Not recursive).
fn has_descendant(this: &dyn Node, other: NonNull<dyn Node>) -> bool {
    let mut ancestor = unsafe { other.as_ref() }.parent();

    while let Some(node) = ancestor {
        if this.is_same_as(node) {
            return true;
        }
        ancestor = node.parent();
    }

    false
}
fn find_self_next<'a>(this: &dyn Node, mut iter: impl Iterator<Item = &'a dyn Node>) -> Option<&'a dyn Node> {
    iter.find(|sib| this.is_same_as(*sib));
    iter.next()
}


pub struct BaseNode<T> {
    pub content: T,
    parent: Option<Parent<Self>>,
    children: Vec<Owned<Self>>,
    _pin: PhantomPinned,
}
impl<T> BaseNode<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Holds references to each **child**.
    pub fn children(&self) -> Box<[&Self]> {
        self.children
            .iter()
            .map(|child| child.as_ref().get_ref())
            .collect()
    }
    /// Holds mutable references to each **child**.
    pub fn children_mut(&mut self) -> Box<[Pin<&mut Self>]> {
        self.children
            .iter_mut()
            .map(|child| child.as_mut())
            .collect()
    }

    /// Get an *immutable reference* to the `parent` [`Node`] of `self`.
    /// To get a *mutable reference*,
    /// call [`crate::Tree::borrow_descendant()`] from the owner [`Tree`] with `self.parent().ptr()`.
    pub fn parent(&self) -> Option<&Self> {
        self.parent.map(|p| unsafe { p.as_ref() })
    }

    /// Look at every ancestor of **other** until **self** is found. (Not recursive).
    fn has_descendant(&self, other: NonNull<Self>) -> bool {
        let mut ancestor = unsafe { other.as_ref() }.parent();

        while let Some(node) = ancestor {
            if self.is_same_as(node) {
                return true;
            }
            ancestor = node.parent();
        }

        false
    }
    fn find_self_next<'a>(&self, iter: impl Iterator<Item = &'a Owned<Self>>) -> Option<&'a Self> {
        let mut iter = iter.map(|sib| sib.as_ref().get_ref());
        iter.find(|sib| self.is_same_as(*sib));
        iter.next()
    }

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(&self) -> Option<&Self> {
        self.find_self_next(self.parent()?.children.iter())
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(&self) -> Option<&Self> {
        self.find_self_next(self.parent()?.children.iter().rev())
    }

    /// Pushes the **child** to the end of **self**'s *children*.
    pub fn append_child(self: Pin<&mut Self>, mut child: Tree<T>) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut();
            child.root_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
            this.children.push(child.root)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    pub fn insert_child(self: Pin<&mut Self>, mut child: Tree<T>, index: usize) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut() ;
            child.root_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
            this.children.insert(index, child.root)
        }
    }

    /// See [`crate::Tree::detach_descendant()`].
    /// TODO: Don't know if should make it public.
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    pub(super) fn detach_descendant(self: Pin<&mut Self>, descendant: NonNull<Self>) -> Option<Tree<T>> {
        if self.is_same_as(descendant)
        || !self.has_descendant(descendant) {
            return None;
        }

        let parent = unsafe { descendant.as_ref().parent.unwrap().as_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| descendant.as_ptr() == child.ptr().as_ptr())
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let mut tree = Tree::from(parent.children.remove(index));
        unsafe { tree.root_mut().get_unchecked_mut() }.parent = None;
        Some(tree)
    }

    /// See [`crate::Tree::borrow_descendant()`].
    /// TODO: Don't know if should make it public.
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    pub(super) fn borrow_descendant(self: Pin<&mut Self>, descendant: NonNull<Self>) -> Option<Pin<&mut Self>> {
        if self.is_same_as(descendant)
        || !self.has_descendant(descendant) {
            None
        } else {
            Some(unsafe { Pin::new_unchecked(&mut *descendant.as_ptr()) })
        }
    }

    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self)
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Depth-First Search**.
    pub fn iter_dfs(&self) -> IterDFS<T> {
        IterDFS::new(self)
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: impl AsPtr<Raw = Self>) -> bool {
        std::ptr::eq(self, other.as_ptr())
    }
    #[inline]
    /// Get a *[`NonNull`] pointer* for **self**, which should only be treated as a `*const Self`.
    /// Useful for [`Tree::detach_descendant`] and [`Tree::borrow_descendant`].
    /// To get a *raw pointer* (*const Self) do `.ptr().as_ptr()`.
    pub fn ptr(&self) -> NonNull<Self> {
        NonNull::from(self)
    }
}

impl<T: Clone> Clone for BaseNode<T> {
    /// Copies the [`Node`]'s [`content`](Node::content), but not its [`children`](Node::children).
    /// The resulting cloned [`Node`] will have no **parent** or **children**.
    ///
    /// For a method that clones the [`Node`] *and* its subtree, see [`Node::clone_deep`].
    fn clone(&self) -> Self {
        Self {
            content: self.content.clone(),
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }
    }
}
impl<T: Clone> BaseNode<T> {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        let mut tree = Tree::from(Box::pin(self.clone()));

        unsafe { tree.root_mut().get_unchecked_mut() }.children = self.clone_children_deep(tree.root.ptr());

        tree
    }
    fn clone_children_deep(&self, parent: Parent<Self>) -> Vec<Owned<Self>> {
        self.children
            .iter()
            .map(|node| {
                let mut child = Box::pin(node.as_ref().get_ref().clone());
                let mut_child = unsafe { child.as_mut().get_unchecked_mut() };
                mut_child.parent = Some(parent);
                mut_child.children = node.clone_children_deep(mut_child.ptr());
                child
            })
            .collect()
    }
}

/// Can't implement the [`Default`] trait because a [`Node`] can't exist without being wrapped in a [`Pin`]ned [`UnsafeCell`].
impl<T: Default> BaseNode<T> {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Tree<T> {
        NodeBuilder::default().build()
    }
}

impl<T: PartialEq> PartialEq for BaseNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T: Eq> Eq for BaseNode<T> {}
impl<T: Debug> Debug for BaseNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field(
                "parent",
                &self.parent.map(|p| unsafe { &p.as_ref().content }),
            )
            .field("children", &self.children())
            .finish()
    }
}
