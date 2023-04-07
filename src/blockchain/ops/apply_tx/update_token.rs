use super::*;

pub fn update_token<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    token_id: &TokenId,
    update: &TokenUpdate,
) -> Result<(), BlockchainError> {
    if let Some(mut token) = chain.get_token(*token_id)? {
        if let Some(minter) = token.minter.clone() {
            if minter == tx_src {
                match update {
                    TokenUpdate::Mint { amount } => {
                        let mut bal = chain.get_balance(tx_src.clone(), *token_id)?;
                        if bal + *amount < bal || token.supply + *amount < token.supply {
                            return Err(BlockchainError::TokenSupplyOverflow);
                        }
                        bal += *amount;
                        token.supply += *amount;
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::token(token_id), (&token).into())])?;
                        chain.database.update(&[WriteOp::Put(
                            keys::account_balance(&tx_src, *token_id),
                            bal.into(),
                        )])?;
                    }
                    TokenUpdate::ChangeMinter { minter } => {
                        token.minter = Some(minter.clone());
                        chain
                            .database
                            .update(&[WriteOp::Put(keys::token(token_id), (&token).into())])?;
                    }
                }
            } else {
                return Err(BlockchainError::TokenUpdatePermissionDenied);
            }
        } else {
            return Err(BlockchainError::TokenNotUpdatable);
        }
    } else {
        return Err(BlockchainError::TokenNotFound);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_non_existing_token() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let token_id: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let addr: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        assert!(matches!(
            chain.isolated(|chain| Ok(update_token(
                chain,
                addr,
                &token_id,
                &TokenUpdate::Mint {
                    amount: Amount::new(100)
                }
            )?)),
            Err(BlockchainError::TokenNotFound)
        ));
    }

    #[test]
    fn test_unupdatable_token() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let token_id_1: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let token_id_2: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090002"
                .parse()
                .unwrap();
        let addr_1: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let addr_2: Address = "ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
            .parse()
            .unwrap();
        let tkn = Token {
            name: "KeyvanCoin".into(),
            symbol: "KIWI".into(),
            supply: Amount::new(12345),
            decimals: 2,
            minter: None,
        };
        super::create_token::create_token(&mut chain, addr_1.clone(), token_id_1, &tkn).unwrap();
        let tkn2 = Token {
            name: "KeyvanCoin".into(),
            symbol: "KIWI".into(),
            supply: Amount::new(12345),
            decimals: 2,
            minter: Some(addr_2),
        };
        super::create_token::create_token(&mut chain, addr_1.clone(), token_id_2, &tkn2).unwrap();
        assert!(matches!(
            chain.isolated(|chain| Ok(update_token(
                chain,
                addr_1.clone(),
                &token_id_1,
                &TokenUpdate::Mint {
                    amount: Amount::new(100)
                }
            )?)),
            Err(BlockchainError::TokenNotUpdatable)
        ));
        assert!(matches!(
            chain.isolated(|chain| Ok(update_token(
                chain,
                addr_1,
                &token_id_2,
                &TokenUpdate::Mint {
                    amount: Amount::new(100)
                }
            )?)),
            Err(BlockchainError::TokenUpdatePermissionDenied)
        ));
    }

    #[test]
    fn test_mint_token() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let token_id: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let addr: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let tkn = Token {
            name: "KeyvanCoin".into(),
            symbol: "KIWI".into(),
            supply: Amount::new(12345),
            decimals: 2,
            minter: Some(addr.clone()),
        };
        super::create_token::create_token(&mut chain, addr.clone(), token_id, &tkn).unwrap();

        let (ops, _) = chain
            .isolated(|chain| {
                Ok(update_token(
                    chain,
                    addr.clone(),
                    &token_id,
                    &TokenUpdate::Mint {
                        amount: Amount::new(100),
                    },
                )?)
            })
            .unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
                Amount::new(12445).into()
            ),
            WriteOp::Put(
                "TKN-0x0001020304050607080900010203040506070809000102030405060708090001"
                    .into(),
                (&Token {
                    name: "KeyvanCoin".into(),
                    symbol: "KIWI".into(),
                    supply: Amount::new(12445),
                    decimals: 2,
                    minter: Some(addr),
                }).into()
            ),
        ];

        assert_eq!(ops, expected_ops);
    }

    #[test]
    fn test_change_minter() {
        let mut chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let token_id: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
                .parse()
                .unwrap();
        let addr: Address = "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
            .parse()
            .unwrap();
        let new_addr: Address =
            "ed9e9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad641"
                .parse()
                .unwrap();
        let tkn = Token {
            name: "KeyvanCoin".into(),
            symbol: "KIWI".into(),
            supply: Amount::new(12345),
            decimals: 2,
            minter: Some(addr.clone()),
        };
        super::create_token::create_token(&mut chain, addr.clone(), token_id, &tkn).unwrap();

        let (ops, _) = chain
            .isolated(|chain| {
                Ok(update_token(
                    chain,
                    addr.clone(),
                    &token_id,
                    &TokenUpdate::ChangeMinter {
                        minter: new_addr.clone(),
                    },
                )?)
            })
            .unwrap();

        let expected_ops = vec![WriteOp::Put(
            "TKN-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
            (&Token {
                name: "KeyvanCoin".into(),
                symbol: "KIWI".into(),
                supply: Amount::new(12345),
                decimals: 2,
                minter: Some(new_addr),
            })
                .into(),
        )];

        assert_eq!(ops, expected_ops);
        chain.database.update(&ops).unwrap();

        assert!(matches!(
            chain.isolated(|chain| Ok(update_token(
                chain,
                addr,
                &token_id,
                &TokenUpdate::Mint {
                    amount: Amount::new(100)
                }
            )?)),
            Err(BlockchainError::TokenUpdatePermissionDenied)
        ));
    }
}
