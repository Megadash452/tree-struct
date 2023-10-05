use tree_struct::{BaseNode as Node, Node as _};

#[test]
fn siblings() {
    let root = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();

    // Siblings of "child a"
    let target = root.children()[0];
    assert!(target.prev_sibling().is_none());
    assert_eq!(
        Node::builder("child b").build().as_ref().get_ref(),
        target.next_sibling().unwrap()
    );
    // Siblings of "child b"
    let target = root.children()[1];
    assert_eq!(
        Node::builder("child a").build().as_ref().get_ref(),
        target.prev_sibling().unwrap()
    );
    assert_eq!(
        Node::builder("child c").build().as_ref().get_ref(),
        target.next_sibling().unwrap()
    );
    // Siblings of "child c"
    let target = root.children()[2];
    assert_eq!(
        Node::builder("child b").build().as_ref().get_ref(),
        target.prev_sibling().unwrap()
    );
    assert!(target.next_sibling().is_none());
}

#[test]
fn clone() {
    let root = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();

    let target = root.children()[1].as_any().downcast_ref::<Node<&str>>().unwrap(); // "child b"

    // Regular clone
    let clone = target.clone();
    assert!(!clone.is_same_as(target));
    assert_eq!(
        clone.as_any().downcast_ref::<Node<&str>>().unwrap(),
        target
    );
    assert!(clone.parent().is_none());
    assert!(clone.children().is_empty());

    // Deep clone
    let clone = target.clone_deep();
    assert!(!clone.is_same_as(target));
    assert_eq!(clone.as_ref().get_ref(), target);
    assert!(clone.parent().is_none());
}

#[test]
fn detach() {
    let mut root = Node::builder("parent")
        .child(Node::builder("child a")
            .child(Node::builder("child d")))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();

    let target = root.children()[2].ptr();
    let detached = root.as_mut().detach_descendant(target).unwrap();
    assert!(detached.is_same_as(target.as_ptr()));
    assert_eq!(detached, Node::builder("child c").build());

    let target = root.children()[0].children()[0].ptr();
    let detached = root.as_mut().detach_descendant(target).unwrap();
    assert!(detached.is_same_as(target.as_ptr()));
    assert_eq!(detached, Node::builder("child d").build());

    assert_eq!(
        root,
        Node::builder("parent")
            .child(Node::builder("child a"))
            .child(Node::builder("child b"))
            .build()
    );
}

#[test]
fn append_child() {
    let mut root = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();

    // -- Append a new node.
    let new = Node::builder("child e").build();
    root.as_mut().append_child(new);
    assert_eq!(root.children().last().unwrap().as_any().downcast_ref::<Node<&str>>().unwrap().content, "child e");

    // -- Append a node that was already in the tree.
    let target = root.children()[1].children()[0].ptr();
    let detached = root.as_mut().detach_descendant(target).unwrap();
    root.as_mut().append_child(detached);
    assert!(root.children().last().unwrap().is_same_as(target.as_ptr()));
    assert_eq!(root.children().last().unwrap().as_any().downcast_ref::<Node<&str>>().unwrap().content, "child d");
    assert!(root.children()[1].children().is_empty());

    // -- Append a node from another tree.
    let mut other_root = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = other_root.children()[0].ptr();
    root.as_mut().append_child(other_root.as_mut().detach_descendant(target).unwrap());
    assert!(root.children().last().unwrap().is_same_as(target.as_ptr()));
    assert!(other_root.children().is_empty());

    // -- End
    assert_eq!(
        root,
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

// #[test]
// fn dangling() {
//     let mut root = Node::builder("parent")
//         .child(Node::builder("child")
//             .child(Node::builder("grandchild")))
//         .build();

//     // These will be dangling ptrs
//     let child = root.children()[0].ptr();
//     let grandchild = unsafe { child.as_ref().children()[0].ptr() };

//     drop(root.detach_descendant(child));

//     // All methods taking a ptr should return None when the ptr is dangling.
//     assert_eq!(root.detach_descendant(child), None);
//     assert_eq!(root.borrow_descendant(child), None);
//     assert_eq!(root.detach_descendant(grandchild), None);
//     assert_eq!(root.borrow_descendant(grandchild), None);
// }
