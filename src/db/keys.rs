use super::*;
use crate::core::{Address, ContractId, TokenId};
use crate::zk::ZkDataLocator;
use thiserror::Error;

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

pub fn nonce(address: &Address) -> StringKey {
    format!("NNC-{}", address).into()
}

pub fn deposit_nonce(address: &Address, contract_id: &ContractId) -> StringKey {
    format!("DNC-{}-{}", address, contract_id).into()
}

pub fn staker(address: &Address) -> StringKey {
    format!("SKR-{}", address).into()
}

pub fn stake(address: &Address) -> StringKey {
    format!("STK-{}", address).into()
}

#[derive(Error, Debug)]
pub enum ParseDbKeyError {
    #[error("invalid db-key")]
    Invalid,
}

pub trait DbKey: Into<StringKey> + TryFrom<StringKey> {}

pub struct StakerRankDbKey {
    pub amount: Amount,
    pub address: Address,
}
impl Into<StringKey> for StakerRankDbKey {
    fn into(self) -> StringKey {
        format!(
            "{}-{:016x}-{}",
            Self::prefix(),
            (u64::MAX - self.amount.normalize(crate::config::UNIT_ZEROS)),
            self.address
        )
        .into()
    }
}

impl TryFrom<StringKey> for StakerRankDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let amount = Amount::new(
            u64::MAX
                - u64::from_str_radix(&key.0[4..20], 16).map_err(|_| ParseDbKeyError::Invalid)?,
        );
        let address: Address = key.0[21..].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(StakerRankDbKey { amount, address })
    }
}

impl StakerRankDbKey {
    pub fn prefix() -> String {
        "SRK".into()
    }
}

pub struct DelegatorRankDbKey {
    pub delegatee: Address,
    pub amount: Amount,
    pub delegator: Address,
}
impl Into<StringKey> for DelegatorRankDbKey {
    fn into(self) -> StringKey {
        format!(
            "{}-{:016x}-{}",
            Self::prefix(&self.delegatee),
            (u64::MAX - self.amount.normalize(crate::config::UNIT_ZEROS)),
            self.delegator
        )
        .into()
    }
}
impl TryFrom<StringKey> for DelegatorRankDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let splitted = key.0.split("-").collect::<Vec<_>>();
        if splitted.len() != 4 {
            return Err(ParseDbKeyError::Invalid);
        }
        let delegatee: Address = splitted[1].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let amount = Amount::new(
            u64::MAX
                - u64::from_str_radix(&splitted[2], 16).map_err(|_| ParseDbKeyError::Invalid)?,
        );
        let delegator: Address = splitted[3].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(DelegatorRankDbKey {
            amount,
            delegator,
            delegatee,
        })
    }
}
impl DelegatorRankDbKey {
    pub fn prefix(delegatee: &Address) -> String {
        format!("DRK-{}", delegatee).into()
    }
}

pub struct DelegateeRankDbKey {
    pub delegatee: Address,
    pub amount: Amount,
    pub delegator: Address,
}
impl Into<StringKey> for DelegateeRankDbKey {
    fn into(self) -> StringKey {
        format!(
            "{}-{:016x}-{}",
            Self::prefix(&self.delegator),
            (u64::MAX - self.amount.normalize(crate::config::UNIT_ZEROS)),
            self.delegatee
        )
        .into()
    }
}
impl TryFrom<StringKey> for DelegateeRankDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let splitted = key.0.split("-").collect::<Vec<_>>();
        if splitted.len() != 4 {
            return Err(ParseDbKeyError::Invalid);
        }
        let delegator: Address = splitted[1].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let amount = Amount::new(
            u64::MAX
                - u64::from_str_radix(&splitted[2], 16).map_err(|_| ParseDbKeyError::Invalid)?,
        );
        let delegatee: Address = splitted[3].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(DelegateeRankDbKey {
            amount,
            delegator,
            delegatee,
        })
    }
}
impl DelegateeRankDbKey {
    pub fn prefix(delegator: &Address) -> String {
        format!("DEK-{}", delegator).into()
    }
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
