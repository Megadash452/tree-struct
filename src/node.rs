use super::*;
use ptrplus::AsPtr;
use std::{marker::PhantomPinned, ptr::NonNull};

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
        let mut root = Box::pin(Node {
            content: self.content,
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        });

        unsafe { root.as_mut().get_unchecked_mut() }.children =
            Self::build_children(root.ptr(), self.children);

        Tree { root }
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

                unsafe { child.as_mut().get_unchecked_mut() }.children =
                    Self::build_children(child.ptr(), builder.children);

                child
            })
            .collect()
    }
}

/// A [`Node`] has 1 [`parent`](Self::parent()) and multiple [`children`](Self::children()).
/// It also stores [`content`](Self::content) of type **`T`**.
///
/// A Node is [`heap-allocated`](Box) and [`pinned`](Pin) to allow storing a reference to the parent (Node is a *self referential struct*)
/// without the data of said parent being moved.
/// The pointer to the parent must be valid for the lifetime of the Node that holds the pointer.
///
/// Therefore, in theory, a *stack-allocated unpinned* Node should not exist, but that is ok as long as the Node has *no children*.
/// The current implementation of the methods allows asserting that such Node has no children
/// because [`adding children`](Self::append_child()) (i.e. using a *mutable Node*) requires it to be **[`Pin`]ned**.
/// A user can still use [`std::pin::pin!`] on a *stack-allocated* Node and add children to it,
/// but the Node *can't be moved*, and its children are dropped along with it,
/// so the pointer it's children hold **remains valid for their lifetimes**.
///
/// This allows the Node struct to implement traits that require returning a *stack-allocated* Node (e.g. [`Default`] and [`Clone`]).
/// However, it is recommended to convert the returned [`Node`] into a [`Tree`] using `Tree::from()` or `Node::into()` as an "ez mode"
/// for getting rid of compiler errors that are caused by trying to use `&mut Node` or trying to move it.
pub struct Node<T> {
    pub content: T,
    parent: Option<Parent<Self>>,
    children: Vec<Owned<Self>>,
    _pin: PhantomPinned,
}
impl<T> Node<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Get an *immutable reference* to the `parent` [`Node`] of `self`.
    /// To get a *mutable reference*,
    /// call [`crate::Tree::borrow_descendant()`] from the owner [`Tree`] with `self.parent().ptr()`.
    pub fn parent(&self) -> Option<&Self> {
        self.parent.map(|p| unsafe { p.as_ref() })
    }
    /// Holds references to each **child**.
    /// /// To get a *mutable reference* to one of the **children**,
    /// call [`crate::Tree::borrow_descendant()`] from the owner [`Tree`] with `self.parent().ptr()`.
    pub fn children(&self) -> Box<[&Self]> {
        self.children
            .iter()
            .map(|child| child.as_ref().get_ref())
            .collect()
    }

    /// A [`Node`] is a **descendant** of another [`Node`] if:
    /// 1. The two [`Node`]s are not the same ([`std::ptr::eq()`]).
    /// 2. Looking up the [`Tree`] from `other`, `self` is found to be one of `other`'s ancestors. (Not recursive).
    fn is_descendant(&self, other: NonNull<Self>) -> bool {
        if self.is_same_as(other) {
            return false;
        }

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
    /// Also see [`Self::insert_child()`].
    pub fn append_child(self: Pin<&mut Self>, mut child: Tree<T>) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut();
            child.root_mut().get_unchecked_mut().parent = Some(NonNull::new_unchecked(this));
            this.children.push(child.root)
        }
    }
    /// Inserts the **child** to **self**'s *children* at some index.
    /// Also see [`Self::append_child()`].
    pub fn insert_child(self: Pin<&mut Self>, mut child: Tree<T>, index: usize) {
        // Compiler ensures `self != child.root`.
        unsafe {
            let this = self.get_unchecked_mut();
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
        if !self.is_descendant(descendant) {
            return None;
        }

        let parent = unsafe { descendant.as_ref().parent.unwrap().as_mut() };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent
            .children
            .iter()
            .position(|child| descendant.as_ptr() == child.ptr().as_ptr())
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let mut root = parent.children.remove(index);
        unsafe { root.as_mut().get_unchecked_mut() }.parent = None;
        Some(Tree { root })
    }

    /// See [`crate::Tree::borrow_descendant()`].
    /// TODO: Don't know if should make it public.
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the whole [`Tree`] is `mut`, so by extension the **descendant** is also `mut`.
    pub(super) fn borrow_descendant(self: Pin<&mut Self>, descendant: NonNull<Self>) -> Option<Pin<&mut Self>> {
        if self.is_descendant(descendant) {
            Some(unsafe { Pin::new_unchecked(&mut *descendant.as_ptr()) })
        } else {
            None
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
    pub fn ptr(&self) -> NonNull<Self> {
        NonNull::from(self)
    }
}

impl<T: Default> Default for Node<T> {
    /// Creates a Node with the Default content.
    /// Converting the returned Node to a [`Tree`] is recommended.
    fn default() -> Self {
        Self {
            content: T::default(),
            parent: None,
            children: vec![],
            _pin: PhantomPinned,
        }
    }
}

impl<T: Clone> Clone for Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content), but not its [`children`](Node::children).
    /// The resulting cloned [`Node`] will have no **parent** or **children**.
    ///
    /// Converting the returned Node to a [`Tree`] is recommended.
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
        let mut root = Box::pin(self.clone());

        unsafe { root.as_mut().get_unchecked_mut() }.children =
            self.clone_children_deep(root.ptr());

        Tree { root }
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

impl<T: Debug> Node<T> {
    /// [`Debug`] the entire subtree (`self` and its **children**).
    #[inline]
    pub fn debug_tree(&self) -> DebugTree<T> {
        DebugTree { root: self }
    }
}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field(
                "children",
                &self
                    .children()
                    .iter()
                    .map(|c| &c.content)
                    .collect::<Box<_>>(),
            )
            .finish()
    }
}

impl<T: PartialEq> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}
impl<T: Eq> Eq for Node<T> {}
