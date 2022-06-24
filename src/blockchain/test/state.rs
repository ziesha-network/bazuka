use super::*;
use crate::db::RamKvStore;
use crate::zk::{MimcHasher, ZkHasher, ZkScalar};
use std::ops::*;
use std::str::FromStr;

#[derive(Clone)]
struct SumHasher;
impl ZkHasher for SumHasher {
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        let mut sum = ZkScalar::from(0);
        for v in vals.iter() {
            sum.0.add_assign(&v.0);
        }
        sum
    }
}

fn empty_contract<H: ZkHasher>(state_model: zk::ZkStateModel) -> zk::ZkContract {
    zk::ZkContract {
        initial_state: zk::ZkCompressedState::empty::<H>(state_model.clone()).into(),
        state_model: state_model,
        deposit_withdraw_function: zk::ZkVerifierKey::Dummy,
        functions: vec![],
    }
}

#[test]
fn test_state_manager_scalar() -> Result<(), StateManagerError> {
    let mut db = RamKvStore::new();

    let mut sm = KvStoreStateManager::<SumHasher>::new(StateManagerConfig {})?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<SumHasher>(zk::ZkStateModel::Scalar).into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![]), Some(zk::ZkScalar::from(0xf)))]
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

    let mut sm = KvStoreStateManager::<SumHasher>::new(StateManagerConfig {})?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<SumHasher>(zk::ZkStateModel::Struct {
            field_types: vec![zk::ZkStateModel::Scalar, zk::ZkStateModel::Scalar],
        })
        .into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [
                (zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0x0))),
                (zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(0x0))),
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

    let mut sm = KvStoreStateManager::<MimcHasher>::new(StateManagerConfig {})?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    let mut roots = Vec::new();

    db.update(&[WriteOp::Put(
        format!("contract_{}", c0).into(),
        empty_contract::<MimcHasher>(zk::ZkStateModel::List {
            log4_size: 3,
            item_type: Box::new(zk::ZkStateModel::Struct {
                field_types: vec![zk::ZkStateModel::Scalar, zk::ZkStateModel::Scalar],
            }),
        })
        .into(),
    )])?;

    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![62, 0]),
                Some(zk::ZkScalar::from(0xf00000)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![33, 1]),
                Some(zk::ZkScalar::from(0xf0)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf00)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));
    roots.push(sm.root(&db, c0)?);

    sm.update_contract(
        &mut db,
        c0,
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf)),
            )]
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
        &zk::ZkDeltaPairs(
            [
                (
                    zk::ZkDataLocator(vec![33, 0]),
                    Some(zk::ZkScalar::from(0x0)),
                ),
                (
                    zk::ZkDataLocator(vec![33, 1]),
                    Some(zk::ZkScalar::from(0x0)),
                ),
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
        &zk::ZkDeltaPairs(
            [(
                zk::ZkDataLocator(vec![62, 0]),
                Some(zk::ZkScalar::from(0x0)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(&db, c0));

    //sm.reset_contract(c0, zk::ZkDeltaPairs(Default::default()), Default::default())?;

    while sm.root(&db, c0)?.height > 2 {
        if let Some(expected_root) = roots.pop() {
            assert_eq!(Some(expected_root), sm.rollback_contract(&mut db, c0)?);
            println!("{:?} == {:?}", sm.root(&db, c0), expected_root);
        }
    }

    Ok(())
}
