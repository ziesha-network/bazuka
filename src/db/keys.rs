use super::*;
use crate::core::{Address, ContractId, TokenId};
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

pub fn staker(address: &Address) -> StringKey {
    format!("SKR-{}", address).into()
}

pub fn stake(address: &Address) -> StringKey {
    format!("STK-{}", address).into()
}

pub fn staker_rank_prefix() -> String {
    "SRK".into()
}

pub fn staker_rank(amount: Amount, address: &Address) -> StringKey {
    format!(
        "{}-{:016x}-{}",
        staker_rank_prefix(),
        (u64::MAX - Into::<u64>::into(amount)),
        address
    )
    .into()
}

pub fn delegator_rank_prefix(delegatee: &Address) -> String {
    format!("DRK-{}", delegatee).into()
}

pub fn delegator_rank(delegatee: &Address, amount: Amount, delegator: &Address) -> StringKey {
    format!(
        "{}-{:016x}-{}",
        delegator_rank_prefix(delegatee),
        (u64::MAX - Into::<u64>::into(amount)),
        delegator
    )
    .into()
}

pub fn delegatee_rank_prefix(delegator: &Address) -> String {
    format!("DEK-{}", delegator).into()
}

pub fn delegatee_rank(delegator: &Address, amount: Amount, delegatee: &Address) -> StringKey {
    format!(
        "{}-{:016x}-{}",
        delegatee_rank_prefix(delegator),
        (u64::MAX - Into::<u64>::into(amount)),
        delegatee
    )
    .into()
}

pub fn delegate(delegator: &Address, delegatee: &Address) -> StringKey {
    format!("DEL-{}-{}", delegator, delegatee).into()
}

pub fn account_balance(address: &Address, token_id: TokenId) -> StringKey {
    format!("ACB-{}-{}", address, token_id).into()
}

pub fn contract_account(contract_id: &ContractId) -> StringKey {
    format!("CAC-{}", contract_id).into()
}

pub fn contract_balance(contract_id: &ContractId, token_id: TokenId) -> StringKey {
    format!("CAB-{}-{}", contract_id, token_id).into()
}

pub fn contract(contract_id: &ContractId) -> StringKey {
    format!("CON-{}", contract_id).into()
}

pub fn token(token_id: &TokenId) -> StringKey {
    format!("TKN-{}", token_id).into()
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
    aux_id: u64,
) -> StringKey {
    format!("{}-{}-T-{}", local_prefix(contract_id), tree_loc, aux_id).into()
}

pub fn local_rollback_to_height(contract_id: &ContractId, height: u64) -> StringKey {
    format!("{}-RLK-{}", local_prefix(contract_id), height).into()
}

pub fn local_scalar_value_prefix(contract_id: &ContractId) -> String {
    format!("{}-S", local_prefix(contract_id))
}

pub fn local_non_scalar_value_prefix(contract_id: &ContractId) -> String {
    local_prefix(contract_id)
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
