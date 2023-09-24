use super::*;
use std::{ptr::eq as ptr_eq, marker::PhantomPinned};

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
        // Do not pin at first to be able to `Rc::downgrade()` freely.
        let root = Rc::new(RefCell::new(Node {
            content: self.content,
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }));
    
        root.borrow_mut().children = Self::build_children(
            Rc::downgrade(&root),
            self.children
        );
        
        // Can be pinned here because no other unpinned Rcs exist
        Tree::from(unsafe { Pin::new_unchecked(root) })
    }
    fn build_children(parent: Weak<Node<T>>, children: Vec<Self>) -> Vec<Strong<Node<T>>> {
        children.into_iter()
            .map(|builder| {
                // Do not pin at first to be able to `Rc::downgrade()` freely.
                let child = Rc::new(RefCell::new(Node {
                    content: builder.content,
                    parent: Some(Weak::clone(&parent)),
                    children: vec![],
                    _pin: PhantomPinned,
                }));

                child.borrow_mut().children = Self::build_children(
                    Rc::downgrade(&child),
                    builder.children
                );

                // Can be pinned here because no other unpinned Rcs exist
                unsafe { Pin::new_unchecked(child) }
            })
            .collect()
    }
}

pub struct Node<T> {
    pub content: T,
    parent: Option<Weak<Self>>,
    children: Vec<Strong<Self>>,
    _pin: PhantomPinned,
}
impl<T> Node<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Get a [`Strong`] reference to **this** [`Node`]'s **parent**,
    /// which can be [`mutably`](RefCell::borrow_mut()) and [`immutably`](RefCell::borrow()) borrowed.
    pub fn parent(&self) -> Option<Strong<Self>> {
        self.parent
            .as_ref()
            .and_then(|p| unsafe {
                Some(Pin::new_unchecked(Weak::upgrade(p)?))
            })
    }
    /// Get [`Strong`] references to **this** [`Node`]'s **children**,
    /// which can be [`mutably`](RefCell::borrow_mut()) and [`immutably`](RefCell::borrow()) borrowed.
    pub fn children(&self) -> Box<[Strong<Self>]> {
        self.children
            .iter()
            .map(|child| Pin::clone(child))
            .collect()
    }

    // /// A [`Node`] is a **descendant** of another [`Node`] if:
    // /// 1. The two [`Node`]s are not the same ([`std::ptr::eq()`]).
    // /// 2. Looking up the [`Tree`] from `other`, `self` is found to be one of `other`'s ancestors. (Not recursive).
    // fn is_descendant(&self, other: Ref<Self>) -> bool {
    //     if ptr_eq(self, &*other) {
    //         return false;
    //     }
    //
    //     let mut ancestor = other.parent();
    //
    //     while let Some(node) = ancestor {
    //         if ptr_eq(self, node.as_ptr()) {
    //             return true;
    //         }
    //         ancestor = node.borrow().parent();
    //     }
    //
    //     false
    // }
    fn find_self_next<'a>(&'a self, mut iter: impl Iterator<Item = &'a Strong<Self>>) -> Option<Strong<Self>> {
        // Check through all children of parent until `self` is found.
        // Should not use `RefCell::borrow()` because it can cause an unecessary panic!
        iter.find(|sib|
            ptr_eq(self, sib.as_ptr())
        );
        iter.next().map(Pin::clone)
    }

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(&self) -> Option<Strong<Self>> {
        self.find_self_next(self.parent()?.borrow().children.iter())
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(&self) -> Option<Strong<Self>> {
        self.find_self_next(self.parent()?.borrow().children.iter().rev())
    }

    /// Pushes the **child** to the end of **self**'s *children*.
    pub fn append_child(self: Pin<&mut Self>, child: Tree<T>) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut();
            child.root().borrow_mut().parent = Some(todo!());
            this.children.push(child.root)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    pub fn insert_child(self: Pin<&mut Self>, mut child: Tree<T>, index: usize) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut() ;
            child.root().borrow_mut().parent = Some(todo!());
            this.children.insert(index, child.root)
        }
    }

    /// Removes **this** [`Node`] from its **parent** and returns the *detached [`Node`]* with ownership (aka a [`Tree`]).
    /// If `self` has no **parent**, either because it is a *root* or it is not part of a [`Tree`], this will return [`None`].
    pub fn detach(self: Pin<&mut Self>) -> Option<Tree<T>> {
        let parent = self.parent()?;
        let mut parent = parent.borrow_mut();

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| ptr_eq(self.as_ref().get_ref(), child.as_ptr()))
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let root = parent.children.remove(index);
        root.borrow_mut().parent = None;
        Some(Tree::from(root))
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: Strong<Self>) -> bool {
        ptr_eq(self, other.as_ptr())
    }
}

impl<T: Clone> Clone for Node<T> {
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
impl<T: Clone> Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        // Do not pin at first to be able to `Rc::downgrade()` freely.
        let root = Rc::new(RefCell::new(self.clone()));

        root.borrow_mut().children = self.clone_children_deep(Rc::downgrade(&root));

        // Can be pinned here because no other unpinned Rcs exist
        Tree::from(unsafe { Pin::new_unchecked(root) })
    }
    fn clone_children_deep(&self, parent: Weak<Self>) -> Vec<Strong<Self>> {
        self.children
            .iter()
            .map(|child| {
                // Do not pin at first to be able to `Rc::downgrade()` freely.
                let cloned = Rc::new(RefCell::new(child.borrow().clone()));
                cloned.borrow_mut().parent = Some(Weak::clone(&parent));
                cloned.borrow_mut().children = child.borrow().clone_children_deep(Rc::downgrade(&cloned));
                // Can be pinned here because no other unpinned Rcs exist
                unsafe { Pin::new_unchecked(cloned) }
            })
            .collect()
    }
}

/// Can't implement the [`Default`] trait because a [`Node`] can't exist without being wrapped in a [`Pin`]ned pointer.
impl<T: Default> Node<T> {
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Tree<T> {
        NodeBuilder::default().build()
    }
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T: Eq> Eq for Node<T> {}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field(
                "parent",
                &self.parent.as_ref()
                    .and_then(Weak::upgrade)
                    .map(|p| &unsafe { &*RefCell::as_ptr(&p) }.content),
            )
            .field("children", &self.children())
            .finish()
    }
}
