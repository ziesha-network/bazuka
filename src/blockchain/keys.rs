use super::*;

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

pub fn contract_updates(index: u64) -> StringKey {
    format!("contract_updates_{:010}", index).into()
}
