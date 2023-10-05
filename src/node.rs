use ptrplus::AsPtr;

use super::*;
use std::{
    ptr::{NonNull, eq as ptr_eq},
    os::raw::c_void,
    marker::PhantomPinned,
    any::Any,
};

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
    pub fn build(self) -> Owned<BaseNode<T>> {
        let mut root = Box::pin(BaseNode {
            content: self.content,
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        });

        unsafe { root.as_mut().get_unchecked_mut() }.children = Self::build_children(
            NonNull::from(root.as_ref().get_ref()),
            self.children
        );

        root
    }
    fn build_children(parent: Parent<BaseNode<T>>, children: Vec<Self>) -> Vec<Owned<BaseNode<T>>> {
        children
            .into_iter()
            .map(|builder| {
                let mut child = Box::pin(BaseNode {
                    content: builder.content,
                    parent: Some(parent),
                    children: vec![],
                    _pin: PhantomPinned,
                });

                unsafe { child.as_mut().get_unchecked_mut() }.children = Self::build_children(
                    NonNull::from(child.as_ref().get_ref()),
                    builder.children
                );

                child
            })
            .collect()
    }
}

pub trait Node: Any + Debug {
    /// Get an *immutable reference* to the `parent` [`Node`] of `self`.
    /// To get a *mutable reference*,
    /// call [`crate::Tree::borrow_descendant()`] from the owner [`Tree`] with `self.parent().ptr()`.
    fn parent(&self) -> Option<&dyn Node>;
    /// Holds references to each **child**.
    fn children(&self) -> Box<[&dyn Node]>;

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children()).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    fn next_sibling(&self) -> Option<&dyn Node> {
        let children = self.parent()?.children();
        let mut iter = children.into_iter().map(|p| *p);
        iter.find(|sib| ptr_eq(self as *const Self as *const c_void, *sib as *const dyn Node as *const c_void));
        iter.next()
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children()).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    fn prev_sibling(&self) -> Option<&dyn Node> {
        let children = self.parent()?.children();
        let mut iter = children.into_iter().rev().map(|p| *p);
        iter.find(|sib| ptr_eq(self as *const Self as *const c_void, *sib as *const dyn Node as *const c_void));
        iter.next()
    }

    /// See [`crate::Tree::borrow_descendant()`].
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    fn borrow_descendant(self: Pin<&mut Self>, descendant: NonNull<dyn Node>) -> Option<Pin<&mut dyn Node>> {
        if is_descendant(self.as_ref().get_ref() as *const Self as *const c_void, descendant) {
            Some(unsafe { Pin::new_unchecked(&mut *descendant.as_ptr()) })
        } else {
            None
        }
    }

    // /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    // /// The resulting cloned [`Node`] will have no **parent**.
    // ///
    // /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    // fn clone_deep(&self) -> Tree;

    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Breadth-First Search**.
    fn iter_bfs<'a>(&'a self) -> IterBFS<'a>;
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Depth-First Search**.
    fn iter_dfs<'a>(&'a self) -> IterDFS<'a>;

    /// [`Debug`] the entire subtree (`self` and its **children**).
    fn debug_tree(&self) -> DebugTree;
    fn debug_content(&self) -> &dyn Debug;

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    fn is_same_as(&self, other: *const dyn Node) -> bool {
        std::ptr::eq(self as *const Self as *const c_void, other.as_ptr() as *const c_void)
    }
    /// Get a *[`NonNull`] pointer* for **self**, which should only be treated as a `*const Self`.
    /// Useful for [`Tree::detach_descendant`] and [`Tree::borrow_descendant`].
    /// To get a *raw pointer* (*const Self) do `.ptr().as_ptr()`.
    fn ptr(&self) -> NonNull<dyn Node>;

    fn as_any(&self) -> &dyn Any;
}

/// A [`Node`] is a **descendant** of another [`Node`] if:
/// 1. The two [`Node`]s are not the same ([`std::ptr::eq()`]).
/// 2. Looking up the [`Tree`] from `other`,  `self` is found to be one of `other`'s ancestors. (Not recursive).
fn is_descendant(this: *const c_void, other: NonNull<dyn Node>) -> bool {
    if !ptr_eq(this, other.as_ptr() as *const dyn Node as *const c_void) {
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


pub struct BaseNode<T> {
    pub content: T,
    parent: Option<Parent<Self>>,
    children: Vec<Owned<Self>>,
    _pin: PhantomPinned,
}
impl<T> BaseNode<T>
where T: Debug + 'static {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Pushes the **child** to the end of **self**'s *children*.
    /// Also see [`Self::insert_child()`].
    pub fn append_child(self: Pin<&mut Self>, mut child: Owned<Self>) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut();
            child.as_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
            this.children.push(child)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    /// Also see [`Self::append_child()`].
    pub fn insert_child(self: Pin<&mut Self>, mut child: Owned<Self>, index: usize) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut() ;
            child.as_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
            this.children.insert(index, child)
        }
    }

    pub fn detach_descendant(self: Pin<&mut Self>, descendant: NonNull<dyn Node>) -> Option<Owned<Self>> {
        if !is_descendant(self.as_ref().get_ref() as *const Self as *const c_void, descendant) {
            return None;
        }

        // if it is a descendant, it is guaranteed to be Self
        let parent = unsafe { descendant.as_ref().as_any().downcast_ref::<Self>().unwrap().parent.unwrap().as_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| ptr_eq(descendant.as_ptr(), child.as_ref().get_ref()))
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        // todo!()
        let mut root = parent.children.remove(index);
        unsafe { root.as_mut().get_unchecked_mut() }.parent = None;
        Some(root)
    }
}
impl<T> BaseNode<T>
where T: Clone {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Owned<Self> {
        let mut root = Box::pin(self.clone());

        fn clone_children_deep<T: Clone>(this: &BaseNode<T>, parent: Parent<BaseNode<T>>) -> Vec<Owned<BaseNode<T>>> {
            this.children
                .iter()
                .map(|node| {
                    let mut child = Box::pin(node.as_ref().get_ref().clone());
                    let mut_child = unsafe { child.as_mut().get_unchecked_mut() };
                    mut_child.parent = Some(parent);
                    mut_child.children = clone_children_deep(node, unsafe { NonNull::new_unchecked(mut_child) });
                    child
                })
                .collect()
        }

        unsafe { root.as_mut().get_unchecked_mut() }.children = clone_children_deep(self, NonNull::from(root.as_ref().get_ref()));

        root
    }
}

impl<T> Node for BaseNode<T>
where T: Debug + 'static {
    fn parent(&self) -> Option<&dyn Node> {
        self.parent.map(|p| unsafe { p.as_ref() } as &dyn Node)
    }
    fn children(&self) -> Box<[&dyn Node]> {
        self.children
            .iter()
            .map(|child| child.as_ref().get_ref() as &dyn Node)
            .collect()
    }

    fn iter_bfs<'a>(&'a self) -> IterBFS<'a> {
        IterBFS::new(self)
    }
    fn iter_dfs<'a>(&'a self) -> IterDFS<'a> {
        IterDFS::new(self)
    }
    
    fn debug_tree(&self) -> DebugTree {
        DebugTree { root: self }
    }
    fn debug_content(&self) -> &dyn Debug {
        &self.content as &dyn Debug
    }
    
    fn ptr(&self) -> NonNull<dyn Node> {
        NonNull::from(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T> Default for BaseNode<T>
where T: Default {
    fn default() -> Self {
        Self {
            content: T::default(),
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }
    }
}
impl<T> Clone for BaseNode<T>
where T: Clone {
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
impl<T> PartialEq for BaseNode<T>
where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T> PartialEq<dyn Node> for BaseNode<T>
where T: PartialEq + 'static {
    fn eq<'a>(&'a self, other: &'a dyn Node) -> bool {
        match other.as_any().downcast_ref::<Self>() {
            Some(other) => self == other,
            None => false
        }
    }
}
impl<T> Eq for BaseNode<T>
where T: Eq {}
impl<T> Debug for BaseNode<T>
where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field("children", &self.children.iter().map(|c| &c.content).collect::<Box<_>>())
            .finish()
    }
}
