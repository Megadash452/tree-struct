use std::{
    rc::Weak,
    cell::RefCell
};
use super::*;


pub fn builders_to_node<T>(parent: WeakNode<T>, children: Vec<NodeBuilder<T>>) -> Vec<StrongNode<T>> {
    children.into_iter()
        .map(|builder|
            make_node(Some(Weak::clone(&parent)), builder)
        )
        .collect()
}

pub fn make_node<T>(parent: Option<WeakNode<T>>, builder: NodeBuilder<T>) -> StrongNode<T> {
    let root = Rc::new(RefCell::new(Node {
        content: builder.content,
        parent,
        children: vec![]
    }));

    // The children will be wrapped in RC and point to the rtrn Node as the parent
    root.borrow_mut().children = builders_to_node(Rc::downgrade(&root), builder.children);

    root
}
