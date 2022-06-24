use crate::core::{
    Address, ContractId, ContractUpdate, Money, Signature, Transaction, TransactionAndDelta,
    TransactionData,
};
use crate::crypto::{EdDSA, PrivateKey, SignatureScheme};
use crate::zk;

#[derive(Clone)]
pub struct Wallet {
    seed: Vec<u8>,
    private_key: PrivateKey,
    address: Address,
}

impl Wallet {
    pub fn new(seed: Vec<u8>) -> Self {
        let (pk, sk) = EdDSA::generate_keys(&seed);
        Self {
            seed,
            address: Address::PublicKey(pk),
            private_key: sk,
        }
    }
    pub fn get_address(&self) -> Address {
        self.address.clone()
    }
    pub fn create_transaction(
        &self,
        dst: Address,
        amount: Money,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::RegularSend { dst, amount },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(EdDSA::sign(&self.private_key, &bytes));
        TransactionAndDelta {
            tx,
            state_delta: None,
        }
    }
    pub fn create_contract(
        &self,
        contract: zk::ZkContract,
        initial_state: zk::ZkDataPairs,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let (_, sk) = EdDSA::generate_keys(&self.seed);
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::CreateContract { contract },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(EdDSA::sign(&sk, &bytes));
        TransactionAndDelta {
            tx,
            state_delta: Some(initial_state.as_delta()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn call_function(
        &self,
        contract_id: ContractId,
        function_id: u32,
        state_delta: zk::ZkDeltaPairs,
        next_state: zk::ZkCompressedState,
        proof: zk::ZkProof,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let (_, sk) = EdDSA::generate_keys(&self.seed);
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::UpdateContract {
                contract_id,
                updates: vec![ContractUpdate::FunctionCall {
                    function_id,
                    next_state,
                    proof,
                }],
            },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(EdDSA::sign(&sk, &bytes));
        TransactionAndDelta {
            tx,
            state_delta: Some(state_delta),
        }
    }
}
