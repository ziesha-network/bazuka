use super::*;
use crate::core::Ratio;

pub fn pay_validator_and_delegators<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    validator: Address,
    fee_sum: Amount,
) -> Result<Amount, BlockchainError> {
    let height = chain.get_height()?;
    let staker = chain
        .get_staker(validator.clone())?
        .ok_or(BlockchainError::ValidatorNotRegistered)?;

    let next_reward = chain.next_reward()? + fee_sum;
    let stakers_reward =
        u64::from(next_reward) as f64 * (1.0f64 - Into::<f64>::into(staker.commission) as f64);

    let delegators = chain.get_delegators(validator.clone(), None)?;
    let total_f64 = delegators
        .iter()
        .map(|(_, a)| Into::<u64>::into(*a))
        .sum::<u64>() as f64;
    let mut payments = delegators
        .into_iter()
        .map(|(addr, stake)| {
            (
                addr,
                Amount(((u64::from(stake) as f64 / total_f64) * stakers_reward) as u64),
            )
        })
        .collect::<Vec<_>>();

    let validator_reward = next_reward
        - payments
            .iter()
            .map(|(_, a)| *a)
            .fold(Amount(0), |a, b| a + b);
    payments.push((validator.clone(), validator_reward));
    for (addr, amnt) in payments.into_iter() {
        chain.apply_tx(
            &Transaction {
                memo: String::new(),
                src: None,
                data: TransactionData::RegularSend {
                    entries: vec![RegularSendEntry {
                        dst: addr.clone(),
                        amount: Money {
                            amount: amnt,
                            token_id: ContractId::Ziesha,
                        },
                    }],
                },
                nonce: 0,
                fee: Money::ziesha(0),
                sig: Signature::Unsigned,
            },
            true,
        )?;
        let auto_delegate_ratio = chain.get_auto_delegate_ratio(addr.clone(), validator.clone())?;
        if auto_delegate_ratio > Ratio(0) {
            let auto_delegate_amount =
                Amount((amnt.0 as f64 * Into::<f64>::into(auto_delegate_ratio)) as u64);
            chain.apply_tx(
                &Transaction {
                    memo: String::new(),
                    src: Some(addr.clone()),
                    data: TransactionData::Delegate {
                        to: validator.clone(),
                        amount: auto_delegate_amount,
                    },
                    nonce: 0,
                    fee: Money::ziesha(0),
                    sig: Signature::Unsigned,
                },
                true,
            )?;
        }
    }

    for (k, _) in chain
        .database
        .pairs(keys::UndelegationCallbackDbKey::prefix(height + 1).into())?
        .into_iter()
        .collect::<Vec<_>>()
    {
        let key = keys::UndelegationCallbackDbKey::try_from(k)?;
        let undelegation = chain
            .get_undelegation(key.undelegator.clone(), key.undelegation_id)?
            .ok_or(BlockchainError::Inconsistency)?;
        let new_balance =
            chain.get_balance(key.undelegator.clone(), ContractId::Ziesha)? + undelegation.amount;
        chain.database.update(&[
            WriteOp::Remove(
                keys::UndelegationDbKey {
                    undelegator: key.undelegator.clone(),
                    undelegation_id: key.undelegation_id,
                }
                .into(),
            ),
            WriteOp::Put(
                keys::account_balance(&key.undelegator, ContractId::Ziesha),
                new_balance.into(),
            ),
        ])?;
    }

    Ok(validator_reward)
}
