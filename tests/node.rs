use std::rc::Rc;
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
    assert_eq!(target.next_sibling().unwrap().as_ref(), Node::builder("child b").build().root());
    // Siblings of "child b"
    let target = &tree.root().children()[1];
    assert_eq!(target.prev_sibling().unwrap().as_ref(), Node::builder("child a").build().root());
    assert_eq!(target.next_sibling().unwrap().as_ref(), Node::builder("child c").build().root());
    // Siblings of "child c"
    let target = &tree.root().children()[2];
    assert_eq!(target.prev_sibling().unwrap().as_ref(), Node::builder("child b").build().root());
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
    
    let target = &root.children()[1]; // "child b"

    // Regular clone
    let clone = Node::clone(target);
    assert!(!clone.is_same_as(target));
    assert_eq!(clone.content, target.content);
    assert!(clone.parent.is_none());
    assert_eq!(clone.children(), vec![]);

    // Deep clone
    let clone = target.clone_deep();
    let clone = clone.root();
    assert!(!clone.is_same_as(target));
    assert_eq!(*clone, *target.as_ref());
    assert!(clone.parent.is_none());
}

#[test]
fn detach() {
    let mut tree = Node::builder("parent")
        .child(Node::builder("child a")
            .child(Node::builder("child d")))
        .child(Node::builder("child b"))
        .child(Node::builder("child c"))
        .build();
    let root = tree.root_mut();

    let target = root.children()[2].clone();
    let detached = root.detach_child(target.as_ref()).unwrap();
    assert!(target.is_same_as(detached.root()));
    assert_eq!(detached, Node::builder("child c").build());

    let target = root.children()[0].children()[0].clone();
    let detached = root.detach_child(target.as_ref()).unwrap();
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
    let mut tree = Node::builder("parent")
        .child(Node::builder("child a"))
        .child(Node::builder("child b")
            .child(Node::builder("child d")))
        .child(Node::builder("child c"))
        .build();
    let root = &mut tree.root;

    // -- Append a new node.
    let new = Node::builder("child e").build().root;
    root.append_child(Rc::clone(&new));
    assert!(root.children().last().unwrap().is_same_as(&new));

    // -- Append a node that was already in the tree.
    let target = &root.children()[1].children()[0].clone();
    let prev_parent = target.parent.as_ref().unwrap().upgrade().unwrap().clone();
    root.append_child(Rc::clone(&target));
    assert!(root.children().last().unwrap().is_same_as(&target));
    assert_eq!(prev_parent.children(), vec![]);

    // -- Append a node from another tree.
    let other_tree = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = &other_tree.root().children()[0];
    root.append_child(Rc::clone(&target));
    assert!(root.children().last().unwrap().is_same_as(&target));
    assert_eq!(other_tree.root().children(), vec![]);

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

// #[test]
// fn root_node() {
//     let tree = NodeBuilder {
//         content: "parent",
//         children: vec![
//             NodeBuilder {
//                 content: "child a",
//                 children: vec![
//                     NodeBuilder {
//                         content: "child b",
//                         children: vec![]
//                     }
//                 ]
//             }
//         ]
//     }.build();
//     let root = tree.root();
//
//     let target = root.children()[0].children()[0].clone();
//     assert!(target.root().is_same_as(root));
// }
