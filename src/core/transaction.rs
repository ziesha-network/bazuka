use super::address::{Address, Signature};
use super::hash::Hash;
use super::Money;
use crate::crypto::{SignatureScheme, ZkSignatureScheme};
use crate::zk::{ZkCompressedState, ZkContract, ZkDeltaPairs, ZkProof, ZkScalar};

use std::str::FromStr;
use thiserror::Error;

#[derive(
    serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone, Copy, Eq, std::hash::Hash,
)]
pub struct ContractId<H: Hash>(H::Output);

#[derive(Error, Debug)]
pub enum ParseContractIdError {
    #[error("contract-id invalid")]
    Invalid,
}

impl<H: Hash> ContractId<H> {
    pub fn new<S: SignatureScheme, ZS: ZkSignatureScheme>(tx: &Transaction<H, S, ZS>) -> Self {
        Self(tx.hash())
    }
}

impl<H: Hash> std::fmt::Display for ContractId<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl<H: Hash> FromStr for ContractId<H> {
    type Err = ParseContractIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ParseContractIdError::Invalid)?;
        let hash_output = H::Output::try_from(bytes).map_err(|_| ParseContractIdError::Invalid)?;
        Ok(Self(hash_output))
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum PaymentDirection<S: SignatureScheme, ZS: ZkSignatureScheme> {
    Deposit(Option<S::Sig>),
    Withdraw(Option<ZS::Sig>),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ContractPayment<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub address: S::Pub,
    pub zk_address: ZS::Pub,
    pub zk_address_index: u32, // Help locating the zk_address in a a huge database
    pub contract_id: ContractId<H>, // Makes sure the payment can only run on this contract.
    pub nonce: u32,            // Makes sure a contract payment cannot be replayed on this contract.
    pub amount: Money,
    pub fee: Money, // Executor fee
    pub direction: PaymentDirection<S, ZS>,
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> PartialEq for ContractPayment<H, S, ZS> {
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}

impl<H: Hash + PartialEq, S: SignatureScheme + PartialEq, ZS: ZkSignatureScheme + PartialEq> Eq
    for ContractPayment<H, S, ZS>
{
}
impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> std::hash::Hash
    for ContractPayment<H, S, ZS>
{
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> ContractPayment<H, S, ZS> {
    pub fn verify_signature(&self) -> bool {
        let mut unsigned = self.clone();
        unsigned.direction = match &unsigned.direction {
            PaymentDirection::<S, ZS>::Deposit(_) => PaymentDirection::<S, ZS>::Deposit(None),
            PaymentDirection::<S, ZS>::Withdraw(_) => PaymentDirection::<S, ZS>::Withdraw(None),
        };
        let unsigned_bin = bincode::serialize(&unsigned).unwrap();
        match &self.direction {
            PaymentDirection::<S, ZS>::Deposit(Some(sig)) => {
                S::verify(&self.address, &unsigned_bin, sig)
            }
            PaymentDirection::<S, ZS>::Withdraw(Some(sig)) => {
                let scalar = ZkScalar::new(H::hash(&unsigned_bin).as_ref());
                ZS::verify(&self.zk_address, scalar, sig)
            }
            _ => false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub struct ContractAccount {
    pub height: u64,
    pub balance: Money,
    pub compressed_state: ZkCompressedState,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum ContractUpdate<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    // Proof for PaymentCircuit(curr_state, next_state, hash(entries))
    Payment {
        circuit_id: u32,
        payments: Vec<ContractPayment<H, S, ZS>>,
        next_state: ZkCompressedState,
        proof: ZkProof,
    },
    // Proof for FunctionCallCircuits[function_id](curr_state, next_state)
    FunctionCall {
        function_id: u32,
        next_state: ZkCompressedState,
        proof: ZkProof,
        fee: Money, // Executor fee
    },
}

// A transaction could be as simple as sending some funds, or as complicated as
// creating a smart-contract.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
pub enum TransactionData<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    RegularSend {
        dst: Address<S>,
        amount: Money,
    },
    // Create a Zero-Contract. The creator can consider multiple ways (Circuits) of updating
    // the state. But there should be only one circuit for entering and exiting the contract.
    CreateContract {
        contract: ZkContract,
    },
    // Collection of contract updates
    UpdateContract {
        contract_id: ContractId<H>,
        updates: Vec<ContractUpdate<H, S, ZS>>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Transaction<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub src: Address<S>,
    pub nonce: u32,
    pub data: TransactionData<H, S, ZS>,
    pub fee: Money,
    pub sig: Signature<S>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TransactionAndDelta<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub tx: Transaction<H, S, ZS>,
    pub state_delta: Option<ZkDeltaPairs>,
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> PartialEq<TransactionAndDelta<H, S, ZS>>
    for TransactionAndDelta<H, S, ZS>
{
    fn eq(&self, other: &Self) -> bool {
        bincode::serialize(self).unwrap() == bincode::serialize(other).unwrap()
    }
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> Transaction<H, S, ZS> {
    pub fn size(&self) -> usize {
        bincode::serialize(self).unwrap().len()
    }
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(self).unwrap())
    }
    pub fn verify_signature(&self) -> bool {
        match &self.src {
            Address::<S>::Treasury => true,
            Address::<S>::PublicKey(pk) => match &self.sig {
                Signature::Unsigned => false,
                Signature::Signed(sig) => {
                    let mut unsigned = self.clone();
                    unsigned.sig = Signature::Unsigned;
                    let bytes = bincode::serialize(&unsigned).unwrap();
                    S::verify(pk, &bytes, sig)
                }
            },
        }
    }
}

impl<H: Hash, S: SignatureScheme + PartialEq, ZS: ZkSignatureScheme + PartialEq> Eq
    for TransactionAndDelta<H, S, ZS>
{
}
impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> std::hash::Hash
    for TransactionAndDelta<H, S, ZS>
{
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        state.write(&bincode::serialize(self).unwrap());
        state.finish();
    }
}
