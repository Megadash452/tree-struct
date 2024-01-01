# Tree Struct (Reference Counted)

A general-purpose Tree implementation in Rust.

## Trees and Nodes

A Tree is essentially an `owned` Node, which owns its **content** and its **children**.
Most of the time, you will be dealing with `Reference Counted` Nodes, which can be `mutably and immutably borrowed`.
Create a Tree with the `NodeBuilder`, using the *Builder Pattern* or the struct itself..

A Node can be `detached` from its parent (and Tree), giving an *explicitly owned* Tree, which can then be `appended` to another Node.

## Iterators

You can iterate over all the Nodes of a Tree or a subtree (Node) using **Breadth-first** or **Depth-first Search** algorithms.
The iterators can be used to [find](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.find) a Node that you want to *detach* or *append* to another Node.
