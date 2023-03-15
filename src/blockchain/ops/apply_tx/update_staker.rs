use super::*;
use crate::core::{Address, Vrf};
use crate::crypto::VerifiableRandomFunction;

pub fn update_staker<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    vrf_pub_key: <Vrf as VerifiableRandomFunction>::Pub,
    commision: u8,
) -> Result<(), BlockchainError> {
    if commision > chain.config.max_validator_commision {
        return Err(BlockchainError::ExcessiveValidatorCommision);
    }

    chain.database.update(&[WriteOp::Put(
        keys::staker(&tx_src),
        Staker {
            vrf_pub_key,
            commision,
        }
        .into(),
    )])?;
    Ok(())
}
