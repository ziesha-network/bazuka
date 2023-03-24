use super::*;

pub fn create_token<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    token_id: TokenId,
    token: &Token,
) -> Result<(), BlockchainError> {
    if chain.get_token(token_id)?.is_some() {
        return Err(BlockchainError::TokenAlreadyExists);
    } else {
        if !token.validate() {
            return Err(BlockchainError::TokenBadNameSymbol);
        }
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, token_id),
            token.supply.into(),
        )])?;
        chain
            .database
            .update(&[WriteOp::Put(keys::token(&token_id), token.into())])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{RamKvStore, WriteOp};

    #[test]
    fn test_create_token() {
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
        let tkn = Token {
            name: "KeyvanCoin".into(),
            symbol: "KIWI".into(),
            supply: Amount(12345),
            decimals: 2,
            minter: None,
        };
        let (ops, _) = chain
            .isolated(|chain| Ok(create_token(chain, addr, token_id, &tkn)?))
            .unwrap();

        let expected_ops = vec![
            WriteOp::Put(
                "ACB-edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
                Amount(12345).into(),
            ),
            WriteOp::Put(
                "TKN-0x0001020304050607080900010203040506070809000102030405060708090001".into(),
                (&tkn).into(),
            ),
        ];

        assert_eq!(ops, expected_ops);
    }

    #[test]
    fn test_already_exists() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let token_id: TokenId =
            "0x0001020304050607080900010203040506070809000102030405060708090001"
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
            supply: Amount(12345),
            decimals: 2,
            minter: None,
        };
        assert!(matches!(
            chain.isolated(|chain| {
                create_token(chain, addr_1, token_id, &tkn)?;
                create_token(chain, addr_2, token_id, &tkn)?;
                Ok(())
            }),
            Err(BlockchainError::TokenAlreadyExists)
        ));
    }

    #[test]
    fn test_bad_name_symbol() {
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
        let good_token = Token {
            name: "Keyvan Coin".into(),
            symbol: "KIWI1".into(),
            supply: Amount(12345),
            decimals: 2,
            minter: None,
        };
        let bad_tokens = vec![
            Token {
                name: " KeyvanCoin".into(),
                symbol: "KIWI1".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "KeyvanCoin".into(),
                symbol: "KIW I".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "کیوان".into(),
                symbol: "KIWI".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "KeyvanCoin".into(),
                symbol: "KIWi".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "KeyvanCoin".into(),
                symbol: "1KIWI".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "KeyvanCoinAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
                symbol: "KIWI".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
            Token {
                name: "KeyvanCoin".into(),
                symbol: "KIWIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII".into(),
                supply: Amount(12345),
                decimals: 2,
                minter: None,
            },
        ];
        assert!(chain
            .isolated(|chain| {
                create_token(chain, addr.clone(), token_id, &good_token)?;
                Ok(())
            })
            .is_ok());
        for bad_token in bad_tokens {
            assert!(matches!(
                chain.isolated(|chain| {
                    create_token(chain, addr.clone(), token_id, &bad_token)?;
                    Ok(())
                }),
                Err(BlockchainError::TokenBadNameSymbol)
            ));
        }
    }
}
