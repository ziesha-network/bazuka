use crate::core::{
    Address, ContractDeposit, ContractId, ContractUpdate, ContractWithdraw, Money, MpnDeposit,
    MpnWithdraw, RegularSendEntry, Signature, Signer, Transaction, TransactionAndDelta,
    TransactionData, ZkSigner,
};
use crate::crypto::SignatureScheme;
use crate::crypto::ZkSignatureScheme;
use crate::zk;
use crate::zk::ZkHasher;

#[derive(Clone)]
pub struct TxBuilder {
    seed: Vec<u8>,
    private_key: <Signer as SignatureScheme>::Priv,
    zk_private_key: <ZkSigner as ZkSignatureScheme>::Priv,
    address: <Signer as SignatureScheme>::Pub,
    zk_address: <ZkSigner as ZkSignatureScheme>::Pub,
}

impl TxBuilder {
    pub fn new(seed: &[u8]) -> Self {
        let (pk, sk) = Signer::generate_keys(seed);
        let (zk_pk, zk_sk) = ZkSigner::generate_keys(seed);
        Self {
            seed: seed.to_vec(),
            address: pk,
            zk_address: zk_pk,
            private_key: sk,
            zk_private_key: zk_sk,
        }
    }
    pub fn get_pub_key(&self) -> <Signer as SignatureScheme>::Pub {
        self.address.clone()
    }
    pub fn get_priv_key(&self) -> <Signer as SignatureScheme>::Priv {
        self.private_key.clone()
    }
    pub fn get_address(&self) -> Address {
        Address::PublicKey(self.address.clone())
    }
    pub fn get_zk_address(&self) -> <ZkSigner as ZkSignatureScheme>::Pub {
        self.zk_address.clone()
    }
    pub fn sign(&self, bytes: &[u8]) -> <Signer as SignatureScheme>::Sig {
        Signer::sign(&self.private_key, &bytes)
    }
    pub fn sign_tx(&self, tx: &mut Transaction) {
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
        self.create_multi_transaction(vec![RegularSendEntry { dst, amount }], fee, nonce)
    }
    pub fn create_multi_transaction(
        &self,
        entries: Vec<RegularSendEntry>,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            src: self.get_address(),
            data: TransactionData::RegularSend { entries },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        self.sign_tx(&mut tx);
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
        self.sign_tx(&mut tx);
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
            zk::ZkStateBuilder::<crate::core::ZkHasher>::new(zk::MPN_DEPOSIT_STATE_MODEL.clone());
        let pk = self.get_zk_address().0.decompress();
        calldata_builder
            .batch_set(&zk::ZkDeltaPairs(
                [
                    (zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(pk.0))),
                    (zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(pk.1))),
                ]
                .into(),
            ))
            .unwrap();
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
            zk_address: self.get_zk_address(),
            payment: tx,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_mpn(
        &self,
        contract_id: ContractId,
        zk_address_index: u32,
        nonce: u64,
        amount: Money,
        fee: Money,
    ) -> MpnWithdraw {
        let mut tx = ContractWithdraw {
            dst: self.private_key.clone().into(),
            contract_id,
            withdraw_circuit_id: 0,
            calldata: zk::ZkScalar::default(),
            amount,
            fee,
        };
        let fingerprint: zk::ZkScalar =
            crate::zk::hash_to_scalar(&bincode::serialize(&tx).unwrap());
        let sig = ZkSigner::sign(
            &self.zk_private_key,
            crate::core::ZkHasher::hash(&[fingerprint, zk::ZkScalar::from(nonce as u64)]),
        );
        let mut calldata_builder =
            zk::ZkStateBuilder::<crate::core::ZkHasher>::new(zk::MPN_WITHDRAW_STATE_MODEL.clone());
        let pk = self.get_zk_address().0.decompress();
        calldata_builder
            .batch_set(&zk::ZkDeltaPairs(
                [
                    (zk::ZkDataLocator(vec![0]), Some(zk::ZkScalar::from(pk.0))),
                    (zk::ZkDataLocator(vec![1]), Some(zk::ZkScalar::from(pk.1))),
                    (
                        zk::ZkDataLocator(vec![2]),
                        Some(zk::ZkScalar::from(nonce as u64)),
                    ),
                    (
                        zk::ZkDataLocator(vec![3]),
                        Some(zk::ZkScalar::from(sig.r.0)),
                    ),
                    (
                        zk::ZkDataLocator(vec![4]),
                        Some(zk::ZkScalar::from(sig.r.1)),
                    ),
                    (zk::ZkDataLocator(vec![5]), Some(sig.s)),
                ]
                .into(),
            ))
            .unwrap();
        tx.calldata = calldata_builder.compress().unwrap().state_hash;
        MpnWithdraw {
            zk_address_index,
            zk_address: self.get_zk_address(),
            zk_nonce: nonce,
            zk_sig: sig,
            payment: tx,
        }
    }
}
