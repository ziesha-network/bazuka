use super::*;
use crate::core::{Address, ContractId, MpnAddress, TokenId, UndelegationId};
use crate::zk::ZkDataLocator;
use thiserror::Error;

pub fn height() -> StringKey {
    "HGT".into()
}

pub fn randomness() -> StringKey {
    "RND".into()
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

pub fn auto_delegate(delegator: &Address, delegatee: &Address) -> StringKey {
    format!("ADL-{}-{}", delegator, delegatee).into()
}

pub struct UndelegationDbKey {
    pub undelegation_id: UndelegationId,
    pub undelegator: Address,
}
impl Into<StringKey> for UndelegationDbKey {
    fn into(self) -> StringKey {
        format!(
            "{}-{}",
            Self::prefix(&self.undelegator),
            self.undelegation_id
        )
        .into()
    }
}
impl TryFrom<StringKey> for UndelegationDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let splitted = key.0.split("-").collect::<Vec<_>>();
        if splitted.len() != 3 {
            return Err(ParseDbKeyError::Invalid);
        }
        let undelegator = splitted[1].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let undelegation_id = splitted[2].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(UndelegationDbKey {
            undelegator,
            undelegation_id,
        })
    }
}
impl UndelegationDbKey {
    pub fn prefix(undelegator: &Address) -> String {
        format!("UDL-{}", undelegator).into()
    }
}

pub struct UndelegationCallbackDbKey {
    pub block: u64,
    pub undelegator: Address,
    pub undelegation_id: UndelegationId,
}
impl Into<StringKey> for UndelegationCallbackDbKey {
    fn into(self) -> StringKey {
        format!(
            "{}{}-{}",
            Self::prefix(self.block),
            self.undelegator,
            self.undelegation_id
        )
        .into()
    }
}
impl TryFrom<StringKey> for UndelegationCallbackDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let splitted = key.0.split("-").collect::<Vec<_>>();
        if splitted.len() != 4 {
            return Err(ParseDbKeyError::Invalid);
        }
        let block = splitted[1].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let undelegator = splitted[2].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let undelegation_id = splitted[3].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(UndelegationCallbackDbKey {
            block,
            undelegator,
            undelegation_id,
        })
    }
}
impl UndelegationCallbackDbKey {
    pub fn prefix(block: u64) -> String {
        format!("UDC-{}-", block).into()
    }
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
            (u64::MAX - Into::<u64>::into(self.amount)),
            self.address
        )
        .into()
    }
}

impl TryFrom<StringKey> for StakerRankDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let amount = Amount(
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
            (u64::MAX - Into::<u64>::into(self.amount)),
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
        let amount = Amount(
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
            (u64::MAX - Into::<u64>::into(self.amount)),
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
        let amount = Amount(
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

pub fn mpn_account_index(mpn_address: &MpnAddress, index: u64) -> StringKey {
    format!("MPN-{}-{}", mpn_address, index).into()
}

pub struct MpnAccountIndexDbKey {
    pub address: MpnAddress,
    pub index: u64,
}
impl Into<StringKey> for MpnAccountIndexDbKey {
    fn into(self) -> StringKey {
        format!("{}-{:016x}", Self::prefix(&self.address), self.index).into()
    }
}
impl TryFrom<StringKey> for MpnAccountIndexDbKey {
    type Error = ParseDbKeyError;
    fn try_from(key: StringKey) -> Result<Self, ParseDbKeyError> {
        let splitted = key.0.split("-").collect::<Vec<_>>();
        if splitted.len() != 3 {
            return Err(ParseDbKeyError::Invalid);
        }

        let address: MpnAddress = splitted[1].parse().map_err(|_| ParseDbKeyError::Invalid)?;
        let index = u64::from_str_radix(&splitted[2], 16).map_err(|_| ParseDbKeyError::Invalid)?;
        Ok(Self { address, index })
    }
}
impl MpnAccountIndexDbKey {
    pub fn prefix(addr: &MpnAddress) -> String {
        format!("MPN-{}", addr).into()
    }
}

pub fn mpn_account_count() -> StringKey {
    "MPN-CNT".into()
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
