use super::*;
use crate::core::{Address, Ratio, Vrf};
use crate::crypto::VerifiableRandomFunction;

pub fn update_staker<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    vrf_pub_key: <Vrf as VerifiableRandomFunction>::Pub,
    commission: Ratio,
) -> Result<(), BlockchainError> {
    let commission = std::cmp::min(commission, chain.config.max_validator_commission);

    chain.database.update(&[WriteOp::Put(
        keys::staker(&tx_src),
        Staker {
            vrf_pub_key,
            commission,
        }
        .into(),
    )])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_update_staker() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();

        let abc = TxBuilder::new(&Vec::from("HAHA"));

        let (ops, _) = chain
            .isolated(|chain| {
                update_staker(
                    chain,
                    abc.get_address(),
                    abc.get_vrf_public_key(),
                    Ratio(10),
                )?;
                Ok(())
            })
            .unwrap();
        let expected_ops = vec![WriteOp::Put(
            "SKR-edf7860e4ff620f392165924386a895e9c82b156e65c16f86cb1ce0085455c4d24".into(),
            Staker {
                vrf_pub_key: abc.get_vrf_public_key(),
                commission: Ratio(10),
            }
            .into(),
        )];
        assert_eq!(ops, expected_ops);

        let (ops, _) = chain
            .isolated(|chain| {
                update_staker(
                    chain,
                    abc.get_address(),
                    abc.get_vrf_public_key(),
                    Ratio(255),
                )?;
                Ok(())
            })
            .unwrap();
        let expected_ops = vec![WriteOp::Put(
            "SKR-edf7860e4ff620f392165924386a895e9c82b156e65c16f86cb1ce0085455c4d24".into(),
            Staker {
                vrf_pub_key: abc.get_vrf_public_key(),
                commission: Ratio(26),
            }
            .into(),
        )];
        assert_eq!(ops, expected_ops);
    }
}
