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
                    Money {
                        amount: 0.into(),
                        token_id: TokenId::Ziesha,
                    },
                    chain.get_nonce(validator.get_address()).unwrap() + 1,
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

#[test]
fn test_auto_delegate() {
    let validator = TxBuilder::new(&Vec::from("VALIDATOR"));
    let delegator = TxBuilder::new(&Vec::from("DELEGATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    chain
        .apply_tx(
            &Transaction {
                memo: "".into(),
                src: Some(delegator.get_address()),
                data: TransactionData::AutoDelegate {
                    to: validator.get_address(),
                    ratio: Ratio(55),
                },
                nonce: 0,
                fee: Money::ziesha(0),
                sig: Signature::Unsigned,
            },
            true,
        )
        .unwrap();

    let expected_reward = chain.next_reward().unwrap();
    let expected_validator_reward = Amount(expected_reward.0 * 12 / 255);
    let expected_delegator_reward_before_auto_delegate =
        expected_reward - expected_validator_reward;
    let expected_delegator_reward_after_auto_delegate =
        Amount((expected_delegator_reward_before_auto_delegate.0 as f64 * 200.0 / 255.0) as u64);
    let expected_delegator_balance = Amount(25) + expected_delegator_reward_after_auto_delegate;
    let expected_delegation = Amount(25)
        + Amount((expected_delegator_reward_before_auto_delegate.0 as f64 * 55.0 / 255.0) as u64);
    assert_eq!(
        chain
            .get_balance(delegator.get_address(), TokenId::Ziesha)
            .unwrap(),
        Amount(25)
    );
    assert_eq!(expected_reward, Amount(19999999999999));
    let draft = chain
        .draft_block(0, &[], &validator, true)
        .unwrap()
        .unwrap();
    chain.apply_block(&draft.block).unwrap();
    assert!(close_enough(
        chain
            .get_delegate(delegator.get_address(), validator.get_address())
            .unwrap()
            .amount,
        expected_delegation
    ));
    assert!(close_enough(
        chain
            .get_balance(validator.get_address(), TokenId::Ziesha)
            .unwrap(),
        expected_validator_reward
    ));
    assert!(close_enough(
        chain
            .get_balance(delegator.get_address(), TokenId::Ziesha)
            .unwrap(),
        expected_delegator_balance
    ));
}
