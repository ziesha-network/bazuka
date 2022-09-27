use super::*;
use crate::core::{Address, ContractId};
use crate::zk::ZkDataLocator;

pub fn height() -> StringKey {
    "HGT".into()
}

pub fn outdated() -> StringKey {
    "OUT".into()
}

pub fn block(index: u64) -> StringKey {
    format!("BLK-{:010}", index).into()
}

pub fn header(index: u64) -> StringKey {
    format!("HDR-{:010}", index).into()
}

pub fn power(index: u64) -> StringKey {
    format!("POW-{:010}", index).into()
}

pub fn rollback(index: u64) -> StringKey {
    format!("RLK-{:010}", index).into()
}

pub fn merkle(index: u64) -> StringKey {
    format!("MRK-{:010}", index).into()
}

pub fn compressed_state_at(contract_id: &ContractId, at: u64) -> StringKey {
    format!("CSA-{:010}-{}", at, contract_id).into()
}

pub fn account(address: &Address) -> StringKey {
    format!("ACC-{}", address).into()
}

pub fn contract_account(contract_id: &ContractId) -> StringKey {
    format!("CAC-{}", contract_id).into()
}

pub fn contract(contract_id: &ContractId) -> StringKey {
    format!("CON-{}", contract_id).into()
}

pub fn contract_updates() -> StringKey {
    "CUP".into()
}

pub fn local_prefix(contract_id: &ContractId) -> String {
    format!("S-{}", contract_id)
}

pub fn local_height(contract_id: &ContractId) -> StringKey {
    format!("{}-HGT", local_prefix(contract_id)).into()
}

pub fn local_root(contract_id: &ContractId) -> StringKey {
    format!("{}-RT", local_prefix(contract_id)).into()
}

pub fn local_tree_aux(
    contract_id: &ContractId,
    tree_loc: &ZkDataLocator,
    aux_id: u32,
) -> StringKey {
    format!("{}-{}-T-{}", local_prefix(contract_id), tree_loc, aux_id).into()
}

pub fn local_rollback_to_height(contract_id: &ContractId, height: u64) -> StringKey {
    format!("{}-RLK-{}", local_prefix(contract_id), height).into()
}

pub fn local_scalar_value_prefix(contract_id: &ContractId) -> String {
    format!("{}-S", local_prefix(contract_id),).into()
}

pub fn local_non_scalar_value_prefix(contract_id: &ContractId) -> String {
    format!("{}", local_prefix(contract_id),).into()
}

pub fn local_value(
    contract_id: &ContractId,
    locator: &ZkDataLocator,
    is_scalar: bool,
) -> StringKey {
    format!(
        "{}-{}",
        if is_scalar {
            local_scalar_value_prefix(contract_id)
        } else {
            local_non_scalar_value_prefix(contract_id)
        },
        locator
    )
    .into()
}
