use super::address::Signature;
use super::hash::Hash;
use super::Amount;
use crate::crypto::VerifiableRandomFunction;
use crate::crypto::{SignatureScheme, ZkSignatureScheme};
use crate::zk::{
    ZkCompressedState, ZkContract, ZkDataPairs, ZkDeltaPairs, ZkHasher, ZkProof, ZkScalar,
};
use ff::Field;
use std::str::FromStr;
use thiserror::Error;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    Debug,
    Clone,
    Copy,
    Eq,
    std::hash::Hash,
    Default,
)]
pub struct UndelegationId<H: Hash>(H::Output);

#[derive(Error, Debug)]
pub enum ParseUndelegationIdError {
    #[error("undelegate-id invalid")]
    Invalid,
}

impl<H: Hash> UndelegationId<H> {
    pub fn new<S: SignatureScheme, V: VerifiableRandomFunction>(tx: &Transaction<H, S, V>) -> Self {
        Self(tx.hash())
    }
}

impl<H: Hash> std::fmt::Display for UndelegationId<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl<H: Hash> FromStr for UndelegationId<H> {
    type Err = ParseUndelegationIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|_| ParseUndelegationIdError::Invalid)?;
        let hash_output =
            H::Output::try_from(bytes).map_err(|_| ParseUndelegationIdError::Invalid)?;
        Ok(Self(hash_output))
    }
}

#[derive(Error, Debug)]
pub enum ParseContractIdError {
    #[error("contract-id invalid")]
    Invalid,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone, Copy, Hash, Eq)]
pub enum ContractId<H: Hash> {
    Null(std::marker::PhantomData<H>),
    Ziesha,
    Custom(ZkScalar),
}
impl<H: Hash> Default for ContractId<H> {
    fn default() -> Self {
        Self::Null(std::marker::PhantomData)
    }
}
impl<H: Hash> ContractId<H> {
    pub fn new<S: SignatureScheme, V: VerifiableRandomFunction>(tx: &Transaction<H, S, V>) -> Self {
        Self::Custom(crate::zk::hash_to_scalar(&bincode::serialize(&tx).unwrap()))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Money<H: Hash> {
    pub token_id: ContractId<H>,
    pub amount: Amount,
}

impl<H: Hash> Money<H> {
    pub fn new(token_id: ContractId<H>, amount: u64) -> Self {
        Self {
            token_id,
            amount: Amount(amount),
        }
    }
    pub fn ziesha(amount: u64) -> Self {
        Self {
            token_id: ContractId::Ziesha,
            amount: Amount(amount),
        }
    }
}

impl<H: Hash> From<ZkScalar> for ContractId<H> {
    fn from(val: ZkScalar) -> Self {
        if val == ZkScalar::ZERO {
            Self::Null(std::marker::PhantomData)
        } else if val == ZkScalar::ONE {
            Self::Ziesha
        } else {
            Self::Custom(val)
        }
    }
}

impl<H: Hash> std::fmt::Display for ContractId<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ContractId::Null(_) => {
                write!(f, "Null")
            }
            ContractId::Ziesha => {
                write!(f, "Ziesha")
            }
            ContractId::Custom(id) => {
                write!(f, "{}", id)
            }
        }
    }
}

impl<H: Hash> FromStr for ContractId<H> {
    type Err = ParseContractIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Ziesha" {
            Ok(Self::Ziesha)
        } else {
            let parsed: ZkScalar = s.parse().map_err(|_| Self::Err::Invalid)?;
            Ok(Self::Custom(parsed))
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractDeposit<H: Hash, S: SignatureScheme> {
    pub memo: String,
    pub contract_id: ContractId<H>,
    pub deposit_circuit_id: u32,
    pub calldata: ZkScalar,
    pub src: S::Pub,
    pub amount: Money<H>,
    pub fee: Money<H>,
    pub nonce: u32,
    pub sig: Option<S::Sig>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct ContractWithdraw<H: Hash, S: SignatureScheme> {
    pub memo: String,
    pub contract_id: ContractId<H>,
    pub withdraw_circuit_id: u32,
    pub calldata: ZkScalar,
    pub dst: S::Pub,
    pub amount: Money<H>, // Amount sent from contract to dst
    pub fee: Money<H>,    // Executor fee, paid by contract
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct MpnDeposit<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub mpn_address: ZS::Pub,
    pub payment: ContractDeposit<H, S>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct MpnWithdraw<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub mpn_address: ZS::Pub,
    pub mpn_withdraw_nonce: u32,
    pub mpn_sig: ZS::Sig,
    pub payment: ContractWithdraw<H, S>,
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> MpnWithdraw<H, S, ZS> {
    pub fn verify_calldata<ZH: ZkHasher>(&self) -> bool {
        let mut preimage: Vec<ZkScalar> = self.mpn_address.clone().into();
        preimage.push((self.mpn_withdraw_nonce as u64).into());
        preimage.extend(&self.mpn_sig.clone().into());
        self.payment.calldata == ZH::hash(&preimage)
    }
    pub fn verify_signature<ZH: ZkHasher>(&self) -> bool {
        let msg = ZH::hash(&[
            self.payment.fingerprint(),
            ZkScalar::from(self.mpn_withdraw_nonce as u64),
        ]);
        ZS::verify(&self.mpn_address, msg, &self.mpn_sig)
    }
}

impl<H: Hash, S: SignatureScheme> ContractDeposit<H, S> {
    pub fn verify_signature(&self) -> bool {
        let mut unsigned = self.clone();
        unsigned.sig = None;
        let unsigned_bin = bincode::serialize(&unsigned).unwrap();
        self.sig
            .as_ref()
            .map(|sig| S::verify(&self.src, &unsigned_bin, sig))
            .unwrap_or(false)
    }
}

impl<H: Hash, S: SignatureScheme> ContractWithdraw<H, S> {
    pub fn fingerprint(&self) -> ZkScalar {
        let mut unsigned = self.clone();
        unsigned.calldata = ZkScalar::default();
        let unsigned_bin = bincode::serialize(&unsigned).unwrap();
        ZkScalar::new(H::hash(&unsigned_bin).as_ref())
    }
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ContractAccount {
    pub height: u64,
    pub compressed_state: ZkCompressedState,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ContractUpdateData<H: Hash, S: SignatureScheme> {
    // Proof for DepositCircuit[circuit_id](curr_state, next_state, hash(entries))
    Deposit {
        deposits: Vec<ContractDeposit<H, S>>,
    },
    // Proof for WithdrawCircuit[circuit_id](curr_state, next_state, hash(entries))
    Withdraw {
        withdraws: Vec<ContractWithdraw<H, S>>,
    },
    // Proof for FunctionCallCircuits[function_id](curr_state, next_state)
    FunctionCall {
        fee: Money<H>, // Executor fee
    },
    Mint {
        amount: Amount,
    },
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ContractUpdate<H: Hash, S: SignatureScheme> {
    pub circuit_id: u32,
    pub data: ContractUpdateData<H, S>,
    pub next_state: ZkCompressedState,
    pub prover: S::Pub,
    pub reward: Amount,
    pub proof: ZkProof,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct RegularSendEntry<H: Hash, S: SignatureScheme> {
    pub dst: S::Pub,
    pub amount: Money<H>,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Token<S: SignatureScheme> {
    pub name: String,
    pub symbol: String,
    pub supply: Amount, // 1u64 in case of a NFT
    pub decimals: u8,
    pub minter: Option<S::Pub>,
}

impl<S: SignatureScheme> Token<S> {
    pub fn validate(&self) -> bool {
        use regex::Regex;
        const MIN_NAME_LEN: usize = 3;
        const MAX_NAME_LEN: usize = 32;
        const MIN_SYMBOL_LEN: usize = 3;
        const MAX_SYMBOL_LEN: usize = 6;
        lazy_static! {
            static ref RE_NAME: Regex = Regex::new(r"^(?:[a-zA-Z0-9]+ )*[a-zA-Z0-9]+$").unwrap();
            static ref RE_SYMBOL: Regex = Regex::new(r"^[A-Z][A-Z0-9]*$").unwrap();
        }
        self.name.len() >= MIN_NAME_LEN
            && self.name.len() <= MAX_NAME_LEN
            && self.symbol.len() >= MIN_SYMBOL_LEN
            && self.symbol.len() <= MAX_SYMBOL_LEN
            && RE_NAME.is_match(&self.name)
            && RE_SYMBOL.is_match(&self.symbol)
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord,
)]
pub struct Ratio(pub u8);

impl Into<f64> for Ratio {
    fn into(self) -> f64 {
        self.0 as f64 / u8::MAX as f64
    }
}

#[derive(Error, Debug)]
pub enum ConvertRatioError {
    #[error("floating point not in correct range")]
    Invalid,
}

impl TryFrom<f32> for Ratio {
    type Error = ConvertRatioError;
    fn try_from(val: f32) -> Result<Self, ConvertRatioError> {
        if val < 0.0 || val > 1.0 {
            Err(ConvertRatioError::Invalid)
        } else {
            Ok(Ratio((255.0f64 * val as f64) as u8))
        }
    }
}

// A transaction could be as simple as sending some funds, or as complicated as
// creating a smart-contract.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TransactionData<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    UpdateStaker {
        vrf_pub_key: V::Pub,
        commission: Ratio,
    },
    Delegate {
        amount: Amount,
        to: S::Pub,
    },
    Undelegate {
        amount: Amount,
        from: S::Pub,
    },
    AutoDelegate {
        to: S::Pub,
        ratio: Ratio,
    },
    RegularSend {
        entries: Vec<RegularSendEntry<H, S>>,
    },
    // Create a Zero-Contract. The creator can consider multiple ways (Circuits) of updating
    // the state. But there should be only one circuit for entering and exiting the contract.
    CreateContract {
        contract: ZkContract,
        money: Money<H>,
        state: Option<ZkDataPairs>, // Removable for space efficiency, not considered inside signature!
    },
    // Collection of contract updates
    UpdateContract {
        contract_id: ContractId<H>,
        updates: Vec<ContractUpdate<H, S>>,
        delta: Option<ZkDeltaPairs>, // Removable for space efficiency, not considered inside signature!
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Transaction<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    pub src: Option<S::Pub>, // None is reward treasury!
    pub nonce: u32,
    pub data: TransactionData<H, S, V>,
    pub fee: Money<H>,
    pub memo: String,
    pub sig: Signature<S>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TransactionAndDelta<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    pub tx: Transaction<H, S, V>,
    pub state_delta: Option<ZkDeltaPairs>,
}

impl<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> Transaction<H, S, V> {
    pub fn size(&self) -> usize {
        bincode::serialize(self).unwrap().len()
    }
    pub fn sig_state_excluded(&self) -> Self {
        let mut clean = self.clone();
        match &mut clean.data {
            TransactionData::UpdateContract { delta, .. } => {
                *delta = None;
            }
            TransactionData::CreateContract { state, .. } => {
                *state = None;
            }
            _ => {}
        }
        clean.sig = Signature::Unsigned;
        clean
    }
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(&self.sig_state_excluded()).unwrap())
    }
    pub fn verify_signature(&self) -> bool {
        match &self.src {
            None => true,
            Some(pk) => match &self.sig {
                Signature::Unsigned => false,
                Signature::Signed(sig) => {
                    let bytes = bincode::serialize(&self.sig_state_excluded()).unwrap();
                    S::verify(pk, &bytes, sig)
                }
            },
        }
    }
}
