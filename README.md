# Tree Struct

A general-purpose Tree implementation in Rust.

## Trees and Nodes

A Tree is essentially an `owned` Node with **content**, **children**, and no **parent**.
Most of the time, you will be dealing with `mutably and immutably borrowed` Nodes.
Create a Tree with `NodeBuilder`.

Nodes can be **mutably borrowed** with from their tree with `Tree::borrow_descendant`,
then you can change the **content** of the Node, or **append children**.
Nodes can also be **detached** from the Tree with `Tree::detach_descendant`, but that does *not* require a *mutable reference* to the Node.

## Iterators

You can iterate over all the Nodes of a Tree or a subtree (borrowed Node) using **Breadth-first** or **Depth-first Search** algorithms.
The iterators can be used to [find](https://doc.rust-lang.org/core/iter/trait.Iterator.html#method.find) a Node that you want to *detach* or *append* to another Node.

### Iterators for mutable Nodes

Mutable iterators (`Iterator<Item = &mut Node>`) are unsafe due to the fact that they yield mutable references to every Node.
A child of the yielded Nodes can then be `immutably borrowed` with `node.children()`, but the same child *will* be yielded in a future iteration.
Now `mutable and shared` references to the same Node exist simultaneusly, which is **unsafe**.

A better (and *safe*) alternative to *mutable iterators* is using the *immutable iterators* (`IterBFS` and `IterDFS`) and **mutably borrowing** a descendant from the Tree.