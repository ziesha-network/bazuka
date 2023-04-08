use crate::core::{Money, MpnDeposit};
use crate::crypto::jubjub;
use crate::zk::groth16::gadgets::common::Number;
use crate::zk::groth16::gadgets::common::UnsignedInteger;
use crate::zk::groth16::gadgets::eddsa::AllocatedPoint;
use crate::zk::groth16::gadgets::merkle;
use crate::zk::groth16::gadgets::reveal::{reveal, AllocatedState};
use crate::zk::groth16::gadgets::{common, poseidon, BellmanFr};
use crate::zk::{MpnAccount, ZkScalar};
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{Circuit, ConstraintSystem, SynthesisError};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Deposit {
    pub mpn_deposit: Option<MpnDeposit>,
    pub index: u64,
    pub token_index: u64,
    pub pub_key: jubjub::PointAffine,
    pub amount: Money,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DepositTransition<const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8> {
    pub enabled: bool,
    pub tx: Deposit,
    pub before: MpnAccount,
    pub before_balances_hash: ZkScalar,
    pub before_balance: Money,
    pub proof: merkle::Proof<LOG4_TREE_SIZE>,
    pub balance_proof: merkle::Proof<LOG4_TOKENS_TREE_SIZE>,
}

impl<const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    DepositTransition<LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    pub fn from_crate(trans: crate::mpn::DepositTransition) -> Self {
        Self {
            enabled: true,
            tx: Deposit {
                mpn_deposit: Some(trans.tx.clone()),
                index: trans.tx.zk_address_index(LOG4_TREE_SIZE),
                token_index: trans.token_index,
                pub_key: trans.tx.zk_address.0.decompress(),
                amount: trans.tx.payment.amount,
            },
            before: trans.before,
            before_balances_hash: trans.before_balances_hash,
            before_balance: trans.before_balance,
            proof: merkle::Proof::<LOG4_TREE_SIZE>(trans.proof),
            balance_proof: merkle::Proof::<LOG4_TOKENS_TREE_SIZE>(trans.balance_proof),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepositTransitionBatch<
    const LOG4_BATCH_SIZE: u8,
    const LOG4_TREE_SIZE: u8,
    const LOG4_TOKENS_TREE_SIZE: u8,
>(Vec<DepositTransition<LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>>);
impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    DepositTransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    pub fn new(ts: Vec<crate::mpn::DepositTransition>) -> Self {
        let mut ts = ts
            .into_iter()
            .map(|t| DepositTransition::from_crate(t))
            .collect::<Vec<_>>();
        while ts.len() < 1 << (2 * LOG4_BATCH_SIZE) {
            ts.push(DepositTransition::default());
        }
        Self(ts)
    }
}
impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8> Default
    for DepositTransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    fn default() -> Self {
        Self(
            (0..1 << (2 * LOG4_BATCH_SIZE))
                .map(|_| DepositTransition::default())
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepositCircuit<
    const LOG4_BATCH_SIZE: u8,
    const LOG4_TREE_SIZE: u8,
    const LOG4_TOKENS_TREE_SIZE: u8,
> {
    pub height: u64,          // Public
    pub state: ZkScalar,      // Public
    pub aux_data: ZkScalar,   // Public
    pub next_state: ZkScalar, // Public
    pub transitions:
        Box<DepositTransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>>, // Secret :)
}

impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    Circuit<BellmanFr> for DepositCircuit<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    fn synthesize<CS: ConstraintSystem<BellmanFr>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        // Contract height feeded as input
        let height_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.height.into()))?;
        height_wit.inputize(&mut *cs)?;

        // Previous state feeded as input
        let mut state_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.state.into()))?;
        state_wit.inputize(&mut *cs)?;

        // Sum of internal tx fees feeded as input
        let aux_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.aux_data.into()))?;
        aux_wit.inputize(&mut *cs)?;

        // Expected next state feeded as input
        let claimed_next_state_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.next_state.into()))?;
        claimed_next_state_wit.inputize(&mut *cs)?;

        let state_model = crate::zk::ZkStateModel::List {
            item_type: Box::new(crate::zk::ZkStateModel::Struct {
                field_types: vec![
                    crate::zk::ZkStateModel::Scalar, // Enabled
                    crate::zk::ZkStateModel::Scalar, // Token-id
                    crate::zk::ZkStateModel::Scalar, // Amount
                    crate::zk::ZkStateModel::Scalar, // Calldata
                ],
            }),
            log4_size: LOG4_BATCH_SIZE,
        };

        // Uncompress all the Deposit txs that were compressed inside aux_witness
        let mut tx_wits = Vec::new();
        let mut children = Vec::new();
        for trans in self.transitions.0.iter() {
            // If enabled, transaction is validated, otherwise neglected
            let enabled = AllocatedBit::alloc(&mut *cs, Some(trans.enabled))?;

            // Tx amount should always have at most 64 bits
            let token_id = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.tx.amount.token_id).into())
            })?;

            // Tx amount should always have at most 64 bits
            let amount = UnsignedInteger::alloc_64(&mut *cs, trans.tx.amount.amount.into())?;

            // Pub-key only needs to reside on curve if tx is enabled, which is checked in the main loop
            let pub_key = AllocatedPoint::alloc(&mut *cs, || Ok(trans.tx.pub_key))?;

            tx_wits.push((
                Boolean::Is(enabled.clone()),
                token_id.clone(),
                amount.clone(),
                pub_key.clone(),
            ));
            let pub_key_hash =
                poseidon::poseidon(&mut *cs, &[&pub_key.x.into(), &pub_key.y.into()])?;

            let calldata = common::mux(
                &mut *cs,
                &enabled.clone().into(),
                &Number::zero(),
                &pub_key_hash,
            )?;

            children.push(AllocatedState::Children(vec![
                AllocatedState::Value(enabled.into()),
                AllocatedState::Value(token_id.into()),
                AllocatedState::Value(amount.into()),
                AllocatedState::Value(calldata.into()),
            ]));
        }
        let tx_root = reveal(&mut *cs, &state_model, &AllocatedState::Children(children))?;
        cs.enforce(
            || "",
            |lc| lc + aux_wit.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + tx_root.get_lc(),
        );

        for (trans, (enabled_wit, tx_token_id_wit, tx_amount_wit, tx_pub_key_wit)) in
            self.transitions.0.iter().zip(tx_wits.into_iter())
        {
            // Tx index should always have at most LOG4_TREE_SIZE * 2 bits
            let tx_index_wit =
                UnsignedInteger::constrain_strict(&mut *cs, tx_pub_key_wit.x.clone().into())?
                    .extract_bits(LOG4_TREE_SIZE as usize * 2);

            let tx_token_index_wit = UnsignedInteger::alloc(
                &mut *cs,
                (trans.tx.token_index as u64).into(),
                LOG4_TOKENS_TREE_SIZE as usize * 2,
            )?;

            // Check if tx pub-key resides on the curve if tx is enabled
            tx_pub_key_wit.assert_on_curve(&mut *cs, &enabled_wit)?;

            let src_tx_nonce_wit =
                AllocatedNum::alloc(&mut *cs, || Ok((trans.before.tx_nonce as u64).into()))?;
            let src_withdraw_nonce_wit =
                AllocatedNum::alloc(&mut *cs, || Ok((trans.before.withdraw_nonce as u64).into()))?;

            // Account address doesn't necessarily need to reside on curve as it might be empty
            let src_addr_wit = AllocatedPoint::alloc(&mut *cs, || Ok(trans.before.address))?;

            let src_balances_hash_wit =
                AllocatedNum::alloc(&mut *cs, || Ok(trans.before_balances_hash.into()))?;

            let src_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.before_balance.token_id).into())
            })?;

            // We don't need to make sure account balance is 64 bits. If everything works as expected
            // nothing like this should happen.
            let src_balance_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<u64>::into(trans.before_balance.amount).into())
            })?;

            let src_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_token_id_wit.clone().into(),
                    &src_balance_wit.clone().into(),
                ],
            )?;

            let mut src_balance_proof_wits = Vec::new();
            for b in trans.balance_proof.0.clone() {
                src_balance_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }

            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_token_index_wit.clone().into(),
                &src_token_balance_hash_wit,
                &src_balance_proof_wits,
                &src_balances_hash_wit.clone().into(),
            )?;

            let src_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_tx_nonce_wit.clone().into(),
                    &src_withdraw_nonce_wit.clone().into(),
                    &src_addr_wit.x.clone().into(),
                    &src_addr_wit.y.clone().into(),
                    &src_balances_hash_wit.clone().into(),
                ],
            )?;

            let mut proof_wits = Vec::new();
            for b in trans.proof.0.clone() {
                proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }

            // Token-id of account slot can either be empty or equal with tx token-id
            let is_src_token_id_null = Number::from(src_token_id_wit.clone()).is_zero(&mut *cs)?;
            let is_src_token_id_and_tx_token_id_equal = Number::from(src_token_id_wit.clone())
                .is_equal(&mut *cs, &tx_token_id_wit.clone().into())?;
            let token_id_valid = common::boolean_or(
                &mut *cs,
                &is_src_token_id_null,
                &is_src_token_id_and_tx_token_id_equal,
            )?;
            common::assert_true(&mut *cs, &token_id_valid);

            // Address of account slot can either be empty or equal with tx destination
            let is_src_addr_null = src_addr_wit.is_null(&mut *cs)?;
            let is_src_and_tx_pub_key_equal = src_addr_wit.is_equal(&mut *cs, &tx_pub_key_wit)?;
            let addr_valid =
                common::boolean_or(&mut *cs, &is_src_addr_null, &is_src_and_tx_pub_key_equal)?;
            common::assert_true(&mut *cs, &addr_valid);

            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_index_wit.clone().into(),
                &src_hash_wit,
                &proof_wits,
                &state_wit.clone().into(),
            )?;

            let src_balance_lc = Number::from(src_balance_wit);
            let tx_amount_lc = Number::from(tx_amount_wit);

            let new_balances_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &tx_token_id_wit.clone().into(),
                    &(src_balance_lc.clone() + tx_amount_lc.clone()),
                ],
            )?;

            let new_balances_hash_wit = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_token_index_wit,
                &new_balances_hash_wit,
                &src_balance_proof_wits,
            )?;

            // Calculate next-state hash and update state if tx is enabled
            let new_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_tx_nonce_wit.clone().into(),
                    &src_withdraw_nonce_wit.clone().into(),
                    &tx_pub_key_wit.x.clone().into(),
                    &tx_pub_key_wit.y.clone().into(),
                    &new_balances_hash_wit,
                ],
            )?;
            let next_state_wit =
                merkle::calc_root_poseidon4(&mut *cs, &tx_index_wit, &new_hash_wit, &proof_wits)?;
            state_wit = common::mux(&mut *cs, &enabled_wit, &state_wit.into(), &next_state_wit)?;
        }

        // Check if applying txs result in the claimed next state
        cs.enforce(
            || "",
            |lc| lc + state_wit.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + claimed_next_state_wit.get_variable(),
        );

        Ok(())
    }
}
