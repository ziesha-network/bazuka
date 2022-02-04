use std::marker::PhantomData;

pub struct Node<I>(PhantomData<I>);

impl<I> Node<I> {
    pub fn new() -> Node<I> {
        Node::<I>(PhantomData)
    }
    pub fn get_peers() {

    }
}
