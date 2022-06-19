use super::*;
use crate::zk::{ZkHasher, ZkScalar};
use std::ops::*;
use std::str::FromStr;

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

struct MimcHasher;
impl ZkHasher for MimcHasher {
    fn hash(vals: &[ZkScalar]) -> ZkScalar {
        ZkScalar(zeekit::mimc::mimc(
            &vals.iter().map(|v| v.0).collect::<Vec<_>>(),
        ))
    }
}

#[test]
fn test_state_manager_scalar() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::<db::RamKvStore, SumHasher>::new(
        db::RamKvStore::new(),
        easy_config(),
    )?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    sm.new_contract(c0, zk::ZkDataType::Scalar)?;

    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![]), Some(zk::ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;

    println!("{:?}", sm.root(c0));

    Ok(())
}

#[test]
fn test_state_manager_struct() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::<db::RamKvStore, SumHasher>::new(
        db::RamKvStore::new(),
        easy_config(),
    )?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    sm.new_contract(
        c0,
        zk::ZkDataType::Struct {
            field_types: vec![zk::ZkDataType::Scalar, zk::ZkDataType::Scalar],
        },
    )?;

    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(0xf0)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf00)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0xf)))]
                .into_iter()
                .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [
                (zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(0x0))),
                (zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(0x0))),
            ]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    Ok(())
}

#[test]
fn test_state_manager_list() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::<db::RamKvStore, MimcHasher>::new(
        db::RamKvStore::new(),
        easy_config(),
    )?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    sm.new_contract(
        c0,
        zk::ZkDataType::List {
            log4_size: 3,
            item_type: Box::new(zk::ZkDataType::Struct {
                field_types: vec![zk::ZkDataType::Scalar, zk::ZkDataType::Scalar],
            }),
        },
    )?;

    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![62, 0]),
                Some(zk::ZkScalar::from(0xf00000)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![33, 1]),
                Some(zk::ZkScalar::from(0xf0)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf00)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![33, 0]),
                Some(zk::ZkScalar::from(0xf)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
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
    println!("{:?}", sm.root(c0));

    sm.update_contract(
        c0,
        zk::ZkDataPairs(
            [(
                zk::ZkDataLocator(vec![62, 0]),
                Some(zk::ZkScalar::from(0x0)),
            )]
            .into_iter()
            .collect(),
        ),
    )?;
    println!("{:?}", sm.root(c0));

    //sm.reset_contract(c0, zk::ZkDataPairs(Default::default()), Default::default())?;

    while sm.root(c0)?.height > 2 {
        sm.rollback_contract(c0)?;
        println!("{:?}", sm.root(c0));
    }

    Ok(())
}
