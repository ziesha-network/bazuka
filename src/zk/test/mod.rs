use super::*;
use crate::core::ContractId;
use crate::db::{KvStore, RamKvStore, WriteOp};
use std::ops::*;
use std::str::FromStr;

#[derive(Clone, Default)]
struct SumHasher;
impl ZkHasher for SumHasher {
    const MAX_ARITY: usize = 16;
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        let mut sum = ZkScalar::from(0);
        for v in vals.iter() {
            sum.add_assign(v);
        }
        sum
    }
}

fn empty_contract<H: ZkHasher>(state_model: ZkStateModel) -> ZkContract {
    ZkContract {
        token: None,
        initial_state: ZkCompressedState::empty::<H>(state_model.clone()).into(),
        state_model: state_model,
        deposit_functions: vec![],
        withdraw_functions: vec![],
        functions: vec![],
    }
}

#[test]
fn test_u64_conversion() {
    let zero: u64 = ZkScalar::from(0).try_into().unwrap();
    let num123: u64 = ZkScalar::from(123).try_into().unwrap();
    let u64max: u64 = ZkScalar::from(u64::MAX).try_into().unwrap();
    assert_eq!(zero, 0);
    assert_eq!(num123, 123);
    assert_eq!(u64max, u64::MAX);
    let u64max_plus_1: Result<u64, _> = (ZkScalar::from(u64::MAX) + ZkScalar::from(1)).try_into();
    assert!(u64max_plus_1.is_err());
}

#[test]
fn test_zk_list_membership_proof() {
    let model = ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar,
            ZkStateModel::List {
                log4_size: 4,
                item_type: Box::new(ZkStateModel::Scalar),
            },
        ],
    };
    let mut builder = ZkStateBuilder::<SumHasher>::new(model);
    for i in 0..256 {
        builder
            .batch_set(&ZkDeltaPairs(
                [(ZkDataLocator(vec![1, i]), Some(ZkScalar::from(i as u64)))].into(),
            ))
            .unwrap();
    }
    for i in 0..256 {
        let mut accum = ZkScalar::from(i as u64);
        for part in builder.prove(ZkDataLocator(vec![1]), i).unwrap() {
            for val in part.iter() {
                accum.add_assign(val);
            }
        }
        assert_eq!(accum, ZkScalar::from(32640)); // sum(0..255)
    }
}

#[test]
fn test_state_manager_scalar() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("CON-{}", c0).into(),
        empty_contract::<SumHasher>(ZkStateModel::Scalar).into(),
    )])?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    Ok(())
}

#[test]
fn test_state_manager_struct() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("CON-{}", c0).into(),
        empty_contract::<SumHasher>(ZkStateModel::Struct {
            field_types: vec![ZkStateModel::Scalar, ZkStateModel::Scalar],
        })
        .into(),
    )])?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![1]), Some(ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    KvStoreStateManager::<SumHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [
                (ZkDataLocator(vec![0]), Some(ZkScalar::from(0x0))),
                (ZkDataLocator(vec![1]), Some(ZkScalar::from(0x0))),
            ]
            .into_iter()
            .collect(),
        ),
        1,
    )?;

    Ok(())
}

#[test]
fn test_state_manager_list() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    let mut roots = Vec::new();

    db.update(&[WriteOp::Put(
        format!("CON-{}", c0).into(),
        empty_contract::<PoseidonHasher>(ZkStateModel::List {
            log4_size: 3,
            item_type: Box::new(ZkStateModel::Struct {
                field_types: vec![ZkStateModel::Scalar, ZkStateModel::Scalar],
            }),
        })
        .into(),
    )])?;

    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![62, 0]), Some(ZkScalar::from(0xf00000)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 1]), Some(ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [
                (ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0x0))),
                (ZkDataLocator(vec![33, 1]), Some(ZkScalar::from(0x0))),
            ]
            .into_iter()
            .collect(),
        ),
        1,
    )?;
    roots.push(KvStoreStateManager::<PoseidonHasher>::root(&db, c0)?);

    KvStoreStateManager::<PoseidonHasher>::update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![62, 0]), Some(ZkScalar::from(0x0)))]
                .into_iter()
                .collect(),
        ),
        1,
    )?;

    Ok(())
}
