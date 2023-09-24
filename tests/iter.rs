use tree_struct::Node;

#[test]
fn breadth_first() {
    let tree =
        Node::builder('a')
        .child(Node::builder('b')
            .child(Node::builder('d')
                .child(Node::builder('h'))
                .child(Node::builder('i')))
            .child(Node::builder('e')
                .child(Node::builder('j'))
                .child(Node::builder('k'))))
        .child(Node::builder('c')
            .child(Node::builder('f')
                .child(Node::builder('l'))
                .child(Node::builder('m')))
            .child(Node::builder('g')
                .child(Node::builder('n'))
                .child(Node::builder('o'))))
        .build();

    assert_eq!(
        tree.iter_bfs().map(|n| n.borrow().content).collect::<Vec<_>>(),
        vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o']
    );
}

#[test]
fn depth_first() {
    let tree =
        Node::builder('a')
        .child(Node::builder('b')
            .child(Node::builder('c')
                .child(Node::builder('d'))
                .child(Node::builder('e')))
            .child(Node::builder('f')
                .child(Node::builder('g'))
                .child(Node::builder('h'))))
        .child(Node::builder('i')
            .child(Node::builder('j')
                .child(Node::builder('k'))
                .child(Node::builder('l')))
            .child(Node::builder('m')
                .child(Node::builder('n'))
                .child(Node::builder('o'))))
        .build();

    assert_eq!(
        tree.iter_dfs().map(|n| n.borrow().content).collect::<Vec<_>>(),
        vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o']
    );
}
