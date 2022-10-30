use crate::core::{
    Address, ContractDeposit, ContractId, ContractUpdate, ContractWithdraw, Money, MpnDeposit,
    MpnWithdraw, Signature, Signer, Transaction, TransactionAndDelta, TransactionData, ZkHasher,
    ZkSigner,
};
use crate::crypto::SignatureScheme;
use crate::crypto::ZkSignatureScheme;
use crate::zk;

#[derive(Clone)]
pub struct TxBuilder {
    seed: Vec<u8>,
    private_key: <Signer as SignatureScheme>::Priv,
    zk_private_key: <ZkSigner as ZkSignatureScheme>::Priv,
    address: <Signer as SignatureScheme>::Pub,
    zk_address: <ZkSigner as ZkSignatureScheme>::Pub,
}

impl TxBuilder {
    pub fn new(seed: Vec<u8>) -> Self {
        let (pk, sk) = Signer::generate_keys(&seed);
        let (zk_pk, zk_sk) = ZkSigner::generate_keys(&seed);
        Self {
            seed,
            address: pk,
            zk_address: zk_pk,
            private_key: sk,
            zk_private_key: zk_sk,
        }
    }
    pub fn get_address(&self) -> Address {
        Address::PublicKey(self.address.clone())
    }
    pub fn get_zk_address(&self) -> <ZkSigner as ZkSignatureScheme>::Pub {
        self.zk_address.clone()
    }
    pub fn sign(&self, tx: &mut Transaction) {
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(Signer::sign(&self.private_key, &bytes));
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
        self.sign(&mut tx);
        TransactionAndDelta {
            tx,
            state_delta: None,
        }
    }
    pub fn create_mpn_transaction(
        &self,
        from_index: u32,
        to_index: u32,
        to: <ZkSigner as ZkSignatureScheme>::Pub,
        amount: Money,
        fee: Money,
        nonce: u64,
    ) -> zk::MpnTransaction {
        let mut tx = zk::MpnTransaction {
            nonce,
            src_index: from_index,
            dst_index: to_index,
            dst_pub_key: to,
            amount,
            fee,
            sig: Default::default(),
        };
        tx.sign(&self.zk_private_key);
        tx
    }
    pub fn create_contract(
        &self,
        contract: zk::ZkContract,
        initial_state: zk::ZkDataPairs,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::CreateContract { contract },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        self.sign(&mut tx);
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
        exec_fee: Money,
        miner_fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let (_, sk) = Signer::generate_keys(&self.seed);
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::UpdateContract {
                contract_id,
                updates: vec![ContractUpdate::FunctionCall {
                    function_id,
                    next_state,
                    proof,
                    fee: exec_fee,
                }],
            },
            nonce,
            fee: miner_fee,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Signature::Signed(Signer::sign(&sk, &bytes));
        TransactionAndDelta {
            tx,
            state_delta: Some(state_delta),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn deposit_mpn(
        &self,
        contract_id: ContractId,
        zk_address_index: u32,
        nonce: u32,
        amount: Money,
        fee: Money,
    ) -> MpnDeposit {
        let mut calldata_builder =
            zk::ZkStateBuilder::<ZkHasher>::new(zk::MPN_DEPOSIT_STATE_MODEL.clone());
        let mut tx = ContractDeposit {
            src: self.private_key.clone().into(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: calldata_builder.compress().unwrap().state_hash,
            nonce,
            amount,
            fee,
            sig: None,
        };
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Some(Signer::sign(&self.private_key, &bytes));
        MpnDeposit {
            zk_address_index,
            payment: tx,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_mpn(
        &self,
        contract_id: ContractId,
        zk_address_index: u32,
        nonce: u32,
        amount: Money,
        fee: Money,
    ) -> MpnWithdraw {
        let mut calldata_builder =
            zk::ZkStateBuilder::<ZkHasher>::new(zk::MPN_DEPOSIT_STATE_MODEL.clone());
        let mut tx = ContractWithdraw {
            dst: self.private_key.clone().into(),
            contract_id,
            withdraw_circuit_id: 0,
            calldata: calldata_builder.compress().unwrap().state_hash,
            amount,
            fee,
        };
        MpnWithdraw {
            zk_address_index,
            payment: tx,
        }
    }
}
