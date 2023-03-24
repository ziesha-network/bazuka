use super::*;

pub fn delegate<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    amount: Amount,
    to: Address,
    reverse: bool,
) -> Result<(), BlockchainError> {
    let mut src_bal = chain.get_balance(tx_src.clone(), TokenId::Ziesha)?;
    if !reverse {
        if src_bal < amount {
            return Err(BlockchainError::BalanceInsufficient);
        }
        src_bal -= amount;
    } else {
        src_bal += amount;
    }
    chain.database.update(&[WriteOp::Put(
        keys::account_balance(&tx_src, TokenId::Ziesha),
        src_bal.into(),
    )])?;

    let mut delegate = chain.get_delegate(tx_src.clone(), to.clone())?;
    let old_delegate = delegate.amount;
    if !reverse {
        delegate.amount += amount;
    } else {
        if delegate.amount < amount {
            return Err(BlockchainError::BalanceInsufficient);
        }
        delegate.amount -= amount;
    }
    chain.database.update(&[WriteOp::Put(
        keys::delegate(&tx_src, &to),
        delegate.clone().into(),
    )])?;

    let old_stake = chain.get_stake(to.clone())?;
    let new_stake = old_stake + amount;
    chain.database.update(&[
        WriteOp::Remove(keys::delegatee_rank(&tx_src, old_delegate, &to)),
        WriteOp::Put(
            keys::delegatee_rank(&tx_src, delegate.amount, &to),
            ().into(),
        ),
        WriteOp::Remove(keys::delegator_rank(&to, old_delegate, &tx_src)),
        WriteOp::Put(
            keys::delegator_rank(&to, delegate.amount, &tx_src),
            ().into(),
        ),
        WriteOp::Remove(keys::staker_rank(old_stake, &to)),
        WriteOp::Put(keys::staker_rank(new_stake, &to), ().into()),
        WriteOp::Put(keys::stake(&to), new_stake.into()),
    ])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_delegate() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let src: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let abc = TxBuilder::new(&Vec::from("ABC")).get_address();
        let dst: Address = "ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
            .parse()
            .unwrap();
        let (ops, _) = chain
            .isolated(|chain| {
                Ok(delegate(
                    chain,
                    abc.clone(),
                    Amount(123),
                    dst.clone(),
                    false,
                )?)
            })
            .unwrap();
        chain.database.update(&ops).unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha".into(),
                Amount(9877).into()
            ),
            WriteOp::Put(
                "DEK-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ffffffffffffff84-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "DEK-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ffffffffffffffff-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into()
            ),
            WriteOp::Put(
                "DEL-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                Delegate {
                    amount: Amount(123)
                }.into()
            ),
            WriteOp::Put(
                "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffff84-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffffff-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a"
                    .into()
            ),
            WriteOp::Put(
                "SRK-ffffffffffffff84-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "SRK-ffffffffffffffff-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into()
            ),
            WriteOp::Put(
                "STK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                Amount(123).into()
            ),
        ];
        assert_eq!(ops, expected_ops);

        let (ops, _) = chain
            .isolated(|chain| {
                Ok(delegate(
                    chain,
                    abc.clone(),
                    Amount(77),
                    dst.clone(),
                    false,
                )?)
            })
            .unwrap();
        chain.database.update(&ops).unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-Ziesha".into(),
                Amount(9800).into()
            ),
            WriteOp::Put(
                "DEK-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ffffffffffffff37-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "DEK-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ffffffffffffff84-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into()
            ),
            WriteOp::Put(
                "DEL-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                Delegate {
                    amount: Amount(200)
                }.into()
            ),
            WriteOp::Put(
                "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffff37-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffff84-ed8c19c6a4cf1460e961f7bae8eea54d437b9edac27cbeb09be32ae367adf9098a"
                    .into()
            ),
            WriteOp::Put(
                "SRK-ffffffffffffff37-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                ().into()
            ),
            WriteOp::Remove(
                "SRK-ffffffffffffff84-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into()
            ),
            WriteOp::Put(
                "STK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                    .into(),
                Amount(200).into()
            ),
        ];
        assert_eq!(ops, expected_ops);

        regular_send::regular_send(
            &mut chain,
            abc.clone(),
            &[RegularSendEntry {
                dst: src.clone(),
                amount: Money {
                    token_id: TokenId::Ziesha,
                    amount: Amount(100),
                },
            }],
        )
        .unwrap();

        let (ops, _) = chain
            .isolated(|chain| {
                Ok(delegate(
                    chain,
                    src.clone(),
                    Amount(60),
                    dst.clone(),
                    false,
                )?)
            })
            .unwrap();

        let expected_ops = vec![
                WriteOp::Put(
                    "ACB-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-Ziesha".into(),
                    Amount(40).into()
                ),
                WriteOp::Put(
                    "DEK-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-ffffffffffffffc3-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into(),
                    ().into()
                ),
                WriteOp::Remove(
                    "DEK-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-ffffffffffffffff-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into()
                ),
                WriteOp::Put(
                    "DEL-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into(),
                    Delegate {
                        amount: Amount(60)
                    }.into()
                ),
                WriteOp::Put(
                    "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffffc3-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
                        .into(),
                    ().into()
                ),
                WriteOp::Remove(
                    "DRK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641-ffffffffffffffff-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
                        .into()
                ),
                WriteOp::Put(
                    "SRK-fffffffffffffefb-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into(),
                    ().into()
                ),
                WriteOp::Remove(
                    "SRK-ffffffffffffff37-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into()
                ),
                WriteOp::Put(
                    "STK-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                        .into(),
                    Amount(260).into()
                ),
            ];
        assert_eq!(ops, expected_ops);
    }

    #[test]
    fn test_delegate_balance_insufficient() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let abc = TxBuilder::new(&Vec::from("ABC")).get_address();
        let src: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let dst: Address = "ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
            .parse()
            .unwrap();
        assert!(matches!(
            chain.isolated(|chain| {
                delegate(chain, src.clone(), Amount(123), dst.clone(), false)?;
                Ok(())
            }),
            Err(BlockchainError::BalanceInsufficient)
        ));
    }
}
