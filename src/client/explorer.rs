use crate::core::{
    Address, Amount, Block, ContractDeposit, ContractUpdate, ContractUpdateData, ContractWithdraw,
    GeneralTransaction, Header, Money, MpnDeposit, MpnWithdraw, ProofOfStake, Token, Transaction,
    TransactionData,
};
use crate::crypto::jubjub::*;
use crate::zk::{
    MpnAccount, MpnTransaction, ZkCompressedState, ZkContract, ZkDataPairs, ZkDeltaPairs,
    ZkMultiInputVerifierKey, ZkProof, ZkSingleInputVerifierKey, ZkStateModel, ZkVerifierKey,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMoney {
    pub amount: u64,
    pub token_id: String,
}

impl From<Money> for ExplorerMoney {
    fn from(obj: Money) -> Self {
        Self {
            amount: obj.amount.into(),
            token_id: obj.token_id.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerDataPairs {
    pub data: HashMap<String, String>,
}

impl From<&ZkDataPairs> for ExplorerDataPairs {
    fn from(obj: &ZkDataPairs) -> Self {
        Self {
            data: obj
                .0
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerDeltaPairs {
    pub data: HashMap<String, Option<String>>,
}

impl From<&ZkDeltaPairs> for ExplorerDeltaPairs {
    fn from(obj: &ZkDeltaPairs) -> Self {
        Self {
            data: obj
                .0
                .iter()
                .map(|(k, v)| (k.to_string(), v.map(|v| v.to_string())))
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnAccount {
    pub tx_nonce: u32,
    pub withdraw_nonce: u32,
    pub address: String,
    pub tokens: HashMap<u64, ExplorerMoney>,
}

impl From<&MpnAccount> for ExplorerMpnAccount {
    fn from(obj: &MpnAccount) -> Self {
        Self {
            tx_nonce: obj.tx_nonce,
            withdraw_nonce: obj.withdraw_nonce,
            address: PublicKey(obj.address.compress()).to_string(),
            tokens: obj
                .tokens
                .iter()
                .map(|(k, money)| (*k, (*money).into()))
                .collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerToken {
    pub name: String,
    pub symbol: String,
    pub supply: u64,
    pub minter: Option<String>,
}

impl From<&Token> for ExplorerToken {
    fn from(obj: &Token) -> Self {
        Self {
            name: obj.name.clone(),
            symbol: obj.symbol.clone(),
            supply: obj.supply.into(),
            minter: obj.minter.as_ref().map(|a| a.to_string()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerProofOfStake {
    pub timestamp: u32,
    pub validator: String,
}

impl From<&ProofOfStake> for ExplorerProofOfStake {
    fn from(obj: &ProofOfStake) -> Self {
        Self {
            timestamp: obj.timestamp,
            validator: obj.validator.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerHeader {
    pub parent_hash: String,
    pub number: u64,
    pub block_root: String,
    pub proof_of_stake: ExplorerProofOfStake,
}

impl From<&Header> for ExplorerHeader {
    fn from(obj: &Header) -> Self {
        Self {
            parent_hash: hex::encode(&obj.parent_hash),
            number: obj.number,
            block_root: hex::encode(&obj.parent_hash),
            proof_of_stake: (&obj.proof_of_stake).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerStateModel {
    pub state_model: ZkStateModel,
}

impl From<&ZkStateModel> for ExplorerStateModel {
    fn from(obj: &ZkStateModel) -> Self {
        Self {
            state_model: obj.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerVerifierKey {
    pub vk: ZkVerifierKey,
}

impl From<&ZkVerifierKey> for ExplorerVerifierKey {
    fn from(obj: &ZkVerifierKey) -> Self {
        Self { vk: obj.clone() }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMultiInputVerifierKey {
    pub verifier_key: ExplorerVerifierKey,
    pub log4_payment_capacity: u8,
}

impl From<&ZkMultiInputVerifierKey> for ExplorerMultiInputVerifierKey {
    fn from(obj: &ZkMultiInputVerifierKey) -> Self {
        Self {
            verifier_key: (&obj.verifier_key).into(),
            log4_payment_capacity: obj.log4_payment_capacity,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerSingleInputVerifierKey {
    pub verifier_key: ExplorerVerifierKey,
}

impl From<&ZkSingleInputVerifierKey> for ExplorerSingleInputVerifierKey {
    fn from(obj: &ZkSingleInputVerifierKey) -> Self {
        Self {
            verifier_key: (&obj.verifier_key).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContract {
    pub initial_state: ExplorerCompressedState,
    pub state_model: ExplorerStateModel,
    pub deposit_functions: Vec<ExplorerMultiInputVerifierKey>,
    pub withdraw_functions: Vec<ExplorerMultiInputVerifierKey>,
    pub functions: Vec<ExplorerSingleInputVerifierKey>,
}

impl From<&ZkContract> for ExplorerContract {
    fn from(obj: &ZkContract) -> Self {
        Self {
            initial_state: (&obj.initial_state).into(),
            state_model: (&obj.state_model).into(),
            deposit_functions: obj.deposit_functions.iter().map(|f| f.into()).collect(),
            withdraw_functions: obj.withdraw_functions.iter().map(|f| f.into()).collect(),
            functions: obj.functions.iter().map(|f| f.into()).collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerCompressedState {
    pub state: ZkCompressedState,
}

impl From<&ZkCompressedState> for ExplorerCompressedState {
    fn from(obj: &ZkCompressedState) -> Self {
        Self { state: *obj }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContractDeposit {
    pub memo: String,
    pub contract_id: String,
    pub deposit_circuit_id: u32,
    pub src: String,
    pub amount: ExplorerMoney,
    pub fee: ExplorerMoney,

    pub nonce: u32,
    pub sig: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContractWithdraw {
    pub memo: String,
    pub contract_id: String,
    pub withdraw_circuit_id: u32,
    pub dst: String,
    pub amount: ExplorerMoney,
    pub fee: ExplorerMoney,
}

impl From<&ContractDeposit> for ExplorerContractDeposit {
    fn from(obj: &ContractDeposit) -> Self {
        Self {
            memo: obj.memo.clone(),
            src: obj.src.to_string(),
            contract_id: obj.contract_id.to_string(),
            deposit_circuit_id: obj.deposit_circuit_id,
            nonce: obj.nonce,
            amount: obj.amount.into(),
            fee: obj.fee.into(),
            sig: obj.sig.as_ref().map(|s| s.to_string()),
        }
    }
}

impl From<&ContractWithdraw> for ExplorerContractWithdraw {
    fn from(obj: &ContractWithdraw) -> Self {
        Self {
            memo: obj.memo.clone(),
            dst: obj.dst.to_string(),
            contract_id: obj.contract_id.to_string(),
            withdraw_circuit_id: obj.withdraw_circuit_id,
            amount: obj.amount.into(),
            fee: obj.fee.into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerZkProof {
    pub proof: ZkProof,
}

impl From<&ZkProof> for ExplorerZkProof {
    fn from(obj: &ZkProof) -> Self {
        Self { proof: obj.clone() }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ExplorerContractUpdateData {
    Deposit {
        deposits: Vec<ExplorerContractDeposit>,
    },
    Withdraw {
        withdraws: Vec<ExplorerContractWithdraw>,
    },
    FunctionCall {
        fee: ExplorerMoney,
    },
    Mint {
        amount: u64,
    },
}

impl From<&ContractUpdateData> for ExplorerContractUpdateData {
    fn from(obj: &ContractUpdateData) -> Self {
        match obj {
            ContractUpdateData::Deposit { deposits } => Self::Deposit {
                deposits: deposits.iter().map(|p| p.into()).collect(),
            },
            ContractUpdateData::Withdraw { withdraws } => Self::Withdraw {
                withdraws: withdraws.iter().map(|p| p.into()).collect(),
            },
            ContractUpdateData::FunctionCall { fee } => Self::FunctionCall { fee: (*fee).into() },
            ContractUpdateData::Mint { amount } => Self::Mint {
                amount: (*amount).into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContractUpdate {
    circuit_id: u32,
    data: ExplorerContractUpdateData,
    next_state: ExplorerCompressedState,
    prover: String,
    reward: u64,
    proof: ExplorerZkProof,
}

impl From<&ContractUpdate> for ExplorerContractUpdate {
    fn from(obj: &ContractUpdate) -> Self {
        Self {
            circuit_id: obj.circuit_id,
            data: (&obj.data).into(),
            next_state: (&obj.next_state).into(),
            prover: obj.prover.to_string(),
            reward: obj.reward.into(),
            proof: (&obj.proof).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ExplorerTransactionData {
    UpdateStaker {
        vrf_pub_key: String,
        commission: f64,
    },
    Delegate {
        to: String,
        amount: u64,
    },
    Undelegate {
        from: String,
        amount: u64,
    },
    AutoDelegate {
        to: String,
        ratio: f64,
    },
    RegularSend {
        entries: Vec<(String, ExplorerMoney)>,
    },
    CreateContract {
        contract: ExplorerContract,
        state: Option<ExplorerDataPairs>,
        money: ExplorerMoney,
    },
    UpdateContract {
        contract_id: String,
        updates: Vec<ExplorerContractUpdate>,
        delta: Option<ExplorerDeltaPairs>,
    },
}

impl From<&TransactionData> for ExplorerTransactionData {
    fn from(obj: &TransactionData) -> Self {
        match obj {
            TransactionData::UpdateStaker {
                vrf_pub_key,
                commission,
            } => Self::UpdateStaker {
                vrf_pub_key: hex::encode(vrf_pub_key.as_ref()),
                commission: (*commission).into(),
            },
            TransactionData::Delegate { to, amount } => Self::Delegate {
                to: to.to_string(),
                amount: (*amount).into(),
            },
            TransactionData::Undelegate { from, amount } => Self::Undelegate {
                from: from.to_string(),
                amount: (*amount).into(),
            },
            TransactionData::AutoDelegate { to, ratio } => Self::AutoDelegate {
                to: to.to_string(),
                ratio: (*ratio).into(),
            },
            TransactionData::RegularSend { entries } => Self::RegularSend {
                entries: entries
                    .iter()
                    .map(|e| (e.dst.to_string(), e.amount.into()))
                    .collect(),
            },
            TransactionData::CreateContract {
                contract,
                state,
                money,
            } => Self::CreateContract {
                contract: contract.into(),
                state: state.as_ref().map(|s| s.into()),
                money: (*money).into(),
            },
            TransactionData::UpdateContract {
                contract_id,
                updates,
                delta,
            } => Self::UpdateContract {
                contract_id: contract_id.to_string(),
                updates: updates.iter().map(|u| u.into()).collect(),
                delta: delta.as_ref().map(|d| d.into()),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerTransaction {
    pub memo: String,
    pub src: Option<String>,
    pub nonce: u32,
    pub data: ExplorerTransactionData,
    pub fee: ExplorerMoney,
    pub sig: String,
}

impl From<&Transaction> for ExplorerTransaction {
    fn from(obj: &Transaction) -> Self {
        Self {
            memo: obj.memo.clone(),
            src: obj.src.clone().map(|a| a.to_string()),
            nonce: obj.nonce,
            data: (&obj.data).into(),
            fee: obj.fee.into(),
            sig: "".into(), // TODO: Fix
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerBlock {
    pub header: ExplorerHeader,
    pub body: Vec<ExplorerTransaction>,
}

impl From<&Block> for ExplorerBlock {
    fn from(obj: &Block) -> Self {
        Self {
            header: (&obj.header).into(),
            body: obj.body.iter().map(|tx| tx.into()).collect(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerStaker {
    pub_key: String,
    stake: u64,
}

impl From<&(Address, Amount)> for ExplorerStaker {
    fn from(obj: &(Address, Amount)) -> Self {
        Self {
            pub_key: obj.0.to_string(),
            stake: obj.1.into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnDeposit {
    pub mpn_address: String,
    pub payment: ExplorerContractDeposit,
}

impl From<&MpnDeposit> for ExplorerMpnDeposit {
    fn from(obj: &MpnDeposit) -> Self {
        Self {
            mpn_address: obj.mpn_address.to_string(),
            payment: (&obj.payment).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnWithdraw {
    pub mpn_address: String,
    pub mpn_withdraw_nonce: u32,
    pub mpn_sig: String,
    pub payment: ExplorerContractWithdraw,
}

impl From<&MpnWithdraw> for ExplorerMpnWithdraw {
    fn from(obj: &MpnWithdraw) -> Self {
        Self {
            mpn_address: obj.mpn_address.to_string(),
            mpn_withdraw_nonce: obj.mpn_withdraw_nonce,
            mpn_sig: "".into(), // TODO: Convert sig to hex
            payment: (&obj.payment).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnTransaction {
    pub nonce: u32,
    pub src_pub_key: String,
    pub dst_pub_key: String,

    pub amount: ExplorerMoney,
    pub fee: ExplorerMoney,
    pub sig: String,
}

impl From<&MpnTransaction> for ExplorerMpnTransaction {
    fn from(obj: &MpnTransaction) -> Self {
        Self {
            nonce: obj.nonce,
            src_pub_key: obj.src_pub_key.to_string(),
            dst_pub_key: obj.dst_pub_key.to_string(),

            amount: obj.amount.into(),
            fee: obj.fee.into(),
            sig: "".into(), // TODO: Convert sig to hex
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ExplorerGeneralTransaction {
    TransactionAndDelta(ExplorerTransaction),
    MpnDeposit(ExplorerMpnDeposit),
    MpnTransaction(ExplorerMpnTransaction),
    MpnWithdraw(ExplorerMpnWithdraw),
}

impl From<&GeneralTransaction> for ExplorerGeneralTransaction {
    fn from(obj: &GeneralTransaction) -> Self {
        match obj {
            GeneralTransaction::MpnTransaction(mpn_tx) => Self::MpnTransaction(mpn_tx.into()),
            GeneralTransaction::MpnWithdraw(mpn_withdraw) => Self::MpnWithdraw(mpn_withdraw.into()),
            GeneralTransaction::TransactionAndDelta(tx_delta) => {
                Self::TransactionAndDelta((&tx_delta.tx).into())
            }
            GeneralTransaction::MpnDeposit(mpn_deposit) => Self::MpnDeposit(mpn_deposit.into()),
        }
    }
}
