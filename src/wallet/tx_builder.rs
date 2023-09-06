#[cfg(feature = "client")]
use crate::client::{messages::ValidatorClaim, PeerAddress};

use crate::core::{
    hash::Hash, Address, Amount, ContractDeposit, ContractId, ContractWithdraw, Hasher, Money,
    MpnAddress, MpnDeposit, MpnWithdraw, Ratio, RegularSendEntry, Signature, Signer, Token,
    TokenId, Transaction, TransactionAndDelta, TransactionData, ValidatorProof, Vrf, ZkSigner,
};
use crate::crypto::SignatureScheme;
use crate::crypto::VerifiableRandomFunction;
use crate::crypto::ZkSignatureScheme;
use crate::zk;
use crate::zk::ZkHasher;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

#[derive(Clone)]
pub struct TxBuilder {
    vrf_private_key: <Vrf as VerifiableRandomFunction>::Priv,
    vrf_public_key: <Vrf as VerifiableRandomFunction>::Pub,
    private_key: <Signer as SignatureScheme>::Priv,
    zk_private_key: <ZkSigner as ZkSignatureScheme>::Priv,
    address: Address,
    mpn_address: <ZkSigner as ZkSignatureScheme>::Pub,
}

impl TxBuilder {
    pub fn new(seed: &[u8]) -> Self {
        let (pk, sk) = Signer::generate_keys(seed);
        let (zk_pk, zk_sk) = ZkSigner::generate_keys(seed);
        let chacha_seed: [u8; 32] = <Hasher as crate::core::hash::Hash>::hash(seed);
        let mut chacha_rng = ChaChaRng::from_seed(chacha_seed);
        let (vrf_public_key, vrf_private_key) = Vrf::generate_keys(&mut chacha_rng);
        Self {
            address: pk,
            mpn_address: zk_pk,
            private_key: sk,
            zk_private_key: zk_sk,
            vrf_public_key,
            vrf_private_key,
        }
    }
    pub fn get_priv_key(&self) -> <Signer as SignatureScheme>::Priv {
        self.private_key.clone()
    }
    pub fn get_address(&self) -> Address {
        self.address.clone()
    }
    pub fn get_vrf_public_key(&self) -> <Vrf as VerifiableRandomFunction>::Pub {
        self.vrf_public_key.clone()
    }
    pub fn get_zk_address(&self) -> <ZkSigner as ZkSignatureScheme>::Pub {
        self.mpn_address.clone()
    }
    pub fn get_mpn_address(&self) -> MpnAddress {
        MpnAddress {
            pub_key: self.get_zk_address(),
        }
    }
    pub fn sign(&self, bytes: &[u8]) -> <Signer as SignatureScheme>::Sig {
        Signer::sign(&self.private_key, bytes)
    }
    pub fn sign_deposit(&self, tx: &mut ContractDeposit) {
        let bytes = bincode::serialize(&tx).unwrap();
        tx.sig = Some(Signer::sign(&self.private_key, &bytes));
    }
    pub fn sign_tx(&self, tx: &mut Transaction) {
        let bytes = bincode::serialize(&tx.sig_state_excluded()).unwrap();
        tx.sig = Signature::Signed(Signer::sign(&self.private_key, &bytes));
    }
    pub fn delegate(
        &self,
        memo: String,
        address: Address,
        amount: Amount,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::Delegate {
                to: address,
                amount,
            },
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
    pub fn undelegate(
        &self,
        memo: String,
        address: Address,
        amount: Amount,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::Undelegate {
                from: address,
                amount,
            },
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
    pub fn auto_delegate(
        &self,
        memo: String,
        to: Address,
        ratio: Ratio,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::AutoDelegate { to, ratio },
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
    pub fn generate_random(
        &self,
        randomness: <Hasher as Hash>::Output,
        epoch: u32,
        slot: u32,
        attempt: u32,
    ) -> (
        <Vrf as VerifiableRandomFunction>::Out,
        <Vrf as VerifiableRandomFunction>::Proof,
    ) {
        Vrf::sign(
            &self.vrf_private_key,
            format!("{}-{}-{}-{}", hex::encode(randomness), epoch, slot, attempt).as_bytes(),
        )
    }
    pub fn register_validator(
        &self,
        memo: String,
        commission: Ratio,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::UpdateStaker {
                vrf_pub_key: self.vrf_public_key.clone(),
                commission,
            },
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
    #[cfg(feature = "client")]
    pub fn claim_validator(
        &self,
        timestamp: u32,
        proof: ValidatorProof,
        node: PeerAddress,
    ) -> ValidatorClaim {
        let mut claim = ValidatorClaim {
            timestamp,
            address: self.get_address(),
            proof,
            node,
            sig: Signature::Unsigned,
        };
        let bytes = bincode::serialize(&claim).unwrap();
        claim.sig = Signature::Signed(Signer::sign(&self.private_key, &bytes));
        claim
    }
    pub fn create_transaction(
        &self,
        memo: String,
        dst: Address,
        amount: Money,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        self.create_multi_transaction(memo, vec![RegularSendEntry { dst, amount }], fee, nonce)
    }
    pub fn create_token(
        &self,
        memo: String,
        name: String,
        symbol: String,
        supply: Amount,
        decimals: u8,
        minter: Option<Address>,
        fee: Money,
        nonce: u32,
    ) -> (TransactionAndDelta, TokenId) {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::CreateToken {
                token: Token {
                    name,
                    symbol,
                    minter,
                    supply,
                    decimals,
                },
            },
            nonce,
            fee,
            sig: Signature::Unsigned,
        };
        self.sign_tx(&mut tx);

        let token_id = TokenId::new(&tx);
        (
            TransactionAndDelta {
                tx,
                state_delta: None,
            },
            token_id,
        )
    }
    pub fn create_multi_transaction(
        &self,
        memo: String,
        entries: Vec<RegularSendEntry>,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
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
        to: MpnAddress,
        amount: Money,
        fee: Money,
        nonce: u32,
    ) -> zk::MpnTransaction {
        let mut tx = zk::MpnTransaction {
            nonce,

            src_pub_key: self.get_zk_address(),
            dst_pub_key: to.pub_key,

            amount,
            fee,
            sig: Default::default(),
        };
        tx.sign(&self.zk_private_key);
        tx
    }
    pub fn create_contract(
        &self,
        memo: String,
        contract: zk::ZkContract,
        initial_state: zk::ZkDataPairs,
        money: Money,
        fee: Money,
        nonce: u32,
    ) -> TransactionAndDelta {
        let mut tx = Transaction {
            memo,
            src: Some(self.get_address()),
            data: TransactionData::CreateContract {
                contract,
                state: Some(initial_state.clone()),
                money,
            },
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
    pub fn deposit_mpn(
        &self,
        memo: String,
        contract_id: ContractId,
        to: MpnAddress,
        nonce: u32,
        amount: Money,
        fee: Money,
    ) -> MpnDeposit {
        let mut calldata_builder =
            zk::ZkStateBuilder::<crate::core::ZkHasher>::new(zk::MPN_DEPOSIT_STATE_MODEL.clone());
        let pk = to.pub_key.0.decompress();
        calldata_builder
            .batch_set(&zk::ZkDeltaPairs(
                [
                    (zk::ZkDataLocator(vec![0]), Some(pk.0)),
                    (zk::ZkDataLocator(vec![1]), Some(pk.1)),
                ]
                .into(),
            ))
            .unwrap();
        let mut tx = ContractDeposit {
            memo,
            src: self.get_address(),
            contract_id,
            deposit_circuit_id: 0,
            calldata: calldata_builder.compress().unwrap().state_hash,
            nonce,
            amount,
            fee,
            sig: None,
        };
        self.sign_deposit(&mut tx);
        MpnDeposit {
            mpn_address: to.pub_key,
            payment: tx,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn withdraw_mpn(
        &self,
        memo: String,
        contract_id: ContractId,
        nonce: u32,
        amount: Money,
        fee: Money,
        to: <Signer as SignatureScheme>::Pub,
    ) -> MpnWithdraw {
        let mut tx = ContractWithdraw {
            memo,
            dst: to,
            contract_id,
            withdraw_circuit_id: 0,
            calldata: zk::ZkScalar::default(),
            amount,
            fee,
        };
        let sig = ZkSigner::sign(
            &self.zk_private_key,
            crate::core::ZkHasher::hash(&[tx.fingerprint(), zk::ZkScalar::from(nonce as u64)]),
        );
        let mut calldata_builder =
            zk::ZkStateBuilder::<crate::core::ZkHasher>::new(zk::MPN_WITHDRAW_STATE_MODEL.clone());
        let pk = self.get_zk_address().0.decompress();
        calldata_builder
            .batch_set(&zk::ZkDeltaPairs(
                [
                    (zk::ZkDataLocator(vec![0]), Some(pk.0)),
                    (zk::ZkDataLocator(vec![1]), Some(pk.1)),
                    (
                        zk::ZkDataLocator(vec![2]),
                        Some(zk::ZkScalar::from(nonce as u64)),
                    ),
                    (zk::ZkDataLocator(vec![3]), Some(sig.r.0)),
                    (zk::ZkDataLocator(vec![4]), Some(sig.r.1)),
                    (zk::ZkDataLocator(vec![5]), Some(sig.s)),
                ]
                .into(),
            ))
            .unwrap();
        tx.calldata = calldata_builder.compress().unwrap().state_hash;
        MpnWithdraw {
            mpn_address: self.get_zk_address(),
            mpn_withdraw_nonce: nonce,
            mpn_sig: sig,
            payment: tx,
        }
    }
}
