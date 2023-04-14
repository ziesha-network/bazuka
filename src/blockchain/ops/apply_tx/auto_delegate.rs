use super::*;
use crate::core::Ratio;

pub fn auto_delegate<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    to: Address,
    ratio: Ratio,
) -> Result<(), BlockchainError> {
    chain.database.update(&[WriteOp::Put(
        keys::auto_delegate(&tx_src, &to),
        ratio.into(),
    )])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_auto_delegate() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let src: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let dst: Address = "ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
            .parse()
            .unwrap();
        let (ops, _) = chain
            .isolated(|chain| Ok(auto_delegate(chain, src.clone(), dst.clone(), Ratio(123))?))
            .unwrap();
        chain.database.update(&ops).unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ADL-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641".into(),
                Ratio(123).into()
            ),
        ];
        assert_eq!(ops, expected_ops);
    }
}
