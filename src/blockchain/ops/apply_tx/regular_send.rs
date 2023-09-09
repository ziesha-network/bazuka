use super::*;

use crate::core::ZkHasher as CoreZkHasher;
use crate::zk::ZkHasher;
use ff::PrimeField;
use num_bigint::BigUint;

fn put_in_teleport_tree<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    dst: Address,
    money: Money,
) -> Result<(), BlockchainError> {
    let as_scalar =
        ZkScalar::from_str_vartime(&BigUint::from_bytes_le(&dst.to_bytes()[..31]).to_string());
    let commitment = CoreZkHasher::hash(&[money.token_id.into(), money.amount.into()]);
    Ok(())
}

pub fn regular_send<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    entries: &[RegularSendEntry],
) -> Result<(), BlockchainError> {
    for entry in entries {
        if entry.dst != tx_src {
            let mut src_bal = chain.get_balance(tx_src.clone(), entry.amount.token_id)?;

            if src_bal < entry.amount.amount {
                return Err(BlockchainError::BalanceInsufficient);
            }
            src_bal -= entry.amount.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&tx_src, entry.amount.token_id),
                src_bal.into(),
            )])?;

            let mut dst_bal = chain.get_balance(entry.dst.clone(), entry.amount.token_id)?;
            dst_bal += entry.amount.amount;

            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&entry.dst, entry.amount.token_id),
                dst_bal.into(),
            )])?;

            put_in_teleport_tree(chain, entry.dst.clone(), entry.amount.clone())?;
        }
    }
    Ok(())
}
