use super::*;


/// Helper struct to build a [`Tree`] of [`Node`]s.
/// 
/// ### Examples
/// Can be used as a [Builder Pattern](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html),
/// or something similar, but by assigning the fields.
/// 
/// ```
/// use tree_struct::{Node, NodeBuilder};
/// 
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
    pub children: Vec<NodeBuilder<T>>
}
impl<T> NodeBuilder<T> {
    /// New [`NodeBuilder`] using [Builder Pattern](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html).
    pub fn new(content: T) -> Self {
        NodeBuilder { content, children: vec![] }
    }
    pub fn child(mut self, child: Self) -> Self {
        self.children.push(child);
        self
    }

    /// Create a new [`Tree`] from nodes with **children** and **content**.
    /// The children will be made into [`Node`]s with the proper **parent**.
    /// All of the children will be put into [`Reference Counted Pointer`](Rc)s recursively.
    pub fn build(self) -> Tree<T> {
        let mut tree = Tree::from(Box::pin(UnsafeCell::new(Node {
            content: self.content,
            parent: None,
            children: vec![]
        })));

        tree.root_mut().children = Self::build_children(&*tree.root, self.children);

        tree
    }

    fn build_children(parent: ParentRef<T>, children: Vec<NodeBuilder<T>>) -> Vec<ChildNode<T>> {
        children.into_iter()
            .map(|builder| {
                let child = Box::pin(UnsafeCell::new(Node {
                    content: builder.content,
                    parent: Some(parent),
                    children: vec![]
                }));

                unsafe { &mut *child.get() }.children = Self::build_children(&*child, builder.children);

                child
            })
            .collect()
    }
}


#[derive(Default)]
pub struct Node<T> {
    parent: Option<ParentRef<T>>,
    children: Vec<ChildNode<T>>,
    pub content: T,
}
impl<T> Node<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Holds references to each **child**.
    pub fn children<'a>(&'a self) -> Vec<&'a Self> {
        self.children.iter()
            .map(|child| unsafe { &*child.get() })
            .collect()
    }
    pub fn parent(&self) -> Option<&Self> {
        self.parent.map(|p| unsafe { &*UnsafeCell::raw_get(p) })
    }


    /// Look at every ancestor of **other** until **self** is found. (Not recursive).
    fn has_descendant(&self, other: &Self) -> bool {
        let mut ancestor = other.parent();

        while let Some(node) = ancestor
        {
            if self.is_same_as(node) {
                return true
            }
            ancestor = node.parent();
        }

        false
    }
    fn find_self<'a>(&self, iter: impl Iterator<Item = &'a ChildNode<T>>) -> Option<&'a Self> {
        let mut iter = iter.map(|sib| unsafe { &*sib.get() });
        iter.find(|sib| self.is_same_as(sib));
        iter.next()//.map(|sib| &**sib)
    } 


    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(&self) -> Option<&Self> {
        self.find_self(self.parent()?.children.iter())
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(&self) -> Option<&Self> {
        self.find_self(self.parent()?.children.iter().rev())
    }

    /// Pushes the **child** to `this` [`Node`]'s *children*.
    /// Does nothing if `self` is **child**.
    pub fn append_child(&mut self, mut child: Tree<T>) {
        if self.is_same_as(child.root()) {
            return
        }

        child.root_mut().parent = Some(unsafe {
            std::mem::transmute(self as *const Self)
        });

        self.children.push(child.root)
    }

    /// If **self** [`is_same_as`](Node::is_same_as()) **descendant**,
    /// or if **descendant** is not a descendant of **self**, will return [`None`].
    /// See [`Self::detach_self()`].
    /// 
    /// This function should be called from the root node (*since for now it is the only node that you can get as `mut`*).
    /// 
    /// Ownership of the **descendant** [`Node`] is ***transferred to the caller***.
    /// 
    /// **Descendant** does not have to be `mut`.
    /// It should be enough to assert that the root node is `mut`, so by extension the descendant is also `mut`.
    /// This is helpful because **descendant** cannot be obtained as `mut` (*for now*).
    pub fn detach_descendant(&self, descendant: &Self) -> Option<Tree<T>> {
        if self.is_same_as(&*descendant)
        || !self.has_descendant(&*descendant) {
            return None
        }

        let parent = unsafe { &mut *UnsafeCell::raw_get(descendant.parent.unwrap()) };
        
        // Find the index of the node to be removed in its parent's children list
        let mut index = 0;
        for child in &parent.children {
            if descendant.is_same_as(unsafe { &*child.get() }) {
                break
            }
            index += 1
        }

        if index < parent.children.len() {
            // If children is not UnsafeCell, use std::mem::transmute(parent.children.remove(index)).
            let mut tree = Tree::from(parent.children.remove(index));
            tree.root_mut().parent = None;
            Some(tree)
        } else {
            None
        }
    }

    // /// Remove this [`Node`] from its **parent** (if it has one).
    // /// 
    // /// TODO: how this should be used
    // /// 
    // /// Ownership of the **child** [`Node`] is ***transferred to the caller***.
    // pub fn detach_self(self: &mut Rc<Self>) -> Tree<T> {
    //     if let Some(parent) = &self.parent {
    //         let parent = unsafe {
    //             &mut (*(Rc::as_ptr(&parent.upgrade().unwrap()) as *mut Self))
    //         };
    //
    //         // Find the index of the node to be removed in its parent's children list.
    //         // Will be = parent.children.len() if not found.
    //         let mut index = 0;
    //         for child in &parent.children {
    //             if self.is_same_as(child) {
    //                 break
    //             }
    //             index += 1
    //         }
    //
    //         if index < parent.children.len() {
    //             unsafe {
    //                 (*(Rc::as_ptr(self) as *mut Self)).parent = None;
    //             }
    //         }
    //     }
    //
    //     Rc::clone(self).into()
    // }

    // TODO: should return None if self is root??
    // /// Returns the *root* [`Node`], aka the first ancestor of this [`Node`].
    // /// The root node is the one that has no **parent**.
    // pub fn root(self: Rc<Self>) -> Option<Rc<Self>> {
    //     let mut current = self;
    //
    //     while let Some(parent) = current.parent.as_ref()
    //         .and_then(|parent| parent.upgrade())
    //     {
    //         current = parent
    //     }
    //
    //     match current.parent {
    //         Some(_) => None,
    //         None => Some(current)
    //     }
    // }

    #[inline]
    /// Whether two [`Node`]s are the same (that is, they reference the same object).
    pub fn is_same_as(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
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
            children: vec![]
        }
    }
}
impl <T: Clone> Node<T> {
    /// Copies the [`Node`]'s [`content`](Node::content) and its [`children`](Node::children) recursively.
    /// The resulting cloned [`Node`] will have no **parent**.
    /// 
    /// For a method that clones the [`Node`] but *not* its subtree, see [`Node::clone`].
    pub fn clone_deep(&self) -> Tree<T> {
        let mut tree = Tree::from(Box::pin(UnsafeCell::new(self.clone())));

        tree.root_mut().children = self.clone_children_deep(&*tree.root);
        
        tree
    }
    fn clone_children_deep(&self, parent: ParentRef<T>) -> Vec<ChildNode<T>> {
        self.children.iter()
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

impl<T: PartialEq /*+ ChildrenEq*/> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
        // TODO: some node types dont care if they have the same children. Add a trait for this. if does not implement the trait, only compare Node::content
        && self.children() == other.children()
    }
}
impl<T: Eq> Eq for Node<T> {
}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field("parent", &self.parent.map(|p| unsafe { &*p }))
            .field("children", &self.children)
            .finish()
    }
}
