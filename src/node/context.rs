use super::{PeerAddress, PeerInfo, PeerStats};
use crate::blockchain::{Blockchain, BlockchainError};
use crate::core::Transaction;
use crate::utils;
use crate::wallet::Wallet;
use rand::seq::IteratorRandom;
use rand::RngCore;
use std::collections::HashMap;

#[cfg(feature = "pow")]
use {super::api::messages::Puzzle, crate::core::Block};

#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub first_seen: u32,
}

#[cfg(feature = "pow")]
pub type BlockPuzzle = (Block, Puzzle);

#[cfg(feature = "pow")]
pub struct Miner {
    pub block_puzzle: Option<BlockPuzzle>,
    pub webhook: Option<String>,
}

pub struct NodeContext<B: Blockchain> {
    pub blockchain: B,
    pub wallet: Option<Wallet>,
    pub mempool: HashMap<Transaction, TransactionStats>,
    pub peers: HashMap<PeerAddress, PeerStats>,
    pub timestamp_offset: i32,
    #[cfg(feature = "pow")]
    pub miner: Option<Miner>,
}

impl<B: Blockchain> NodeContext<B> {
    pub fn network_timestamp(&self) -> u32 {
        (utils::local_timestamp() as i32 + self.timestamp_offset) as u32
    }
    pub fn punish(&mut self, bad_peer: PeerAddress, secs: u32) {
        self.peers
            .entry(bad_peer.clone())
            .and_modify(|stats| stats.punish(secs));
    }
    pub fn get_info(&self) -> Result<PeerInfo, BlockchainError> {
        Ok(PeerInfo {
            height: self.blockchain.get_height()?,
            #[cfg(feature = "pow")]
            power: self.blockchain.get_power()?,
        })
    }
    pub fn random_peers<R: RngCore>(
        &self,
        rng: &mut R,
        count: usize,
    ) -> HashMap<PeerAddress, PeerStats> {
        self.active_peers()
            .clone()
            .into_iter()
            .choose_multiple(rng, count)
            .into_iter()
            .collect()
    }
    pub fn most_powerful_peers(
        &self,
        count: usize,
    ) -> HashMap<PeerAddress, PeerStats> {
        let mut active_peers = self
            .active_peers()
            .clone()
            .into_iter()
            .collect::<Vec<(PeerAddress, PeerStats)>>();
        active_peers.sort_by(|(_, a), (_, b)| -> std::cmp::Ordering {
            b.info.clone()
                .unwrap_or_default()
                .power
                .cmp(&a.info.clone().unwrap_or_default().power)
        });
        active_peers.into_iter().take(count).collect()
    }
    pub fn active_peers(&self) -> HashMap<PeerAddress, PeerStats> {
        self.peers
            .iter()
            .filter_map(|(k, v)| {
                if !v.is_punished() {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(feature = "pow")]
    pub fn get_puzzle(&self, wallet: Wallet) -> Result<BlockPuzzle, BlockchainError> {
        let txs = self.mempool.keys().cloned().collect();
        let ts = self.network_timestamp();
        let block = self.blockchain.draft_block(ts, &txs, &wallet)?;
        let puzzle = Puzzle {
            key: hex::encode(self.blockchain.pow_key(block.header.number as usize)?),
            blob: hex::encode(bincode::serialize(&block.header).unwrap()),
            offset: 112,
            size: 8,
            target: block.header.proof_of_work.target,
        };
        Ok((block, puzzle))
    }
}

#[cfg(test)]
mod test_node_context {
    use crate::node::{PeerAddress, PeerInfo};
    use std::collections::HashMap;

    use crate::{blockchain::Blockchain, blockchain::BlockchainError};

    use super::NodeContext;

    struct MockBlockchain;
    impl Blockchain for MockBlockchain {
        fn get_height(&self) -> Result<usize, BlockchainError> {
            Ok(0)
        }
        fn get_power(&self) -> Result<u64, BlockchainError> {
            Ok(0)
        }

        fn get_account(
            &self,
            addr: crate::core::Address,
        ) -> Result<crate::core::Account, crate::blockchain::BlockchainError> {
            todo!()
        }

        fn will_extend(
            &self,
            from: usize,
            headers: &Vec<crate::core::Header>,
        ) -> Result<bool, crate::blockchain::BlockchainError> {
            todo!()
        }

        fn extend(
            &mut self,
            from: usize,
            blocks: &Vec<crate::core::Block>,
        ) -> Result<(), crate::blockchain::BlockchainError> {
            todo!()
        }

        fn draft_block(
            &self,
            timestamp: u32,
            mempool: &Vec<crate::core::Transaction>,
            wallet: &crate::wallet::Wallet,
        ) -> Result<crate::core::Block, crate::blockchain::BlockchainError> {
            todo!()
        }

        fn get_headers(
            &self,
            since: usize,
            until: Option<usize>,
        ) -> Result<Vec<crate::core::Header>, crate::blockchain::BlockchainError> {
            todo!()
        }

        fn get_blocks(
            &self,
            since: usize,
            until: Option<usize>,
        ) -> Result<Vec<crate::core::Block>, crate::blockchain::BlockchainError> {
            todo!()
        }

        fn pow_key(&self, index: usize) -> Result<Vec<u8>, crate::blockchain::BlockchainError> {
            todo!()
        }
    }

    #[test]
    #[cfg(feature = "pow")]
    fn most_powerful_peers_should_returns_peers_with_most_power() {
        use std::net::IpAddr;

        use crate::node::PeerStats;

        let nodes_address: IpAddr = "127.0.0.1".parse().unwrap();
        let node_context = NodeContext {
            blockchain: MockBlockchain,
            wallet: None,
            mempool: HashMap::new(),
            peers: HashMap::from([
                (
                    PeerAddress(nodes_address, 10),
                    PeerStats {
                        punished_until: 0,
                        info: Some(PeerInfo {
                            height: 0,
                            power: 1,
                        }),
                    },
                ),
                (
                    PeerAddress(nodes_address, 9),
                    PeerStats {
                        punished_until: 0,
                        info: Some(PeerInfo {
                            height: 0,
                            power: 5,
                        }),
                    },
                ),
                (
                    PeerAddress(nodes_address, 8),
                    PeerStats {
                        punished_until: 0,
                        info: Some(PeerInfo {
                            height: 0,
                            power: 2,
                        }),
                    },
                ),
                (
                    PeerAddress(nodes_address, 7),
                    PeerStats {
                        punished_until: 0,
                        info: Some(PeerInfo {
                            height: 0,
                            power: 7,
                        }),
                    },
                ),
            ]),
            timestamp_offset: 0,
            miner: None,
        };
        let most_powerful_peers = node_context.most_powerful_peers(2);
        assert_eq!(
            most_powerful_peers
                .iter()
                .map(|(_, v)| v.info.clone().unwrap().power)
                .collect::<Vec<_>>(),
            vec![7, 5]
        );
    }
}
