use super::*;
use std::marker::PhantomPinned;

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
        // Do not pin at first to be able to `Arc::downgrade()` freely.
        let root = Arc::new(RwLock::new(InnerNode::new(self.content)));
    
        // Don't need to unlock for other threads because the Node hasn't been released and is not used while this lock is alive.
        root.write().children = Self::build_children(
            Arc::downgrade(&root),
            self.children
        );
        
        // Can be pinned here because no other unpinned Arcs exist
        Tree::from(Node(unsafe { Pin::new_unchecked(root) }))
    }
    fn build_children(parent: Weak<InnerNode<T>>, children: Vec<Self>) -> Vec<Node<T>> {
        children.into_iter()
            .map(|builder| {
                // Do not pin at first to be able to `Arc::downgrade()` freely.
                let child = Arc::new(RwLock::new(InnerNode::new(builder.content)));
                let mut child_mut = child.write();

                // Don't need to unlock for other threads because the Node hasn't been released and is not used while this lock is alive.
                child_mut.parent = Some(Weak::clone(&parent));

                child_mut.children = Self::build_children(
                    Arc::downgrade(&child),
                    builder.children
                );

                drop(child_mut);
                // Can be pinned here because no other unpinned Arcs exist
                Node(unsafe { Pin::new_unchecked(child) })
            })
            .collect()
    }
}

#[derive(Default)]
pub struct InnerNode<T> {
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
impl<T: Debug> Debug for InnerNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field("children", &self
                .children
                .iter()
                .map(|c| RwLockReadGuard::map(c.read(), |c| &c.content))
                .collect::<Box<_>>()
            )
            .finish()
    }
}
impl<T: PartialEq> PartialEq for InnerNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T: Eq> Eq for InnerNode<T> {}


/// The outward-facing Node is the node struct wrapped in a [`RwLock`] and [`reference counted pointer`](Arc).
/// 
/// A [`Node`] has 1 [`parent`](Self::parent()) and multiple [`children`](Self::children()).
/// It also stores [`content`](Self::content()) of type **`T`**.
pub struct Node<T>(Pin<Arc<RwLock<InnerNode<T>>>>);
impl<T> Node<T> {
    #[inline]
    fn read(&self) -> RwLockReadGuard<InnerNode<T>> {
        self.0.read()
    }
    fn write(&self) -> Pin<RwLockWriteGuard<InnerNode<T>>> {
        unsafe { Pin::new_unchecked(self.0.write()) }
    }
    fn downgrade(&self) -> Weak<InnerNode<T>> {
        // Casting Pin<P> to P is ok as long as nothing is moved later
        unsafe { Arc::downgrade(&*(&self.0 as *const _ as *const Arc<_>)) }
    }

    /// Check through all children of parent until `self` is found.
    fn find_self_next<'a>(&'a self, mut iter: impl Iterator<Item = &'a Self>) -> Option<Self> {
        iter.find(|sib| self.is_same_as(sib));
        iter.next().map(Node::ref_clone)
    }

    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn parent(&self) -> Option<Self> {
        self.read()
            .parent
            .as_ref()
            .and_then(|p| unsafe {
                Some(Self(Pin::new_unchecked(Weak::upgrade(p)?)))
            })
    }
    /// Allocates a *slice* of all of [`Node`]'s children, increasing all of their *reference counter*.
    pub fn children(&self) -> Box<[Self]> {
        self.read()
            .children
            .iter()
            .map(|c| c.ref_clone())
            .collect()
    }
    pub fn content(&self) -> Read<T> {
        RwLockReadGuard::map(self.read(), |n| &n.content)
    }
    pub fn content_mut(&self) -> Write<T> {
        RwLockWriteGuard::map(unsafe { Pin::into_inner_unchecked(self.write()) }, |n| &mut n.content)
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
            child.root.write().as_mut().get_unchecked_mut().parent = Some(self.downgrade());
            self.write().as_mut().get_unchecked_mut().children.push(child.root)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    /// **child** is required to be a **root** (i.e. have no *parent*), and [`Tree`] guarantees that.
    /// 
    /// Also see [`Self::append_child()`].
    pub fn insert_child(&self, child: Tree<T>, index: usize) {
        unsafe {
            child.root.write().as_mut().get_unchecked_mut().parent = Some(self.downgrade());
            self.write().as_mut().get_unchecked_mut().children.insert(index, child.root)
        }
    }

    /// Removes **this** [`Node`] from its **parent** and returns the *detached [`Node`]* with ownership (aka a [`Tree`]).
    /// If `self` has no **parent**, either because it is a *root* or it is not part of a [`Tree`], this will return [`None`].
    pub fn detach(&self) -> Option<Tree<T>> {
        let parent = self.parent()?;
        let mut parent = parent.write();
        let parent = unsafe { parent.as_mut().get_unchecked_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| self.is_same_as(child))
            .expect("Node is not found in its parent");

        let root = parent.children.remove(index);
        unsafe { root.write().as_mut().get_unchecked_mut().parent = None };
        Some(Tree::from(root))
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

    /// Clones the [`Arc`] and increments the internal reference counter of this [`Node`].
    pub fn ref_clone(&self) -> Self {
        Self(Pin::clone(&self.0))
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: &Self) -> bool {
        unsafe {
            Arc::<RwLock<InnerNode<T>>>::ptr_eq(
                // Casting Pin<P> to P is ok as long as nothing is moved later
                &*(&self.0 as *const _ as *const Arc<_>),
                &*(&other.0 as *const _ as *const Arc<_>)
            )
        }
    }
}
impl<T: Clone> Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        // Do not pin at first to be able to `Arc::downgrade()` freely.
        let root = Arc::new(RwLock::new(InnerNode::new(self.read().content.clone())));

        root.write().children = self.clone_children_deep(Arc::downgrade(&root));

        // Can be pinned here because no other unpinned Arcs exist
        Tree::from(Self(unsafe { Pin::new_unchecked(root) }))
    }
    fn clone_children_deep(&self, parent: Weak<InnerNode<T>>) -> Vec<Self> {
        self.children()
            .iter()
            .map(|child| {
                // Do not pin at first to be able to `Arc::downgrade()` freely.
                let cloned = Arc::new(RwLock::new(InnerNode::new(child.read().content.clone())));
                let mut cloned_mut  = cloned.write();

                // Assign the parent with the obtained lock
                cloned_mut.parent = Some(Weak::clone(&parent));
                // Unlock momentarily to let other threads aquire a lock while this thread clones the children of child.
                RwLockWriteGuard::unlock_fair(cloned_mut);
                let children = child.clone_children_deep(Arc::downgrade(&cloned));

                // Lock again to assign the children.
                cloned_mut = cloned.write();
                cloned_mut.children = children;

                drop(cloned_mut);
                // Can be pinned here because no other unpinned Arcs exist
                Self(unsafe { Pin::new_unchecked(cloned) })
            })
            .collect()
    }
}
impl<T: Debug> Node<T> {
    /// [`Debug`] the entire subtree (`self` and its **children**).
    #[inline]
    pub fn debug_tree(&self) -> DebugTree<T> {
        DebugTree { root: self.read() }
    }
}

impl<T: Clone> Clone for Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content), but not its [`children`](Node::children).
    /// The resulting cloned [`Node`] will have no **parent** or **children**.
    ///
    /// For a method that clones the [`Node`] *and* its subtree, see [`Node::clone_deep`].
    fn clone(&self) -> Self {
        Self(Arc::pin(RwLock::new(InnerNode::new(self.read().content.clone()))))
    }
}
impl<T: Default> Default for Node<T> {
    fn default() -> Self {
        Self(Arc::pin(RwLock::new(InnerNode::default())))
    }
}
impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.read().eq(&*other.read())
    }
}
impl<T: Eq> Eq for Node<T> {}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.read(), f)
    }
}
