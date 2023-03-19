use crate::core::{
    Address, Amount, Block, ChainSourcedTx, ContractDeposit, ContractUpdate, ContractWithdraw,
    Header, Money, MpnDeposit, MpnSourcedTx, MpnWithdraw, ProofOfStake, Token, TokenUpdate,
    Transaction, TransactionData,
};
use crate::crypto::jubjub::*;
use crate::zk::{
    MpnAccount, MpnTransaction, ZkCompressedState, ZkContract, ZkMultiInputVerifierKey, ZkProof,
    ZkSingleInputVerifierKey, ZkStateModel, ZkVerifierKey,
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
pub struct ExplorerMpnAccount {
    pub nonce: u64,
    pub address: String,
    pub tokens: HashMap<u64, ExplorerMoney>,
}

impl From<&MpnAccount> for ExplorerMpnAccount {
    fn from(obj: &MpnAccount) -> Self {
        Self {
            nonce: obj.nonce,
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
pub enum ExplorerTokenUpdate {
    Mint { amount: u64 },
    ChangeMinter { minter: String },
}

impl From<&TokenUpdate> for ExplorerTokenUpdate {
    fn from(obj: &TokenUpdate) -> Self {
        match obj {
            TokenUpdate::Mint { amount } => Self::Mint {
                amount: (*amount).into(),
            },
            TokenUpdate::ChangeMinter { minter } => Self::ChangeMinter {
                minter: minter.to_string(),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ExplorerContractUpdate {
    Deposit {
        deposit_circuit_id: u32,
        deposits: Vec<ExplorerContractDeposit>,
        next_state: ExplorerCompressedState,
        proof: ExplorerZkProof,
    },
    Withdraw {
        withdraw_circuit_id: u32,
        withdraws: Vec<ExplorerContractWithdraw>,
        next_state: ExplorerCompressedState,
        proof: ExplorerZkProof,
    },
    FunctionCall {
        function_id: u32,
        next_state: ExplorerCompressedState,
        proof: ExplorerZkProof,
        fee: ExplorerMoney,
    },
}

impl From<&ContractUpdate> for ExplorerContractUpdate {
    fn from(obj: &ContractUpdate) -> Self {
        match obj {
            ContractUpdate::Deposit {
                deposit_circuit_id,
                deposits,
                next_state,
                proof,
            } => Self::Deposit {
                deposit_circuit_id: *deposit_circuit_id,
                deposits: deposits.iter().map(|p| p.into()).collect(),
                next_state: next_state.into(),
                proof: proof.into(),
            },
            ContractUpdate::Withdraw {
                withdraw_circuit_id,
                withdraws,
                next_state,
                proof,
            } => Self::Withdraw {
                withdraw_circuit_id: *withdraw_circuit_id,
                withdraws: withdraws.iter().map(|p| p.into()).collect(),
                next_state: next_state.into(),
                proof: proof.into(),
            },
            ContractUpdate::FunctionCall {
                function_id,
                next_state,
                proof,
                fee,
            } => Self::FunctionCall {
                function_id: *function_id,
                fee: (*fee).into(),
                next_state: next_state.into(),
                proof: proof.into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ExplorerTransactionData {
    UpdateStaker {
        vrf_pub_key: String,
        commision: u8,
    },
    Delegate {
        to: String,
        amount: u64,
        reverse: bool,
    },
    RegularSend {
        entries: Vec<(String, ExplorerMoney)>,
    },
    CreateContract {
        contract: ExplorerContract,
    },
    UpdateContract {
        contract_id: String,
        updates: Vec<ExplorerContractUpdate>,
    },
    CreateToken {
        token: ExplorerToken,
    },
    UpdateToken {
        token_id: String,
        update: ExplorerTokenUpdate,
    },
}

impl From<&TransactionData> for ExplorerTransactionData {
    fn from(obj: &TransactionData) -> Self {
        match obj {
            TransactionData::UpdateStaker {
                vrf_pub_key,
                commision,
            } => Self::UpdateStaker {
                vrf_pub_key: hex::encode(vrf_pub_key.as_ref()),
                commision: *commision,
            },
            TransactionData::Delegate {
                to,
                amount,
                reverse,
            } => Self::Delegate {
                to: to.to_string(),
                amount: (*amount).into(),
                reverse: *reverse,
            },
            TransactionData::RegularSend { entries } => Self::RegularSend {
                entries: entries
                    .iter()
                    .map(|e| (e.dst.to_string(), e.amount.into()))
                    .collect(),
            },
            TransactionData::CreateContract { contract } => Self::CreateContract {
                contract: contract.into(),
            },
            TransactionData::UpdateContract {
                contract_id,
                updates,
            } => Self::UpdateContract {
                contract_id: contract_id.to_string(),
                updates: updates.iter().map(|u| u.into()).collect(),
            },
            TransactionData::CreateToken { token } => Self::CreateToken {
                token: token.into(),
            },
            TransactionData::UpdateToken { token_id, update } => Self::UpdateToken {
                token_id: token_id.to_string(),
                update: update.into(),
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
    pub zk_address: String,
    pub zk_token_index: u64,
    pub payment: ExplorerContractDeposit,
}

impl From<&MpnDeposit> for ExplorerMpnDeposit {
    fn from(obj: &MpnDeposit) -> Self {
        Self {
            zk_address: obj.zk_address.to_string(),
            zk_token_index: obj.zk_token_index,
            payment: (&obj.payment).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnWithdraw {
    pub zk_address: String,
    pub zk_token_index: u64,
    pub zk_fee_token_index: u64,
    pub zk_nonce: u64,
    pub zk_sig: String,
    pub payment: ExplorerContractWithdraw,
}

impl From<&MpnWithdraw> for ExplorerMpnWithdraw {
    fn from(obj: &MpnWithdraw) -> Self {
        Self {
            zk_address: obj.zk_address.to_string(),
            zk_token_index: obj.zk_token_index,
            zk_fee_token_index: obj.zk_fee_token_index,
            zk_nonce: obj.zk_nonce,
            zk_sig: "".into(), // TODO: Convert sig to hex
            payment: (&obj.payment).into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerMpnTransaction {
    pub nonce: u64,
    pub src_pub_key: String,
    pub dst_pub_key: String,

    pub src_token_index: u64,
    pub src_fee_token_index: u64,
    pub dst_token_index: u64,

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

            src_token_index: obj.src_token_index,
            src_fee_token_index: obj.src_fee_token_index,
            dst_token_index: obj.dst_token_index,

            amount: obj.amount.into(),
            fee: obj.fee.into(),
            sig: "".into(), // TODO: Convert sig to hex
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ExplorerChainSourcedTx {
    TransactionAndDelta(ExplorerTransaction),
    MpnDeposit(ExplorerMpnDeposit),
}

impl From<&ChainSourcedTx> for ExplorerChainSourcedTx {
    fn from(obj: &ChainSourcedTx) -> Self {
        match obj {
            ChainSourcedTx::TransactionAndDelta(tx_delta) => {
                Self::TransactionAndDelta((&tx_delta.tx).into())
            }
            ChainSourcedTx::MpnDeposit(mpn_deposit) => Self::MpnDeposit(mpn_deposit.into()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ExplorerMpnSourcedTx {
    MpnTransaction(ExplorerMpnTransaction),
    MpnWithdraw(ExplorerMpnWithdraw),
}

impl From<&MpnSourcedTx> for ExplorerMpnSourcedTx {
    fn from(obj: &MpnSourcedTx) -> Self {
        match obj {
            MpnSourcedTx::MpnTransaction(mpn_tx) => Self::MpnTransaction(mpn_tx.into()),
            MpnSourcedTx::MpnWithdraw(mpn_withdraw) => Self::MpnWithdraw(mpn_withdraw.into()),
        }
    }
}
