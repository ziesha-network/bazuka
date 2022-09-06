use crate::core::{
    Block, ContractPayment, ContractUpdate, Header, PaymentDirection, ProofOfWork, Transaction,
    TransactionData,
};
use crate::zk::{
    ZkCompressedState, ZkContract, ZkPaymentVerifierKey, ZkProof, ZkStateModel, ZkVerifierKey,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerProofOfWork {
    pub timestamp: u32,
    pub target: String,
    pub nonce: u64,
}

impl From<&ProofOfWork> for ExplorerProofOfWork {
    fn from(obj: &ProofOfWork) -> Self {
        Self {
            timestamp: obj.timestamp,
            target: obj.target.power().to_string(),
            nonce: obj.nonce,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerHeader {
    pub parent_hash: String,
    pub number: u64,
    pub block_root: String,
    pub proof_of_work: ExplorerProofOfWork,
}

impl From<&Header> for ExplorerHeader {
    fn from(obj: &Header) -> Self {
        Self {
            parent_hash: hex::encode(&obj.parent_hash),
            number: obj.number,
            block_root: hex::encode(&obj.parent_hash),
            proof_of_work: (&obj.proof_of_work).into(),
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
pub struct ExplorerPaymentVerifierKey {
    pub verifier_key: ExplorerVerifierKey,
    pub log4_payment_capacity: u8,
}

impl From<&ZkPaymentVerifierKey> for ExplorerPaymentVerifierKey {
    fn from(obj: &ZkPaymentVerifierKey) -> Self {
        Self {
            verifier_key: (&obj.verifier_key).into(),
            log4_payment_capacity: obj.log4_payment_capacity,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContract {
    pub initial_state: ExplorerCompressedState,
    pub state_model: ExplorerStateModel,
    pub payment_functions: Vec<ExplorerPaymentVerifierKey>,
    pub functions: Vec<ExplorerVerifierKey>,
}

impl From<&ZkContract> for ExplorerContract {
    fn from(obj: &ZkContract) -> Self {
        Self {
            initial_state: (&obj.initial_state).into(),
            state_model: (&obj.state_model).into(),
            payment_functions: obj.payment_functions.iter().map(|f| f.into()).collect(),
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
        Self { state: obj.clone() }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ExplorerPaymentDirection {
    Deposit,
    Withdraw,
}

impl From<&PaymentDirection> for ExplorerPaymentDirection {
    fn from(obj: &PaymentDirection) -> Self {
        match obj {
            PaymentDirection::Deposit(_) => Self::Deposit,
            PaymentDirection::Withdraw(_) => Self::Withdraw,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerContractPayment {
    pub address: String,
    pub zk_address: String,
    pub zk_address_index: u32,
    pub contract_id: String,
    pub nonce: u32,
    pub amount: u64,
    pub fee: u64,
    pub direction: ExplorerPaymentDirection,
}

impl From<&ContractPayment> for ExplorerContractPayment {
    fn from(obj: &ContractPayment) -> Self {
        Self {
            address: obj.address.to_string(),
            zk_address: obj.zk_address.to_string(),
            zk_address_index: obj.zk_address_index,
            contract_id: obj.contract_id.to_string(),
            nonce: obj.nonce,
            amount: obj.amount.into(),
            fee: obj.fee.into(),
            direction: (&obj.direction).into(),
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
pub enum ExplorerContractUpdate {
    Payment {
        circuit_id: u32,
        payments: Vec<ExplorerContractPayment>,
        next_state: ExplorerCompressedState,
        proof: ExplorerZkProof,
    },
    FunctionCall {
        function_id: u32,
        next_state: ExplorerCompressedState,
        proof: ExplorerZkProof,
        fee: u64,
    },
}

impl From<&ContractUpdate> for ExplorerContractUpdate {
    fn from(obj: &ContractUpdate) -> Self {
        match obj {
            ContractUpdate::Payment {
                circuit_id,
                payments,
                next_state,
                proof,
            } => Self::Payment {
                circuit_id: *circuit_id,
                payments: payments.iter().map(|p| p.into()).collect(),
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
pub enum ExplorerTransactionData {
    RegularSend {
        dst: String,
        amount: u64,
    },
    CreateContract {
        contract: ExplorerContract,
    },
    UpdateContract {
        contract_id: String,
        updates: Vec<ExplorerContractUpdate>,
    },
}

impl From<&TransactionData> for ExplorerTransactionData {
    fn from(obj: &TransactionData) -> Self {
        match obj {
            TransactionData::RegularSend { dst, amount } => Self::RegularSend {
                dst: dst.to_string(),
                amount: (*amount).into(),
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
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExplorerTransaction {
    pub src: String,
    pub nonce: u32,
    pub data: ExplorerTransactionData,
    pub fee: u64,
    pub sig: String,
}

impl From<&Transaction> for ExplorerTransaction {
    fn from(obj: &Transaction) -> Self {
        Self {
            src: obj.src.to_string(),
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
            body: Vec::new(),
        }
    }
}
