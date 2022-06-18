use super::*;
use std::str::FromStr;
#[test]
fn test_state_manager_scalar() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::new(db::RamKvStore::new(), easy_config())?;

    let c0 =
        ContractId::from_str("0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

    sm.new_contract(c0, zk::ZkDataType::Scalar)?;

    println!("{:?}", sm.root(c0));

    sm.set_data(c0, vec![], zk::ZkScalar::from(123))?;

    println!("{:?}", sm.root(c0));

    Ok(())
}

#[test]
fn test_state_manager_struct() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::new(db::RamKvStore::new(), easy_config())?;

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

    sm.set_data(
        c0,
        vec![zk::ZkDataLocator::Field { field_index: 0 }],
        zk::ZkScalar::from(123),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![zk::ZkDataLocator::Field { field_index: 1 }],
        zk::ZkScalar::from(234),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![zk::ZkDataLocator::Field { field_index: 0 }],
        zk::ZkScalar::from(345),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![zk::ZkDataLocator::Field { field_index: 0 }],
        zk::ZkScalar::from(123),
    )?;
    println!("{:?}", sm.root(c0));

    Ok(())
}

#[test]
fn test_state_manager_list() -> Result<(), StateManagerError> {
    let mut sm = KvStoreStateManager::new(db::RamKvStore::new(), easy_config())?;

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

    sm.set_data(
        c0,
        vec![
            zk::ZkDataLocator::Leaf { leaf_index: 33 },
            zk::ZkDataLocator::Field { field_index: 0 },
        ],
        zk::ZkScalar::from(123),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![
            zk::ZkDataLocator::Leaf { leaf_index: 33 },
            zk::ZkDataLocator::Field { field_index: 1 },
        ],
        zk::ZkScalar::from(234),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![
            zk::ZkDataLocator::Leaf { leaf_index: 33 },
            zk::ZkDataLocator::Field { field_index: 0 },
        ],
        zk::ZkScalar::from(345),
    )?;
    println!("{:?}", sm.root(c0));

    sm.set_data(
        c0,
        vec![
            zk::ZkDataLocator::Leaf { leaf_index: 33 },
            zk::ZkDataLocator::Field { field_index: 0 },
        ],
        zk::ZkScalar::from(123),
    )?;
    println!("{:?}", sm.root(c0));

    Ok(())
}
