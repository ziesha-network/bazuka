use super::{promote_block, promote_validator_claim, NodeContext, NodeError};

use crate::client::messages;

mod get_stats;
pub use get_stats::*;
mod get_peers;
pub use get_peers::*;
mod post_peer;
pub use post_peer::*;
mod post_block;
pub use post_block::*;
mod get_blocks;
pub use get_blocks::*;
mod get_explorer_blocks;
pub use get_explorer_blocks::*;
mod get_states;
pub use get_states::*;
mod get_outdated_heights;
pub use get_outdated_heights::*;
mod get_headers;
pub use get_headers::*;
mod transact;
pub use transact::*;
mod shutdown;
pub use shutdown::*;
mod get_account;
pub use get_account::*;
mod get_mpn_account;
pub use get_mpn_account::*;
mod get_explorer_mpn_accounts;
pub use get_explorer_mpn_accounts::*;
mod get_mempool;
pub use get_mempool::*;
mod get_debug_data;
pub use get_debug_data::*;
mod get_balance;
pub use get_balance::*;
mod get_token;
pub use get_token::*;
mod post_validator_claim;
pub use post_validator_claim::*;
mod get_explorer_stakers;
pub use get_explorer_stakers::*;
mod get_mpn_work;
pub use get_mpn_work::*;
mod post_mpn_solution;
pub use post_mpn_solution::*;
mod get_delegations;
pub use get_delegations::*;
mod post_mpn_worker;
pub use post_mpn_worker::*;
mod get_explorer_mempool;
pub use get_explorer_mempool::*;
mod get_check_tx;
pub use get_check_tx::*;
#[cfg(test)]
mod generate_block;
#[cfg(test)]
pub use generate_block::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Blockchain;
    use crate::blockchain::KvStoreChain;
    use crate::client::messages::SocialProfiles;
    use crate::client::NodeRequest;
    use crate::client::OutgoingSender;
    use crate::core::Amount;
    use crate::db::RamKvStore;
    use crate::node::local_timestamp;
    use crate::node::Mempool;
    use crate::node::PeerManager;
    use crate::node::TxBuilder;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::RwLock;

    pub fn test_context() -> Arc<RwLock<NodeContext<RamKvStore, KvStoreChain<RamKvStore>>>> {
        let network: String = "test".into();
        const NUM_BLOCKS: usize = 100;
        let opts = crate::config::node::get_simulator_options();
        let (out_send, _) = mpsc::unbounded_channel::<NodeRequest>();
        let validator_wallet = TxBuilder::new(&Vec::from("VALIDATOR"));
        let user_wallet = TxBuilder::new(&Vec::from("ABC"));
        let mut blockchain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        for i in 0..NUM_BLOCKS {
            let block = blockchain
                .draft_block((i * 60 + 30) as u32, &[], &validator_wallet, true)
                .unwrap()
                .unwrap();
            blockchain.extend((i + 1) as u64, &[block.block]).unwrap();
        }
        Arc::new(RwLock::new(NodeContext {
            _phantom: std::marker::PhantomData,
            firewall: None,
            opts: opts.clone(),
            network: network.clone(),
            social_profiles: SocialProfiles { discord: None },
            address: None,
            shutdown: false,
            outgoing: Arc::new(OutgoingSender {
                network: network.clone(),
                chan: out_send,
                priv_key: user_wallet.get_priv_key(),
            }),
            mpn_workers: Default::default(),
            mpn_work_pool: None,
            mempool: Mempool::new(Amount(1_000_000_000)),
            blockchain,
            validator_wallet: validator_wallet.clone(),
            user_wallet: user_wallet.clone(),
            peer_manager: PeerManager::new(
                None,
                Default::default(),
                local_timestamp(),
                opts.candidate_remove_threshold,
            ),
            timestamp_offset: 0,
            banned_headers: Default::default(),
            outdated_since: None,
            validator_claim: None,
        }))
    }
}
