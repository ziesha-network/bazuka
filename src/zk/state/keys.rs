use super::*;

pub fn local_prefix(contract_id: &ContractId) -> String {
    format!("{}", contract_id)
}

pub fn local_height(contract_id: &ContractId) -> StringKey {
    format!("{}_height", local_prefix(contract_id)).into()
}

pub fn local_root(contract_id: &ContractId) -> StringKey {
    format!("{}_compressed", local_prefix(contract_id)).into()
}
