use super::network::Interface;

pub struct Node;

impl Node {
    pub fn new() -> Node {
        Node
    }
    pub fn get_peers() -> Vec<Box<dyn Interface>> {
        Vec::new()
    }
}
