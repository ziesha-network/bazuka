use super::*;
use crate::core::ContractId;
use crate::db::{KvStore, RamKvStore, WriteOp};
use std::ops::*;
use std::str::FromStr;

#[derive(Clone)]
struct SumHasher;
impl ZkHasher for SumHasher {
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
        initial_state: ZkCompressedState::empty::<H>(state_model.clone()).into(),
        state_model: state_model,
        log4_deposit_withdraw_capacity: 0,
        deposit_withdraw_function: ZkVerifierKey::Dummy,
        functions: vec![],
    }
}

#[test]
fn test_state_manager_scalar() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let sm = KvStoreStateManager::<SumHasher>::new(StateManagerConfig {});

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<SumHasher>(ZkStateModel::Scalar).into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;

    println!("{:?}", sm.root(&db, c0));

    Ok(())
}

#[test]
fn test_state_manager_struct() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let sm = KvStoreStateManager::<SumHasher>::new(StateManagerConfig {});

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<SumHasher>(ZkStateModel::Struct {
            field_types: vec![ZkStateModel::Scalar, ZkStateModel::Scalar],
        })
        .into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![1]), Some(ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
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
    )?;
    println!("{:?}", sm.root(&db, c0));

    Ok(())
}

#[test]
fn test_state_manager_list() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let sm = KvStoreStateManager::<MimcHasher>::new(StateManagerConfig {});

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    let mut roots = Vec::new();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<MimcHasher>(ZkStateModel::List {
            log4_size: 3,
            item_type: Box::new(ZkStateModel::Struct {
                field_types: vec![ZkStateModel::Scalar, ZkStateModel::Scalar],
            }),
        })
        .into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![62, 0]), Some(ZkScalar::from(0xf00000)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 1]), Some(ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![33, 0]), Some(ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    println!("Full: {:?}", sm.get_full_state(&db, c0)?.data);

    sm.update_contract(
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
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &ZkDeltaPairs(
            [(ZkDataLocator(vec![62, 0]), Some(ZkScalar::from(0x0)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    //sm.reset_contract(c0, ZkDeltaPairs(Default::default()), Default::default())?;

    while sm.height_of(&db, c0)? > 2 {
        if let Some(expected_root) = roots.pop() {
            assert_eq!(Some(expected_root), sm.rollback_contract(&mut db, c0)?);
            println!("{:?} == {:?}", sm.root(&db, c0), expected_root);
        }
    }

    Ok(())
}
