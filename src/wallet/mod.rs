use crate::core::{
    Address, ContractId, ContractUpdate, Money, Signature, Transaction, TransactionAndDelta,
    TransactionData,
};
use crate::crypto::{EdDSA, SignatureScheme};
use crate::zk;

#[derive(Debug, Clone)]
pub struct Wallet {
    seed: Vec<u8>,
}

impl Wallet {
    pub fn new(seed: Vec<u8>) -> Self {
        Self { seed }
    }
    pub fn get_address(&self) -> Address {
        let (pk, _) = EdDSA::generate_keys(&self.seed);
        Address::PublicKey(pk)
    }
    pub fn create_transaction(
        &self,
        dst: Address,
        amount: Money,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let (_, sk) = EdDSA::generate_keys(&self.seed);
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::RegularSend { dst, amount },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(EdDSA::sign(&sk, &bytes));
        TransactionAndDelta {
            tx,
            state_delta: None,
        }
    }
    pub fn create_contract(
        &self,
        contract: zk::ZkContract,
        initial_state: zk::ZkState,
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
        state_delta: zk::ZkStateDelta,
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
