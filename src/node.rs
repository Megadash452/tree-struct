use std::rc::Weak;
use super::*;

pub type StrongNode<T> = Rc<RefCell<Node<T>>>;
pub type WeakNode<T> = Weak<RefCell<Node<T>>>;
pub type Strong<T> = Rc<RefCell<T>>;


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
    pub children: Vec<Self>
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
        let mut tree = Tree::from(Rc::new(RefCell::new(Node {
            content: self.content,
            parent: None,
            children: vec![]
        })));
    
        tree.root_mut().children = Self::build_children(Rc::downgrade(&tree.root), self.children);
    
        tree
    }
    fn build_children(parent: WeakNode<T>, children: Vec<Self>) -> Vec<StrongNode<T>> {
        children.into_iter()
            .map(|builder| {
                let mut child = Rc::new(RefCell::new(Node {
                    content: builder.content,
                    parent: Some(Weak::clone(&parent)),
                    children: vec![]
                }));

                unsafe {
                    Rc::get_mut_unchecked(&mut child)
                }.get_mut().children = Self::build_children(Rc::downgrade(&child), builder.children);

                child
            })
            .collect()
    }
}


#[derive(Default)]
pub struct Node<T> {
    pub parent: Option<WeakNode<T>>,
    pub children: Vec<StrongNode<T>>,
    pub content: T,
}
impl<T> Node<T> {
    #[inline]
    pub fn builder(content: T) -> NodeBuilder<T> {
        NodeBuilder::new(content)
    }

    pub fn children<'a>(self: &Ref<'a, Self>) -> Ref<'a, [StrongNode<T>]> {
        Ref::map(Ref::clone(self), |n| n.children.as_slice())
    }
    pub fn parent<'a>(self: &Ref<'a, Self>) -> Option<Ref<'a, Self>> {
        self.parent.as_ref().and_then(Weak::upgrade).map(|p| p.borrow())
    }


    /// Look at every ancestor of **other** until **self** is found. (Not recursive).
    fn has_descendant(&self, other: &Self) -> bool {
        let mut ancestor = other.parent.as_ref().and_then(Weak::upgrade);

        while let Some(node) = ancestor
        {
            if unsafe { self.is_same_as(&*RefCell::as_ptr(&node)) } {
                return true
            }
            ancestor = node.borrow().parent.as_ref().and_then(Weak::upgrade)
        }

        false
    }


    /// Returns the [`Node`] immediately following this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *last* child of the **parent**.
    pub fn next_sibling(self: &Ref<Self>) -> Option<StrongNode<T>> { // TODO: return borrowed node
        let parent = self.parent.as_ref()?.upgrade()?;
        let parent = parent.borrow();
        let mut siblings = parent.children.iter();

        siblings.find(|sib| self.is_same_as(&sib.borrow()));
        siblings.next().map(Rc::clone)
    }
    /// Returns the [`Node`] immediately preceeding this one in the **parent**'s [`children`](Node::children).
    /// Otherwise returns [`None`] if `self` has no **parent**, or if it is the *first* child of the **parent**.
    pub fn prev_sibling(self: &Ref<Self>) -> Option<StrongNode<T>> { // TODO: return borrowed node
        let parent = self.parent.as_ref()?.upgrade()?;
        let parent = parent.borrow();
        let mut siblings = parent.children.iter().rev();

        siblings.find(|sib| self.is_same_as(&sib.borrow()));
        siblings.next().map(Rc::clone)
    }

    /// Pushes the **child** to `this` [`Node`]'s *children*,
    /// removing the **child** from it's previous parent.
    pub fn append_child(this: Strong<Self>, child: impl Into<Tree<T>>) { // TODO: use self
        let child = child.into();
        let child = child.root;
        if let Some(parent) = &child.borrow().parent {
            parent.upgrade().unwrap().borrow_mut().detach_descendant(&mut child.borrow_mut());
        }
        child.borrow_mut().parent = Some(Rc::downgrade(&this));
        this.borrow_mut().children.push(child);
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
    /// This is helpful because **descendantren** cannot be obtained as `mut` (*for now*).
    pub fn detach_descendant(self: &RefMut<Self>, descendant: &mut RefMut<Self>) -> Option<Tree<T>> {
        if self.is_same_as(&*descendant)
        || !self.has_descendant(&*descendant) {
            return None
        }

        let parent = descendant.parent.as_ref().unwrap().upgrade().unwrap();
        // `parent.borrow_mut()` will necessarily panic.
        // let parent = &mut parent.borrow_mut();
        let parent = unsafe { &mut *RefCell::as_ptr(&parent) };
        
        // Find the index of the node to be removed in its parent's children list
        let mut index = 0;
        for c in &parent.children {
            if unsafe { descendant.is_same_as(&*RefCell::as_ptr(&c)) } {
                break
            }
            index += 1
        }

        if index < parent.children.len() {
            // `parent.child.borrow_mut()` will necessarily panic.
            // parent.children.get_mut(index).unwrap().borrow_mut().parent = None;
            descendant.parent = None;
            let rtrn = Tree::from(parent.children.remove(index));
            assert!(descendant.is_same_as(unsafe { &*RefCell::as_ptr(&rtrn.root) }));
            Some(rtrn)
            // Some(parent.children.remove(index).into())
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
        let this = Rc::new(RefCell::new(self.clone()));

        let children = self.children.iter()
            .map(|child| child.borrow().clone_deep().root)
            .collect::<Vec<_>>();
        this.borrow_mut().children = children;
        
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
impl<T: PartialEq> PartialEq<StrongNode<T>> for Node<T> {
    /// Compares this [`Node`] with the **root** [`Node`] of the [`Tree`].
    fn eq(&self, other: &StrongNode<T>) -> bool {
        self.eq(&*other.borrow())
    }
}
impl<T: PartialEq> PartialEq<Ref<'_, Node<T>>> for Node<T> {
    /// Compares this [`Node`] with the **root** [`Node`] of the [`Tree`].
    fn eq(&self, other: &Ref<'_, Node<T>>) -> bool {
        self.eq(&**other)
    }
}
impl<T: PartialEq, O> PartialEq<O> for Node<T>
where O: AsRef<Self> {
    /// Compares this [`Node`] with the **root** [`Node`] of the [`Tree`].
    fn eq(&self, other: &O) -> bool {
        self.eq(other.as_ref())
    }
}
impl<T: Eq> Eq for Node<T> {
}
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("content", &self.content)
            .field("parent", &self.parent.as_ref()
                .and_then(Weak::upgrade)
                .map(|p| &unsafe { &*RefCell::as_ptr(&p) }.content)
            )
            .field("children", &self.children.iter()
                .map(|n| unsafe { &*RefCell::as_ptr(n) })
                .collect::<Vec<_>>()
            )
            .finish()
    }
}
