use tree_struct::Node;

#[derive(Debug)]
struct A {
    name: String,
    left: B,
    right: B
}
#[tree_struct::full_node_impl]
impl Node for A {
    fn parent(&self) -> Option<&dyn Node> {
        None
    }
    fn children(&self) -> Box<[&dyn Node]> {
        Box::from([&self.left as &dyn Node, &self.right as &dyn Node])
    }
    fn debug_content(&self) -> &dyn std::fmt::Debug {
        &self.name as &dyn std::fmt::Debug
    }
}

#[derive(Debug)]
struct B {
    val: u8,
    parent: *const A
}
#[tree_struct::full_node_impl]
impl Node for B {
    fn parent(&self) -> Option<&dyn Node> {
        Some(unsafe { &*self.parent })
    }
    fn children(&self) -> Box<[&dyn Node]> {
        Box::from([])
    }
    fn debug_content(&self) -> &dyn std::fmt::Debug {
        &self.val as &dyn std::fmt::Debug
    }
}

/// This test ensures that all functions of the [`Node`] trait can be called both from a concrete type that implements the trait and using Dynamic Dispatch.
#[test]
fn node_impl() {
    let mut tree = A {
        name: String::from("hiiii"),
        left: B {
            val: 4,
            parent: std::ptr::null()
        },
        right: B {
            val: 2,
            parent: std::ptr::null()
        },
    };
    tree.left.parent = &tree;
    tree.right.parent = &tree;

    tree.parent();
    tree.children();
    tree.next_sibling();
    tree.prev_sibling();
    tree.debug_content();
    tree.debug_tree();
    tree.iter_bfs();
    tree.iter_dfs();
    tree.is_same_as(&tree);
    tree.ptr();

    let tree = &tree as &dyn Node;

    tree.parent();
    tree.children();
    tree.next_sibling();
    tree.prev_sibling();
    tree.debug_content();
    tree.debug_tree();
    tree.iter_bfs();
    tree.iter_dfs();
    tree.is_same_as(tree);
    tree.ptr();
}
