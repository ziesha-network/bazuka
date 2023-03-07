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
mod post_mpn_transaction;
pub use post_mpn_transaction::*;
mod post_mpn_deposit;
pub use post_mpn_deposit::*;
mod post_mpn_withdraw;
pub use post_mpn_withdraw::*;
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

#[cfg(test)]
mod generate_block;
#[cfg(test)]
pub use generate_block::*;
