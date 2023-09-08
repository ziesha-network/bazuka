use super::*;
use crate::core::{ContractDeposit, ContractId, ContractWithdraw, Money, ZkHasher as CoreZkHasher};
use crate::zk::{
    StateManagerError, ZkDataLocator, ZkDeltaPairs, ZkScalar, ZkStateBuilder, ZkStateModel,
};
use bellman::groth16;
use bls12_381::Bls12;
use rand::rngs::OsRng;

fn deposits_root(
    log4_batch_size: u8,
    deposits: &[ContractDeposit],
) -> Result<ZkScalar, StateManagerError> {
    let state_model = ZkStateModel::List {
        item_type: Box::new(ZkStateModel::Struct {
            field_types: vec![
                ZkStateModel::Scalar, // Enabled
                ZkStateModel::Scalar, // Token-id
                ZkStateModel::Scalar, // Amount
                ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: log4_batch_size,
    };
    let mut state_builder = ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, deposit) in deposits.iter().enumerate() {
        state_builder.batch_set(&ZkDeltaPairs(
            [
                (ZkDataLocator(vec![i as u64, 0]), Some(ZkScalar::from(1))),
                (
                    ZkDataLocator(vec![i as u64, 1]),
                    Some(deposit.amount.token_id.into()),
                ),
                (
                    ZkDataLocator(vec![i as u64, 2]),
                    Some(ZkScalar::from(deposit.amount.amount)),
                ),
                (ZkDataLocator(vec![i as u64, 3]), Some(deposit.calldata)),
            ]
            .into(),
        ))?;
    }
    Ok(state_builder.compress()?.state_hash)
}

fn withdraws_root(
    log4_batch_size: u8,
    withdraws: &[ContractWithdraw],
) -> Result<ZkScalar, StateManagerError> {
    let state_model = ZkStateModel::List {
        item_type: Box::new(ZkStateModel::Struct {
            field_types: vec![
                ZkStateModel::Scalar, // Enabled
                ZkStateModel::Scalar, // Amount token-id
                ZkStateModel::Scalar, // Amount
                ZkStateModel::Scalar, // Fee token-id
                ZkStateModel::Scalar, // Fee
                ZkStateModel::Scalar, // Fingerprint
                ZkStateModel::Scalar, // Calldata
            ],
        }),
        log4_size: log4_batch_size,
    };
    let mut state_builder = ZkStateBuilder::<CoreZkHasher>::new(state_model);
    for (i, withdraw) in withdraws.iter().enumerate() {
        state_builder.batch_set(&ZkDeltaPairs(
            [
                (ZkDataLocator(vec![i as u64, 0]), Some(ZkScalar::from(1))),
                (
                    ZkDataLocator(vec![i as u64, 1]),
                    Some(withdraw.amount.token_id.into()),
                ),
                (
                    ZkDataLocator(vec![i as u64, 2]),
                    Some(ZkScalar::from(withdraw.amount.amount)),
                ),
                (
                    ZkDataLocator(vec![i as u64, 3]),
                    Some(withdraw.fee.token_id.into()),
                ),
                (
                    ZkDataLocator(vec![i as u64, 4]),
                    Some(ZkScalar::from(withdraw.fee.amount)),
                ),
                (
                    ZkDataLocator(vec![i as u64, 5]),
                    Some(withdraw.fingerprint()),
                ),
                (ZkDataLocator(vec![i as u64, 6]), Some(withdraw.calldata)),
            ]
            .into(),
        ))?;
    }

    Ok(state_builder.compress()?.state_hash)
}

fn update_aux(fee: Money) -> Result<ZkScalar, StateManagerError> {
    let mut state_builder = ZkStateBuilder::<CoreZkHasher>::new(ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar, // Token-id
            ZkStateModel::Scalar, // Total fee
        ],
    });
    state_builder.batch_set(&ZkDeltaPairs(
        [
            (ZkDataLocator(vec![0]), Some(fee.token_id.into())),
            (ZkDataLocator(vec![1]), Some(ZkScalar::from(fee.amount))),
        ]
        .into(),
    ))?;

    Ok(state_builder.compress()?.state_hash)
}

#[test]
fn test_empty_update_circuit() {
    let aux = update_aux(Money::ziesha(0)).unwrap();
    let circuit = UpdateCircuit {
        log4_tree_size: 3,
        log4_token_tree_size: 3,
        log4_update_batch_size: 1,

        commitment: ZkScalar::from(456),
        height: 0,
        state: ZkScalar::from(123),
        aux_data: aux,
        next_state: ZkScalar::from(123),
        fee_token: ContractId::Ziesha,

        transitions: vec![UpdateTransition::null(3, 3); 4],
    };
    let p =
        groth16::generate_random_parameters::<Bls12, _, _>(circuit.clone(), &mut OsRng).unwrap();
    let proof = groth16::create_random_proof(circuit.clone(), &p, &mut OsRng).unwrap();
    let pvk = groth16::prepare_verifying_key(&p.vk);
    assert!(groth16::verify_proof(
        &pvk,
        &proof,
        &[
            ZkScalar::from(456).into(),
            ZkScalar::from(0).into(),
            ZkScalar::from(123).into(),
            aux.into(),
            ZkScalar::from(123).into(),
        ],
    )
    .is_ok());
}

#[test]
fn test_empty_deposit_circuit() {
    let txs = vec![DepositTransition::null(3, 3); 4];
    let deps = txs
        .iter()
        .filter_map(|t| t.enabled.then(|| t.tx.payment.clone()))
        .collect::<Vec<_>>();
    let aux = deposits_root(1, &deps).unwrap();

    let circuit = DepositCircuit {
        log4_tree_size: 3,
        log4_token_tree_size: 3,
        log4_deposit_batch_size: 1,

        commitment: ZkScalar::from(456),
        height: 0,
        state: ZkScalar::from(123),
        aux_data: aux,
        next_state: ZkScalar::from(123),

        transitions: txs,
    };
    let p =
        groth16::generate_random_parameters::<Bls12, _, _>(circuit.clone(), &mut OsRng).unwrap();
    let proof = groth16::create_random_proof(circuit.clone(), &p, &mut OsRng).unwrap();
    let pvk = groth16::prepare_verifying_key(&p.vk);
    assert!(groth16::verify_proof(
        &pvk,
        &proof,
        &[
            ZkScalar::from(456).into(),
            ZkScalar::from(0).into(),
            ZkScalar::from(123).into(),
            aux.into(),
            ZkScalar::from(123).into(),
        ],
    )
    .is_ok());
}

#[test]
fn test_empty_withdraw_circuit() {
    let txs = vec![WithdrawTransition::null(3, 3); 4];
    let wtihdrawals = txs
        .iter()
        .filter_map(|t| t.enabled.then(|| t.tx.payment.clone()))
        .collect::<Vec<_>>();
    let aux = withdraws_root(1, &wtihdrawals).unwrap();

    let circuit = WithdrawCircuit {
        log4_tree_size: 3,
        log4_token_tree_size: 3,
        log4_withdraw_batch_size: 1,

        commitment: ZkScalar::from(456),
        height: 0,
        state: ZkScalar::from(123),
        aux_data: aux,
        next_state: ZkScalar::from(123),

        transitions: txs,
    };
    let p =
        groth16::generate_random_parameters::<Bls12, _, _>(circuit.clone(), &mut OsRng).unwrap();
    let proof = groth16::create_random_proof(circuit.clone(), &p, &mut OsRng).unwrap();
    let pvk = groth16::prepare_verifying_key(&p.vk);
    assert!(groth16::verify_proof(
        &pvk,
        &proof,
        &[
            ZkScalar::from(456).into(),
            ZkScalar::from(0).into(),
            ZkScalar::from(123).into(),
            aux.into(),
            ZkScalar::from(123).into(),
        ],
    )
    .is_ok());
}
