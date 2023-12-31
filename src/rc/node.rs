use super::*;
use std::{
    pin::Pin,
    marker::PhantomPinned,
};

type Weak<T> = WeakRc<RwLock<T>>;

// Helper functions that allow writing the same code between RefCell and RwLock.
// Resulting types for *read* are `impl Deref<Target = T>` and for *write* the DerefMut variant.
#[inline]
fn borrow<T>(this: &RwLock<T>) -> ReadLock<T> {
    cfg_if! {
        if #[cfg(feature = "arc")] {
            this.read()
        } else if #[cfg(feature = "rc")] {
            this.borrow()
        }
    }
}
#[inline]
fn borrow_mut<T>(this: &RwLock<T>) -> WriteLock<T> {
    cfg_if! {
        if #[cfg(feature = "arc")] {
            this.write()
        } else if #[cfg(feature = "rc")] {
            this.borrow_mut()
        }
    }
}

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
        let root = Rc::new(RwLock::new(InnerNode::new(self.content)));
    
        borrow_mut(&root).children = Self::build_children(
            Rc::downgrade(&root),
            self.children
        );
        
        // Can be pinned here because no other unpinned Rcs exist
        Tree { root: Node(unsafe { Pin::new_unchecked(root) }) }
    }
    fn build_children(parent: Weak<InnerNode<T>>, children: Vec<Self>) -> Vec<Node<T>> {
        children.into_iter()
            .map(|builder| {
                // Do not pin at first to be able to `Rc::downgrade()` freely.
                let child = Rc::new(RwLock::new(InnerNode::new(builder.content)));
                let mut child_mut = borrow_mut(&child);

                // When using RwLock: Don't need to unlock for other threads because the Node hasn't been released and is not used while this lock is alive.
                child_mut.parent = Some(Weak::clone(&parent));
                child_mut.children = Self::build_children(
                    Rc::downgrade(&child),
                    builder.children
                );
                drop(child_mut);

                // Can be pinned here because no other unpinned Rcs exist
                Node(unsafe { Pin::new_unchecked(child) })
            })
            .collect()
    }
}

#[derive(Default)]
pub(super) struct InnerNode<T> {
    pub content: T,
    parent: Option<Weak<Self>>,
    pub(super) children: Vec<Node<T>>,
    _pin: PhantomPinned,
}
impl<T> InnerNode<T> {
    fn new(content: T) -> Self {
        Self {
            content,
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }
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
}
impl<T> Debug for InnerNode<T>
where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field("children", &self
                .children
                .iter()
                .map(|c| ReadLock::map(c.borrow(), |c| &c.content))
                .collect::<Box<_>>()
            )
            .finish()
    }
}
impl<T> PartialEq for InnerNode<T>
where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T> Eq for InnerNode<T>
where T: Eq {}


/// The outward-facing Node is the node struct wrapped in a [`cell`](std::cell::RefCell) (or [`RwLock`]) and `reference counted pointer` ([`Rc`](std::rc::Rc) or [`Arc`](std::sync::Arc)).
/// 
/// A [`Node`] has 1 [`parent`](Self::parent()) and multiple [`children`](Self::children()).
/// It also stores [`content`](Self::content()) of type **`T`**.
pub struct Node<T>(Pin<Rc<RwLock<InnerNode<T>>>>);
impl<T> Node<T> {
    #[inline]
    fn borrow(&self) -> ReadLock<InnerNode<T>> {
        borrow(&self.0)
    }
    fn borrow_mut(&self) -> Pin<WriteLock<InnerNode<T>>> {
        unsafe { Pin::new_unchecked(borrow_mut(&self.0)) }
    }

    /// Must be immediately made into [`Self`] when upgraded.
    unsafe fn downgrade(&self) -> Weak<InnerNode<T>> {
        // Casting Pin<P> to P is ok as long as nothing is moved later
        unsafe { Rc::downgrade(&*(&self.0 as *const _ as *const Rc<_>)) }
    }

    /// Check through all children of parent until `self` is found.
    fn find_self_next<'a>(&'a self, mut iter: impl Iterator<Item = &'a Self>) -> Option<Self> {
        iter.find(|sib| self.is_same_as(sib));
        iter.next().map(Node::ref_clone)
    }

    // vvv Public Functions vvv

    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn parent(&self) -> Option<Self> {
        self.borrow()
            .parent
            .as_ref()
            .and_then(|p| unsafe {
                Some(Self(Pin::new_unchecked(Weak::upgrade(p)?)))
            })
    }
    /// Allocates a *slice* of all of [`Node`]'s children, increasing all of their *reference counter*.
    pub fn children(&self) -> Box<[Self]> {
        self.borrow()
            .children
            .iter()
            .map(|c| c.ref_clone())
            .collect()
    }
    pub fn content(&self) -> ContentReadLock<T> {
        ReadLock::map(self.borrow(), |n| &n.content)
    }
    pub fn content_mut(&self) -> ContentWriteLock<T> {
        WriteLock::map(unsafe { Pin::into_inner_unchecked(self.borrow_mut()) }, |n| &mut n.content)
    }

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(&self) -> Option<Self> {
        self.find_self_next(self.parent()?.children().iter())
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(&self) -> Option<Self> {
        self.find_self_next(self.parent()?.children().iter().rev())
    }

    /// Pushes the **child** to the end of **self**'s *children*.
    /// **child** is required to be a **root** (i.e. have no *parent*), and [`Tree`] guarantees that.
    /// 
    /// Also see [`Self::insert_child()`].
    pub fn append_child(&self, child: Tree<T>) {
        unsafe {
            child.root.borrow_mut().as_mut().get_unchecked_mut().parent = Some(self.downgrade());
            self.borrow_mut().as_mut().get_unchecked_mut().children.push(child.root)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    /// **child** is required to be a **root** (i.e. have no *parent*), and [`Tree`] guarantees that.
    /// 
    /// Also see [`Self::append_child()`].
    pub fn insert_child(&self, child: Tree<T>, index: usize) {
        unsafe {
            child.root.borrow_mut().as_mut().get_unchecked_mut().parent = Some(self.downgrade());
            self.borrow_mut().as_mut().get_unchecked_mut().children.insert(index, child.root)
        }
    }

    /// Removes **this** [`Node`] from its **parent** and returns the *detached [`Node`]* with ownership (aka a [`Tree`]).
    /// If `self` has no **parent**, either because it is a *root* or it is not part of a [`Tree`], this will return [`None`].
    pub fn detach(&self) -> Option<Tree<T>> {
        let parent = self.parent()?;

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.borrow().children.iter()
            .position(|child| self.is_same_as(child))
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let root = unsafe { parent.borrow_mut().as_mut().get_unchecked_mut() }.children.remove(index);
        unsafe { root.borrow_mut().as_mut().get_unchecked_mut().parent = None };
        Some(Tree { root })
    }

    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Breadth-First Search**.
    pub fn iter_bfs(&self) -> IterBFS<T> {
        IterBFS::new(self.ref_clone())
    }
    #[inline]
    /// Iterate over all the [`Node`]s of the *subtree* (including `self`) using **Depth-First Search**.
    pub fn iter_dfs(&self) -> IterDFS<T> {
        IterDFS::new(self.ref_clone())
    }

    /// Clones the [`Rc`] and increments the internal reference counter of this [`Node`].
    pub fn ref_clone(&self) -> Self {
        Self(Pin::clone(&self.0))
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: &Self) -> bool {
        unsafe {
            Rc::<RwLock<InnerNode<T>>>::ptr_eq(
                // Casting Pin<P> to P is ok as long as nothing is moved later
                &*(&self.0 as *const _ as *const Rc<_>),
                &*(&other.0 as *const _ as *const Rc<_>)
            )
        }
    }
}
impl<T> Node<T>
where T: Clone {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        // Do not pin at first to be able to `Rc::downgrade()` freely.
        let root = Rc::new(RwLock::new(InnerNode::new(self.borrow().content.clone())));

        // TODO: Use Arc::get_mut_unchecked() (when it becomes stable) folowed by RwLock::get_mut.
        borrow_mut(&root).children = self.clone_children_deep(Rc::downgrade(&root));

        // Can be pinned here because no other unpinned Rcs exist
        Tree { root: Self(unsafe { Pin::new_unchecked(root) }) }
    }
    fn clone_children_deep(&self, parent: Weak<InnerNode<T>>) -> Vec<Self> {
        self.children()
            .iter()
            .map(|child| {
                // Do not pin at first to be able to `Rc::downgrade()` freely.
                let cloned = Rc::new(RwLock::new(InnerNode::new(child.borrow().content.clone())));
                // TODO: Use Arc::get_mut_unchecked() (when it becomes stable) folowed by RwLock::get_mut.
                let mut cloned_mut  = borrow_mut(&cloned);

                // When using RwLock: Don't need to unlock for other threads because the Node hasn't been released and is not used while this lock is alive.
                cloned_mut.parent = Some(Weak::clone(&parent));
                cloned_mut.children = child.clone_children_deep(Rc::downgrade(&cloned));
                drop(cloned_mut);

                // Can be pinned here because no other unpinned Rcs exist
                Self(unsafe { Pin::new_unchecked(cloned) })
            })
            .collect()
    }
}
impl<T> Node<T>
where T: Debug {
    /// [`Debug`] the entire subtree (`self` and its **children**).
    #[inline]
    pub fn debug_tree(&self) -> DebugTree<T> {
        DebugTree { root: self.borrow() }
    }
}

impl<T> Default for Node<T>
where T: Default {
    fn default() -> Self {
        Self(Rc::pin(RwLock::new(InnerNode::default())))
    }
}
impl<T> Clone for Node<T>
where T: Clone {
    /// Copies the [`Node`]'s [`content`](Node::content), but not its [`children`](Node::children).
    /// The resulting cloned [`Node`] will have no **parent** or **children**.
    ///
    /// For a method that clones the [`Node`] *and* its subtree, see [`Node::clone_deep`].
    fn clone(&self) -> Self {
        Self(Rc::pin(RwLock::new(InnerNode::new(self.borrow().content.clone()))))
    }
}
impl<T> PartialEq for Node<T>
where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.borrow().eq(&*other.borrow())
    }
}
impl<T> Eq for Node<T>
where T: Eq {}
impl<T> Debug for Node<T>
where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.borrow(), f)
    }
}
