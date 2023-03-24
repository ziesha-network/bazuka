use super::address::Signature;
use super::hash::Hash;
use super::Amount;
use crate::crypto::DeriveMpnAccountIndex;
use crate::crypto::VerifiableRandomFunction;
use crate::crypto::{SignatureScheme, ZkSignatureScheme};
use crate::zk::{ZkCompressedState, ZkContract, ZkDeltaPairs, ZkHasher, ZkProof, ZkScalar};
use ff::Field;
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
    pub fn new<S: SignatureScheme, V: VerifiableRandomFunction>(tx: &Transaction<H, S, V>) -> Self {
        Self(tx.hash())
    }
}

impl<H: Hash> std::fmt::Display for ContractId<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

#[derive(Error, Debug)]
pub enum ParseTokenIdError {
    #[error("token-id invalid")]
    Invalid,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone, Copy, Hash, Eq)]
pub enum TokenId {
    Null,
    Ziesha,
    Custom(ZkScalar),
}
impl Default for TokenId {
    fn default() -> Self {
        Self::Null
    }
}
impl TokenId {
    pub fn new<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction>(
        tx: &Transaction<H, S, V>,
    ) -> Self {
        Self::Custom(crate::zk::hash_to_scalar(&bincode::serialize(&tx).unwrap()))
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Money {
    pub token_id: TokenId,
    pub amount: Amount,
}

impl Money {
    pub fn new(token_id: TokenId, amount: u64) -> Self {
        Self {
            token_id,
            amount: Amount(amount),
        }
    }
    pub fn ziesha(amount: u64) -> Self {
        Self {
            token_id: TokenId::Ziesha,
            amount: Amount(amount),
        }
    }
}

impl From<ZkScalar> for TokenId {
    fn from(val: ZkScalar) -> Self {
        if val == ZkScalar::ZERO {
            Self::Null
        } else if val == ZkScalar::ONE {
            Self::Ziesha
        } else {
            Self::Custom(val)
        }
    }
}

impl std::fmt::Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TokenId::Null => {
                write!(f, "Null")
            }
            TokenId::Ziesha => {
                write!(f, "Ziesha")
            }
            TokenId::Custom(id) => {
                write!(f, "{}", id)
            }
        }
    }
}

impl FromStr for TokenId {
    type Err = ParseTokenIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "Ziesha" {
            Ok(Self::Ziesha)
        } else {
            let parsed: ZkScalar = s.parse().map_err(|_| Self::Err::Invalid)?;
            Ok(Self::Custom(parsed))
        }
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ContractDeposit<H: Hash, S: SignatureScheme> {
    pub memo: String,
    pub contract_id: ContractId<H>,
    pub deposit_circuit_id: u32,
    pub calldata: ZkScalar,
    pub src: S::Pub,
    pub amount: Money,
    pub fee: Money,
    pub nonce: u32,
    pub sig: Option<S::Sig>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ContractWithdraw<H: Hash, S: SignatureScheme> {
    pub memo: String,
    pub contract_id: ContractId<H>,
    pub withdraw_circuit_id: u32,
    pub calldata: ZkScalar,
    pub dst: S::Pub,
    pub amount: Money, // Amount sent from contract to dst
    pub fee: Money,    // Executor fee, paid by contract
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MpnDeposit<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub zk_address: ZS::Pub,
    pub zk_token_index: u64,
    pub payment: ContractDeposit<H, S>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MpnWithdraw<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> {
    pub zk_address: ZS::Pub,
    pub zk_token_index: u64,
    pub zk_fee_token_index: u64,
    pub zk_nonce: u64,
    pub zk_sig: ZS::Sig,
    pub payment: ContractWithdraw<H, S>,
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> MpnDeposit<H, S, ZS>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
    pub fn zk_address_index(&self, log4_account_capacity: u8) -> u64 {
        self.zk_address.mpn_account_index(log4_account_capacity)
    }
}

impl<H: Hash, S: SignatureScheme, ZS: ZkSignatureScheme> MpnWithdraw<H, S, ZS>
where
    ZS::Pub: DeriveMpnAccountIndex,
{
    pub fn zk_address_index(&self, log4_account_capacity: u8) -> u64 {
        self.zk_address.mpn_account_index(log4_account_capacity)
    }
    pub fn verify_calldata<ZH: ZkHasher>(&self) -> bool {
        let mut preimage: Vec<ZkScalar> = self.zk_address.clone().into();
        preimage.push(self.zk_nonce.clone().into());
        preimage.extend(&self.zk_sig.clone().into());
        self.payment.calldata == ZH::hash(&preimage)
    }
    pub fn verify_signature<ZH: ZkHasher>(&self) -> bool {
        let msg = ZH::hash(&[
            self.payment.fingerprint(),
            ZkScalar::from(self.zk_nonce as u64),
        ]);
        ZS::verify(&self.zk_address, msg, &self.zk_sig)
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
pub enum ContractUpdate<H: Hash, S: SignatureScheme> {
    // Proof for DepositCircuit[circuit_id](curr_state, next_state, hash(entries))
    Deposit {
        deposit_circuit_id: u32,
        deposits: Vec<ContractDeposit<H, S>>,
        next_state: ZkCompressedState,
        proof: ZkProof,
    },
    // Proof for WithdrawCircuit[circuit_id](curr_state, next_state, hash(entries))
    Withdraw {
        withdraw_circuit_id: u32,
        withdraws: Vec<ContractWithdraw<H, S>>,
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

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct RegularSendEntry<S: SignatureScheme> {
    pub dst: S::Pub,
    pub amount: Money,
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

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenUpdate<S: SignatureScheme> {
    Mint { amount: Amount },
    ChangeMinter { minter: S::Pub },
}

// A transaction could be as simple as sending some funds, or as complicated as
// creating a smart-contract.
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum TransactionData<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    UpdateStaker {
        vrf_pub_key: V::Pub,
        commision: u8, // n parts out of 255 parts
    },
    Delegate {
        amount: Amount,
        to: S::Pub,
        reverse: bool,
    },
    RegularSend {
        entries: Vec<RegularSendEntry<S>>,
    },
    // Create a Zero-Contract. The creator can consider multiple ways (Circuits) of updating
    // the state. But there should be only one circuit for entering and exiting the contract.
    CreateContract {
        contract: ZkContract,
    },
    // Collection of contract updates
    UpdateContract {
        contract_id: ContractId<H>,
        updates: Vec<ContractUpdate<H, S>>,
    },
    CreateToken {
        token: Token<S>,
    },
    UpdateToken {
        token_id: TokenId,
        update: TokenUpdate<S>,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Transaction<H: Hash, S: SignatureScheme, V: VerifiableRandomFunction> {
    pub src: Option<S::Pub>, // None is reward treasury!
    pub nonce: u32,
    pub data: TransactionData<H, S, V>,
    pub fee: Money,
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
    pub fn hash(&self) -> H::Output {
        H::hash(&bincode::serialize(self).unwrap())
    }
    pub fn verify_signature(&self) -> bool {
        match &self.src {
            None => true,
            Some(pk) => match &self.sig {
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
