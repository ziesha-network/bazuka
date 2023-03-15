use super::*;
use crate::core::{Address, Vrf};
use crate::crypto::VerifiableRandomFunction;

pub fn update_staker<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    vrf_pub_key: <Vrf as VerifiableRandomFunction>::Pub,
) -> Result<(), BlockchainError> {
    chain.database.update(&[WriteOp::Put(
        keys::staker(&tx_src),
        Staker { vrf_pub_key }.into(),
    )])?;
    Ok(())
}
