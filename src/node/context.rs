use super::{PeerAddress, PeerInfo};
use crate::blockchain::Blockchain;
use std::collections::HashMap;

pub struct NodeContext<B: Blockchain> {
    pub blockchain: B,
    pub peers: HashMap<PeerAddress, PeerInfo>,
}
