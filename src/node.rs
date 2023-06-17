use std::ptr::NonNull;
use ptrplus::AsPtr;
use super::*;

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
#[derive(Default)]
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
        let mut tree = Tree::from(Box::pin(UnsafeCell::new(Node {
            content: self.content,
            parent: None,
            children: vec![],
        })));

        tree.root_mut().children = Self::build_children(&*tree.root, self.children);

        tree
    }
    fn build_children(parent: Parent<Node<T>>, children: Vec<Self>) -> Vec<Owned<Node<T>>> {
        children
            .into_iter()
            .map(|builder| {
                let child = Box::pin(UnsafeCell::new(Node {
                    content: builder.content,
                    parent: Some(parent),
                    children: vec![],
                }));

                unsafe { &mut *child.get() }.children =
                    Self::build_children(&*child, builder.children);

                child
            })
            .collect()
    }
}

#[derive(Default)]
pub struct Node<T> {
    parent: Option<Parent<Self>>,
    children: Vec<Owned<Self>>,
    pub content: T,
}
impl<T> Node<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Holds references to each **child**.
    pub fn children(&self) -> Vec<&Self> {
        self.children
            .iter()
            .map(|child| unsafe { &*child.get() })
            .collect()
    }
    pub fn parent(&self) -> Option<&Self> {
        self.parent.map(|p| unsafe { &*UnsafeCell::raw_get(p) })
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
        let mut iter = iter.map(|sib| unsafe { &*sib.get() });
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

    /// Pushes the **child** to **self**'s *children*.
    pub fn append_child(&mut self, mut child: Tree<T>) {
        // Compiler ensures `self != child.root`.
        child.root_mut().parent = Some(unsafe { std::mem::transmute(self as *const Self) });
        self.children.push(child.root)
    }

    /// If **self** [`is_same_as`](Self::is_same_as()) **descendant**,
    /// or if **descendant** is not an actual descendant of **self**, will return [`None`].
    ///
    /// This function should be called from the *root [`Node`]* (*since it is the only node that can be obtained as `mut`*).
    ///
    /// Ownership of the **descendant** [`Node`] is ***transferred to the caller*** (as a [`Tree`]).
    /// **descendant** must be a *NonNull pointer* (obtained from [`Node::ptr`]) because, if it was a reference,
    /// the borrow checker will consider the entire [`Tree`] to be *immutably borrowed* (including *self*).
    /// The **descendant** pointer passed to this function will remain valid because it is [`Pin`]ned.
    /// 
    /// # Example
    /// ```
    /// # use tree_struct::Node;
    /// # let mut tree = Node::builder(0).child(Node::builder(1)).child(Node::builder(2)).build();
    /// let root = tree.root_mut();
    /// let target = root.children()[1].ptr();
    /// let detached = root.detach_descendant(target).unwrap();
    /// ```
    ///
    /// **descendant** does not have to be `mut`.
    /// It should be enough to assert that the root node is `mut`, so by extension the descendant is also `mut`.
    /// This is helpful because **descendant** cannot be obtained as `mut` (*for now*).
    pub fn detach_descendant(&mut self, descendant: NonNull<Self>) -> Option<Tree<T>> {
        if self.is_same_as(descendant)
        || !self.has_descendant(descendant) {
            return None;
        }

        let parent = unsafe { &mut *UnsafeCell::raw_get(descendant.as_ref().parent.unwrap()) };

        // Find the index of **descendant** to remove it from its parent's children list
        let index = parent.children.iter()
            .position(|child| descendant.as_ptr() == child.get())
            .expect("Node is not found in its parent");

        // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
        let mut tree = Tree::from(parent.children.remove(index));
        tree.root_mut().parent = None;
        Some(tree)
    }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: impl AsPtr<Raw=Self>) -> bool {
        std::ptr::eq(self, other.as_ptr())
    }
    #[inline]
    /// Get a *[`NonNull`] pointer* for **self**, which should only be treated as a `*const Self`.
    /// Useful for [`Self::detach_descendant`].
    /// To get a *raw pointer* (*const Self) do `.ptr().as_ptr()`.
    pub fn ptr(&self) -> NonNull<Self> {
        NonNull::from(self)
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
        }
    }
}
impl<T: Clone> Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    ///
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        let mut tree = Tree::from(Box::pin(UnsafeCell::new(self.clone())));

        tree.root_mut().children = self.clone_children_deep(&*tree.root);

        tree
    }
    fn clone_children_deep(&self, parent: Parent<Self>) -> Vec<Owned<Self>> {
        self.children
            .iter()
            .map(|node| unsafe { &*node.get() })
            .map(|node| {
                let child = Box::pin(UnsafeCell::new(node.clone()));
                let mut_child = unsafe { &mut *child.get() };
                mut_child.parent = Some(parent);
                mut_child.children = node.clone_children_deep(&*child);
                child
            })
            .collect()
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
            .field("parent", &self.parent.map(|p| unsafe { &(*(*p).get()).content }))
            .field("children", &self.children())
            .finish()
    }
}
