use super::*;

pub fn pay_validator_and_delegators<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    validator: Address,
    fee_sum: Amount,
) -> Result<Amount, BlockchainError> {
    let staker = chain
        .get_staker(validator.clone())?
        .ok_or(BlockchainError::ValidatorNotRegistered)?;

    let treasury_nonce = chain.get_account(Default::default())?.nonce;

    let next_reward = chain.next_reward()? + fee_sum;
    let stakers_reward =
        u64::from(next_reward) as f64 * ((u8::MAX - staker.commision) as f64 / u8::MAX as f64); // WARN: Hardcoded!

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
    for (i, (addr, amnt)) in payments.into_iter().enumerate() {
        chain.apply_tx(
            &Transaction {
                memo: String::new(),
                src: None,
                data: TransactionData::RegularSend {
                    entries: vec![RegularSendEntry {
                        dst: addr,
                        amount: Money {
                            amount: amnt,
                            token_id: TokenId::Ziesha,
                        },
                    }],
                },
                nonce: treasury_nonce + 1 + i as u32,
                fee: Money::ziesha(0),
                sig: Signature::Unsigned,
            },
            true,
        )?;
    }
    Ok(validator_reward)
}
