use crate::core::{Block, Header, ProofOfWork};
use crate::zk::{ZkCompressedState, ZkProof, ZkStateModel};

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

pub struct ExplorerStateModel {
    pub state_model: ZkStateModel,
}

pub enum ExplorerVerifierKey {
    Groth16(String),
}

pub struct ExplorerPaymentVerifierKey {
    pub vk: ExplorerVerifierKey,
    pub log4_payment_capacity: u8,
}

pub struct ExplorerContract {
    pub initial_state: String,
    pub state_model: ExplorerStateModel,
    pub payment_functions: Vec<ExplorerPaymentVerifierKey>,
    pub functions: Vec<ExplorerPaymentVerifierKey>,
}

pub struct ExplorerCompressedState {
    pub state: ZkCompressedState,
}

pub struct ExplorerContractPayment {}

pub struct ExplorerZkProof {
    pub proof: ZkProof,
}

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

pub struct ExplorerTransaction {
    pub src: String,
    pub nonce: u32,
    pub data: ExplorerTransactionData,
    pub fee: u64,
    pub sig: String,
}

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
