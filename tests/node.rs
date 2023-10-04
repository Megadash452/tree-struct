use tree_struct::Node;

#[test]
fn siblings() {
    let tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();
    let root = tree.root();

    // Siblings of "child a"
    let target = &*root.children()[0];
    assert_eq!(target.prev_sibling(), None);
    assert_eq!(
        target.next_sibling().unwrap(),
        Node::builder("child b").build().root()
    );
    // Siblings of "child b"
    let target = &*root.children()[1];
    assert_eq!(
        target.prev_sibling().unwrap(),
        Node::builder("child a").build().root()
    );
    assert_eq!(
        target.next_sibling().unwrap(),
        Node::builder("child c").build().root()
    );
    // Siblings of "child c"
    let target = &*root.children()[2];
    assert_eq!(
        target.prev_sibling().unwrap(),
        Node::builder("child b").build().root()
    );
    assert_eq!(target.next_sibling(), None);
}

#[test]
fn clone() {
    let tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();
    let root = tree.root();

    let target = &*root.children()[1]; // "child b"

    // Regular clone
    let clone = target.clone();
    assert!(!clone.is_same_as(target));
    assert_eq!(clone.content, target.content);
    assert!(clone.parent().is_none());
    assert!(clone.children().is_empty());

    // Deep clone
    let clone = target.clone_deep();
    let clone = clone.root();
    assert!(!clone.is_same_as(target));
    assert_eq!(clone, target);
    assert!(clone.parent().is_none());
}

#[test]
fn detach() {
    let mut tree = Node::builder("parent")
        .child(Node::builder("child a")
            .child(Node::builder("child d")))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();

    let target = tree.root().children()[2].ptr();
    let detached = tree.detach_descendant(target).unwrap();
    assert!(detached.root().is_same_as(target));
    assert_eq!(detached, Node::builder("child c").build());

    let target = tree.root().children()[0].children()[0].ptr();
    let detached = tree.detach_descendant(target).unwrap();
    assert!(detached.root().is_same_as(target));
    assert_eq!(detached, Node::builder("child d").build());

    assert_eq!(
        tree,
        Node::builder("parent")
            .child(Node::builder("child a"))
            .child(Node::builder("child b"))
            .build()
    );
}

#[test]
fn append_child() {
    let mut tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();

    // -- Append a new node.
    let new = Node::builder("child e").build();
    tree.root_mut().append_child(new);
    assert_eq!(tree.root().children().last().unwrap().content, "child e");

    // -- Append a node that was already in the tree.
    let target = tree.root().children()[1].children()[0].ptr();
    let detached = tree.detach_descendant(target).unwrap();
    // TODO: find a way to pass Pin<&mut T> to the function without moving it and without calling Pin::as_mut().
    let mut root = tree.root_mut();
    root.as_mut().append_child(detached);
    assert!(root.as_mut().children().last().unwrap().is_same_as(target));
    assert_eq!(root.children().last().unwrap().content, "child d");
    assert!(root.children()[1].children().is_empty());

    // -- Append a node from another tree.
    let mut other_tree = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = other_tree.root().children()[0].ptr();
    root.as_mut().append_child(other_tree.detach_descendant(target).unwrap());
    assert!(root.children().last().unwrap().is_same_as(target));
    assert!(other_tree.root().children().is_empty());

    // -- End
    assert_eq!(
        tree,
        Node::builder("parent")
            .child(Node::builder("child a"))
            .child(Node::builder("child b"))
            .child(Node::builder("child c"))
            .child(Node::builder("child e"))
            .child(Node::builder("child d"))
            .child(Node::builder("other child a"))
            .build()
    );
}

#[test]
fn dangling() {
    let mut tree = Node::builder("parent")
        .child(Node::builder("child")
            .child(Node::builder("grandchild")))
        .build();

    // These will be dangling ptrs
    let child = tree.root().children()[0].ptr();
    let grandchild = unsafe { child.as_ref().children()[0].ptr() };

    drop(tree.detach_descendant(child));

    // All methods taking a ptr should return None when the ptr is dangling.
    assert_eq!(tree.detach_descendant(child), None);
    assert_eq!(tree.borrow_descendant(child), None);
    assert_eq!(tree.detach_descendant(grandchild), None);
    assert_eq!(tree.borrow_descendant(grandchild), None);
}
