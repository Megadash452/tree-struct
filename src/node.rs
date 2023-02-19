use std::rc::Weak;
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
        Node::make(None, self).into()
    }
}


#[derive(Default)]
pub struct Node<T> {
    pub parent: Option<Weak<Node<T>>>,
    // [`Vec`] covers methods: firstChild, lastChild
    /// TODO: Maybe use RefCell for interior mutability?
    children: Vec<Rc<Node<T>>>,
    pub content: T,
}
impl<T> Node<T> {
    fn builders_to_node(parent: Weak<Node<T>>, children: Vec<NodeBuilder<T>>) -> Vec<Rc<Self>> {
        children.into_iter()
            .map(|builder|
                Self::make(Some(Weak::clone(&parent)), builder)
            )
            .collect()
    }
    pub(crate) fn make(parent: Option<Weak<Node<T>>>, builder: NodeBuilder<T>) -> Rc<Self> {
        #[allow(unused_mut)]
        let mut root = Rc::new(Self {
            content: builder.content,
            parent,
            children: vec![]
        });

        // Force assign the children to the Node
        unsafe {
            // The children will be wrapped in RC and point to the rtrn Node as the parent
            let children = Self::builders_to_node(Rc::downgrade(&root), builder.children);
            (*(Rc::as_ptr(&root) as *mut Self)).children = children
        }

        root
    }

    /// Look at every ancestor of **other** until **self** is found. (Not recursive).
    fn has_descendant(&self, other: &Self) -> bool {
        let mut ancestor = other.parent.as_ref()
            .and_then(|parent| parent.upgrade());

        while let Some(node) = ancestor
        {
            if self.is_same_as(&node) {
                return true
            }
            ancestor = node.parent.as_ref()
                .and_then(|parent| parent.upgrade())
        }

        false
    }

    // // New [`Node`] with no **children** or **parent**.
    // pub fn new(content: T) -> Tree<T> {
    //     Rc::new(Self { content, parent: None, children: vec![] }).into()
    // }
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(&self) -> Option<Rc<Self>> { // TODO: return borrowed node
        let parent = self.parent.as_ref()?.upgrade()?;
        let mut siblings = parent.children.iter();

        siblings.find(|sib| self.is_same_as(sib));
        siblings.next().map(Rc::clone)
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(&self) -> Option<Rc<Self>> { // TODO: return borrowed node
        let siblings = &self.parent.as_ref()?.upgrade()?.children;
        let mut siblings = siblings.iter().rev();

        siblings.find(|sib| self.is_same_as(sib));
        siblings.next().map(Rc::clone)
    }

    #[inline]
    pub fn children(&self) -> &[Rc<Self>] {
        self.children.as_slice()
    }

    /// Pushes the **child** to `this` [`Node`]'s *children*,
    /// removing the **child** from it's previous parent.
    pub fn append_child(self: &mut Rc<Self>, child: impl Into<Tree<T>>) {
        let child = child.into();
        let child_root = unsafe {
            &mut *(Rc::as_ptr(&child.root) as *mut Self)
        };
        let this = unsafe {
            &mut *(Rc::as_ptr(self) as *mut Self)
        };
        if let Some(parent) = &child_root.parent {
            unsafe {
                &mut *(Rc::as_ptr(&parent.upgrade().unwrap()) as *mut Self)
            }.detach_child(child_root);
        }
        this.children.push(child.root);
        child_root.parent = Some(Rc::downgrade(self));
    }

    /// If **self** [`is_same_as`](Node::is_same_as()) **child**,
    /// or if **child** is not a descendant of **self**, will return [`None`].
    /// See [`Self::detach_self()`].
    /// 
    /// This function should be called from the root node (*since for now it is the only node that you can get as `mut`*).
    /// 
    /// Ownership of the **child** [`Node`] is ***transferred to the caller***.
    /// 
    /// **Child** does not have to be `mut`.
    /// It should be enough to assert that the root node is `mut`, so by extension the child is also `mut`.
    /// This is helpful because **children** cannot be obtained as `mut` (*for now*).
    pub fn detach_child(&mut self, child: &Self) -> Option<Tree<T>> {
        if self.is_same_as(child)
        || !self.has_descendant(child) {
            return None
        }

        let target = child;
        let parent = unsafe {
            &mut (*(Rc::as_ptr(&child.parent.as_ref().unwrap().upgrade().unwrap()) as *mut Self))
        };

        // Find the index of the node to be removed in its parent's children list
        let mut index = 0;
        for child in &parent.children {
            if target.is_same_as(child) {
                break
            }
            index += 1
        }

        if index < parent.children.len() {
            unsafe {
                (*(target as *const Self as *mut Self)).parent = None;
            }
            Some(parent.children.remove(index).into())
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
    /// The [`Node`] does not need to be returned in a [`Reference Counted Pointer`](std::rc::Rc)
    /// because it has no [`children`](Node::children), which reference their parent with a [`Weak`].
    /// 
    /// Must be used as `Node::clone(self)` because if used as `self.clone()`
    /// it will clone the [`Rc`] and not the [`Node`].
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
        #[allow(unused_mut)]
        let mut this = Rc::new(Self::clone(self));

        // Force assign the children to the Node
        unsafe {
            let children = self.children.iter()
                .map(|child| child.clone_deep().root)
                .collect::<Vec<_>>();
            (*(Rc::as_ptr(&this) as *mut Self)).children = children
        }
        
        this.into()
    }
}

// impl<T> Deref for Node<T> {
//     type Target = T;
// 
//     fn deref(&self) -> &Self::Target {
//         &self.content
//     }
// }
impl<T: PartialEq /*+ ChildrenEq*/> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
        // TODO: some node types dont care if they have the same children. Add a trait for this. if does not implement the trait, only compare Node::content
        && self.children == other.children
    }
}
impl<T: PartialEq> PartialEq<Tree<T>> for Node<T> {
    /// Compares this [`Node`] with the **root** [`Node`] of the [`Tree`].
    fn eq(&self, other: &Tree<T>) -> bool {
        self.eq(other.root.as_ref())
    }
}
impl<T: Eq> Eq for Node<T> {
}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Only print the `content` of the parent
        let parent = self.parent.as_ref().and_then(|parent| parent.upgrade());
        let parent_content = parent.map(|parent| unsafe {
            &*(&parent.content as *const T)
        });

        f.debug_struct("Node")
            .field("content", &self.content)
            .field("parent", &parent_content)
            .field("children", &self.children)
            .finish()
    }
}
