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
    assert_eq!(target.next_sibling().unwrap(), Node::builder("child b").build().root());
    // Siblings of "child b"
    let target = &*root.children()[1];
    assert_eq!(target.prev_sibling().unwrap(), Node::builder("child a").build().root());
    assert_eq!(target.next_sibling().unwrap(), Node::builder("child c").build().root());
    // Siblings of "child c"
    let target = &*root.children()[2];
    assert_eq!(target.prev_sibling().unwrap(), Node::builder("child b").build().root());
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
    let clone = Node::clone(target);
    assert!(!clone.is_same_as(target));
    assert_eq!(clone.content, target.content);
    assert!(clone.parent().is_none());
    assert_eq!(clone.children(), vec![]);

    // Deep clone
    let clone = target.clone_deep();
    let clone = clone.root();
    assert!(!clone.is_same_as(target));
    assert_eq!(clone, target);
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
    let root = tree.root();

    let target = &*root.children()[2];
    let detached = root.detach_descendant(target).unwrap();
    assert!(detached.root().is_same_as(target));
    assert_eq!(detached, Node::builder("child c").build());

    let target = &*root.children()[0].children()[0];
    let detached = root.detach_descendant(target).unwrap();
    assert!(detached.root().is_same_as(target));
    assert_eq!(detached, Node::builder("child d").build());

    assert_eq!(tree,
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
    let root = tree.root();

    // -- Append a new node.
    let new = Node::builder("child e").build();
    root.append_child(new);
    assert_eq!(&**root.children().last().unwrap(), Node::builder("child e").build().root());

    // -- Append a node that was already in the tree.
    let detached = root.detach_descendant(&*root.children()[1].children()[0]).unwrap();
    root.append_child(detached);
    // assert!(root.children().last().unwrap().is_same_as(target));
    assert_eq!(&**root.children().last().unwrap(), Node::builder("child d").build().root());
    assert_eq!(root.children()[1].children(), vec![]);

    // -- Append a node from another tree.
    let other_tree = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = &*other_tree.root().children()[0];
    root.append_child(other_tree.root().detach_descendant(target).unwrap());
    dbg!(target);
    assert!(root.children().last().unwrap().is_same_as(target));
    assert_eq!(*other_tree.root().children(), vec![]);

    // TODO: append self? root.append_child(tree);

    // -- End
    assert_eq!(tree,
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
