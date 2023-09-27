use tree_struct::Node;

#[test]
fn siblings() {
    let tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();

    // Siblings of "child a"
    let target = &tree.root().children()[0];
    assert_eq!(target.prev_sibling(), None);
    assert_eq!(
        target.next_sibling().unwrap(),
        Node::builder("child b").build().root()
    );
    // Siblings of "child b"
    let target = &tree.root().children()[1];
    assert_eq!(
        target.prev_sibling().unwrap(),
        Node::builder("child a").build().root()
    );
    assert_eq!(
        target.next_sibling().unwrap(),
        Node::builder("child c").build().root()
    );
    // Siblings of "child c"
    let target = &tree.root().children()[2];
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

    let target = &tree.root().children()[1]; // "child b"

    // Regular clone
    let clone = target.clone();
    assert!(!clone.is_same_as(target));
    assert_eq!(&*clone.content(), &*target.content());
    assert!(clone.parent().is_none());
    assert!(clone.children().is_empty());

    // Deep clone
    let clone = target.clone_deep().root();
    // let clone = clone.root();
    assert!(!clone.is_same_as(target));
    assert_eq!(&clone, target);
    assert!(clone.parent().is_none());
}

#[test]
fn detach() {
    let tree = Node::builder("parent")
        .child(Node::builder("child a")
            .child(Node::builder("child d")))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();

    let target = &tree.root().children()[2];
    let detached = target.detach().unwrap();
    assert!(detached.root().is_same_as(target));
    assert_eq!(detached, Node::builder("child c").build());

    let target = &tree.root().children()[0].children()[0];
    let detached = target.detach().unwrap();
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
    let tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();

    // -- Append a new node.
    let new = Node::builder("child e").build();
    tree.root().append_child(new);
    assert_eq!(*tree.root().children().last().unwrap().content(), "child e");

    // -- Append a node that was already in the tree.
    let target = &tree.root().children()[1].children()[0];
    let detached = target.detach().unwrap();
    tree.root().append_child(detached);
    assert!(tree.root().children().last().unwrap().is_same_as(target));
    assert_eq!(*tree.root().children().last().unwrap().content(), "child d");
    assert!(tree.root().children()[1].children().is_empty());

    // -- Append a node from another tree.
    let other_tree = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = &other_tree.root().children()[0];
    tree.root().append_child(target.detach().unwrap());
    assert!(tree.root().children().last().unwrap().is_same_as(target));
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

// Doesn't need Dangling test. No Nodes can dangle because user can't (shouldn't) get a raw pointer to a Node.
