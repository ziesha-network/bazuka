use super::*;
use crate::core::{Address, ContractId};
use crate::zk::ZkDataLocator;

pub fn height() -> StringKey {
    "height".into()
}

pub fn outdated() -> StringKey {
    "outdated".into()
}

pub fn block(index: u64) -> StringKey {
    format!("block_{:010}", index).into()
}

pub fn header(index: u64) -> StringKey {
    format!("header_{:010}", index).into()
}

pub fn power(index: u64) -> StringKey {
    format!("power_{:010}", index).into()
}

pub fn rollback(index: u64) -> StringKey {
    format!("rollback_{:010}", index).into()
}

pub fn merkle(index: u64) -> StringKey {
    format!("merkle_{:010}", index).into()
}

pub fn compressed_state_at(contract_id: &ContractId, at: u64) -> StringKey {
    format!("contract_compressed_state_{}_{}", contract_id, at).into()
}

pub fn account(address: &Address) -> StringKey {
    format!("account_{}", address).into()
}

pub fn contract_account(contract_id: &ContractId) -> StringKey {
    format!("contract_account_{}", contract_id).into()
}

pub fn contract(contract_id: &ContractId) -> StringKey {
    format!("contract_{}", contract_id).into()
}

pub fn contract_updates() -> StringKey {
    "contract_updates".into()
}

pub fn local_prefix(contract_id: &ContractId) -> String {
    format!("{}", contract_id)
}

pub fn local_height(contract_id: &ContractId) -> StringKey {
    format!("{}_height", local_prefix(contract_id)).into()
}

pub fn local_root(contract_id: &ContractId) -> StringKey {
    format!("{}_compressed", local_prefix(contract_id)).into()
}

pub fn local_tree_aux(
    contract_id: &ContractId,
    tree_loc: &ZkDataLocator,
    aux_id: u32,
) -> StringKey {
    format!("{}_{}_aux_{}", local_prefix(contract_id), tree_loc, aux_id).into()
}

pub fn local_rollback_to_height(contract_id: &ContractId, height: u64) -> StringKey {
    format!("{}_rollback_{}", local_prefix(contract_id), height).into()
}

pub fn local_scalar_value_prefix(contract_id: &ContractId) -> String {
    format!("{}_s", local_prefix(contract_id),).into()
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
        "{}_{}",
        if is_scalar {
            local_scalar_value_prefix(contract_id)
        } else {
            local_non_scalar_value_prefix(contract_id)
        },
        locator
    )
    .into()
}
