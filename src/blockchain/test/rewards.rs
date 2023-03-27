use super::*;

const EPSILON: Amount = Amount(5);

fn close_enough(a: Amount, b: Amount) -> bool {
    if a > b {
        a - b < EPSILON
    } else {
        b - a < EPSILON
    }
}

#[test]
fn test_correct_rewards() {
    let validator = TxBuilder::new(&Vec::from("VALIDATOR"));
    let delegator = TxBuilder::new(&Vec::from("ABC"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let expected_reward_1 = chain.next_reward().unwrap();
    let expected_validator_reward_1 = Amount(expected_reward_1.0 * 12 / 255);
    assert_eq!(expected_reward_1, Amount(19999999999999));
    let draft = chain
        .draft_block(0, &[], &validator, true)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();
    assert!(close_enough(
        chain
            .get_balance(validator.get_address(), TokenId::Ziesha)
            .unwrap(),
        expected_validator_reward_1
    ));

    // Vec("DELEGATOR") Has already delegated 25Z too in genesis block
    let validator_stake = Amount(25);
    let delegator_stake = Amount(75);

    let expected_reward_2 = chain.next_reward().unwrap();
    assert_eq!(expected_reward_2, Amount(19999799999999));
    let expected_validator_reward_2 = Amount(expected_reward_2.0 * 12 / 255);
    let draft = chain
        .draft_block(
            0,
            &[
                delegator.delegate(
                    "".into(),
                    validator.get_address(),
                    delegator_stake,
                    false,
                    Money {
                        amount: 0.into(),
                        token_id: TokenId::Ziesha,
                    },
                    1,
                ),
                validator.delegate(
                    "".into(),
                    validator.get_address(),
                    validator_stake,
                    false,
                    Money {
                        amount: 0.into(),
                        token_id: TokenId::Ziesha,
                    },
                    chain.get_account(validator.get_address()).unwrap().nonce + 1,
                ),
            ],
            &validator,
            true,
        )
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();

    let expected_validator_balance_2 =
        expected_validator_reward_1 + expected_validator_reward_2 - validator_stake;
    let expected_delegator_balance_2 = Amount(10000) - delegator_stake;
    assert!(close_enough(
        chain
            .get_balance(validator.get_address(), TokenId::Ziesha)
            .unwrap(),
        expected_validator_balance_2
    ));
    assert!(close_enough(
        chain
            .get_balance(delegator.get_address(), TokenId::Ziesha)
            .unwrap(),
        expected_delegator_balance_2
    ));

    let expected_reward_3 = chain.next_reward().unwrap();
    assert_eq!(expected_reward_3, Amount(19999600001999));
    let expected_validator_reward_3 = Amount(expected_reward_3.0 * 12 / 255);
    let draft = chain
        .draft_block(0, &[], &validator, true)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();
    assert!(close_enough(
        chain
            .get_balance(validator.get_address(), TokenId::Ziesha)
            .unwrap()
            - expected_validator_balance_2,
        Amount(
            (expected_reward_3 - expected_validator_reward_3).0 / 5 + expected_validator_reward_3.0
        )
    ));
    assert!(close_enough(
        chain
            .get_balance(delegator.get_address(), TokenId::Ziesha)
            .unwrap()
            - expected_delegator_balance_2,
        Amount((expected_reward_3 - expected_validator_reward_3).0 * 3 / 5)
    ));
}
