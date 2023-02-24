use std::cell::RefCell;

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
    let target = &root.children()[0];
    assert_eq!(target.borrow().prev_sibling(), None);
    assert_eq!(*target.borrow().next_sibling().unwrap().borrow(), Node::builder("child b").build().root());
    // Siblings of "child b"
    let target = &root.children()[1];
    assert_eq!(*target.borrow().prev_sibling().unwrap().borrow(), Node::builder("child a").build().root());
    assert_eq!(*target.borrow().next_sibling().unwrap().borrow(), Node::builder("child c").build().root());
    // Siblings of "child c"
    let target = &root.children()[2];
    assert_eq!(*target.borrow().prev_sibling().unwrap().borrow(), Node::builder("child b").build().root());
    assert_eq!(target.borrow().next_sibling(), None);
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
    let clone = Node::clone(&target.borrow());
    assert!(!clone.is_same_as(&target.borrow()));
    assert_eq!(clone.content, target.borrow().content);
    assert!(clone.parent.is_none());
    assert_eq!(*clone.children, vec![]);

    // Deep clone
    let clone = target.borrow().clone_deep();
    let clone = clone.root();
    assert!(!clone.is_same_as(&target.borrow()));
    assert_eq!(*clone, *target);
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

    let mut target = &root.children[0];
    let mut detached = root.detach_descendant(&mut target.borrow_mut()).unwrap();
    assert!(target.borrow().is_same_as(unsafe { &*RefCell::as_ptr(&detached.root) }));
    // assert!(target.borrow().is_same_as(&*detached.root()));
    // assert!(detached.root().is_same_as(unsafe { &*RefCell::as_ptr(target) }));
    assert_eq!(detached, Node::builder("child c").build());

    let binding = root.children[0].borrow().children();
    target = &binding[0];
    detached = root.detach_descendant(&mut target.borrow_mut()).unwrap();
    assert_eq!(detached, Node::builder("child d").build());

    // drop(target);
    // drop(binding);
    // drop(detached);
    // drop(root);

    // assert_eq!(tree,
    //     Node::builder("parent")
    //         .child(Node::builder("child a"))
    //         .child(Node::builder("child b"))
    //         .build()
    // );
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
    Node::append_child(root.clone(), &new);
    assert!(root.borrow().children().last().unwrap().borrow().is_same_as(&new.borrow()));

    // -- Append a node that was already in the tree.
    let binding = root.borrow().children();
    let target = &binding[1].borrow().children()[0];
    let prev_parent = target.borrow().parent.as_ref().unwrap().upgrade().unwrap();
    Node::append_child(root.clone(), target);
    assert!(root.borrow().children().last().unwrap().borrow().is_same_as(&target.borrow()));
    assert_eq!(*prev_parent.borrow().children(), vec![]);

    // -- Append a node from another tree.
    let other_tree = Node::builder("other parent")
        .child(Node::builder("other child a"))
        .build();

    let target = &other_tree.root().children()[0];
    Node::append_child(root.clone(), target);
    assert!(root.borrow().children().last().unwrap().borrow().is_same_as(&target.borrow()));
    assert_eq!(*other_tree.root().children(), vec![]);

    // -- End
    // assert_eq!(tree,
    //     Node::builder("parent")
    //         .child(Node::builder("child a"))
    //         .child(Node::builder("child b"))
    //         .child(Node::builder("child c"))
    //         .child(Node::builder("child e"))
    //         .child(Node::builder("child d"))
    //         .child(Node::builder("other child a"))
    //         .build()
    // );
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
