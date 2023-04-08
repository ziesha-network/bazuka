use crate::core::{Money, TokenId};
use crate::zk::groth16::gadgets::common::Number;
use crate::zk::groth16::gadgets::common::UnsignedInteger;
use crate::zk::groth16::gadgets::eddsa::AllocatedPoint;
use crate::zk::groth16::gadgets::merkle;
use crate::zk::groth16::gadgets::{common, eddsa, poseidon, BellmanFr};
use crate::zk::{MpnAccount, MpnTransaction, ZkScalar};
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{Circuit, ConstraintSystem, SynthesisError};

// Validation:
// 0. Check verify_sig(tx)
// 1. Check verify_proof(curr_root, src_before, src_proof)
// 2. src_after := update_acc(src_before, tx)
// 3. root_after_src := calc_new_root(src_after, src_proof)
// 4. Check verify_proof(root_after_src, dst_before, dst_proof)
// 5. dst_after := update_acc(dst_after, tx)
// 6. root_after_dst := calc_new_root(dst_after, dst_proof)
// 7. Check next_state == root_after_dst
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Transition<const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8> {
    pub enabled: bool,
    pub src_token_index: u64,
    pub src_fee_token_index: u64,
    pub dst_token_index: u64,
    pub tx: MpnTransaction,
    pub src_before: MpnAccount, // src_after can be derived
    pub src_before_balances_hash: ZkScalar,
    pub src_before_balance: Money,
    pub src_before_fee_balance: Money,
    pub src_proof: merkle::Proof<LOG4_TREE_SIZE>,
    pub src_balance_proof: merkle::Proof<LOG4_TOKENS_TREE_SIZE>,
    pub src_fee_balance_proof: merkle::Proof<LOG4_TOKENS_TREE_SIZE>,
    pub dst_before: MpnAccount, // dst_after can be derived
    pub dst_before_balances_hash: ZkScalar,
    pub dst_before_balance: Money,
    pub dst_proof: merkle::Proof<LOG4_TREE_SIZE>,
    pub dst_balance_proof: merkle::Proof<LOG4_TOKENS_TREE_SIZE>,
}

impl<const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    Transition<LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    pub fn from_crate(trans: crate::mpn::UpdateTransition) -> Self {
        Self {
            enabled: true,
            tx: trans.tx,
            src_token_index: trans.src_token_index,
            src_fee_token_index: trans.src_fee_token_index,
            dst_token_index: trans.dst_token_index,
            src_before: trans.src_before, // src_after can be derived
            src_before_balances_hash: trans.src_before_balances_hash,
            src_before_balance: trans.src_before_balance,
            src_before_fee_balance: trans.src_before_fee_balance,
            src_proof: merkle::Proof::<LOG4_TREE_SIZE>(trans.src_proof),
            src_balance_proof: merkle::Proof::<LOG4_TOKENS_TREE_SIZE>(trans.src_balance_proof),
            src_fee_balance_proof: merkle::Proof::<LOG4_TOKENS_TREE_SIZE>(
                trans.src_fee_balance_proof,
            ),
            dst_before: trans.dst_before, // dst_after can be derived
            dst_before_balances_hash: trans.dst_before_balances_hash,
            dst_before_balance: trans.dst_before_balance,
            dst_proof: merkle::Proof::<LOG4_TREE_SIZE>(trans.dst_proof),
            dst_balance_proof: merkle::Proof::<LOG4_TOKENS_TREE_SIZE>(trans.dst_balance_proof),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransitionBatch<
    const LOG4_BATCH_SIZE: u8,
    const LOG4_TREE_SIZE: u8,
    const LOG4_TOKENS_TREE_SIZE: u8,
>(Vec<Transition<LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>>);
impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    TransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    pub fn new(ts: Vec<crate::mpn::UpdateTransition>) -> Self {
        let mut ts = ts
            .into_iter()
            .map(|t| Transition::from_crate(t))
            .collect::<Vec<_>>();
        while ts.len() < 1 << (2 * LOG4_BATCH_SIZE) {
            ts.push(Transition::default());
        }
        Self(ts)
    }
}
impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8> Default
    for TransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
{
    fn default() -> Self {
        Self(
            (0..1 << (2 * LOG4_BATCH_SIZE))
                .map(|_| Transition::default())
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateCircuit<
    const LOG4_BATCH_SIZE: u8,
    const LOG4_TREE_SIZE: u8,
    const LOG4_TOKENS_TREE_SIZE: u8,
> {
    pub height: u64,          // Public
    pub state: ZkScalar,      // Public
    pub aux_data: ZkScalar,   // Public
    pub next_state: ZkScalar, // Public
    pub fee_token: TokenId,   // Private
    pub transitions: Box<TransitionBatch<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>>, // Secret :)
}

impl<const LOG4_BATCH_SIZE: u8, const LOG4_TREE_SIZE: u8, const LOG4_TOKENS_TREE_SIZE: u8>
    Circuit<BellmanFr> for UpdateCircuit<LOG4_BATCH_SIZE, LOG4_TREE_SIZE, LOG4_TOKENS_TREE_SIZE>
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

        let accepted_fee_token = AllocatedNum::alloc(&mut *cs, || {
            Ok(Into::<ZkScalar>::into(self.fee_token).into())
        })?;

        // Sum of internal tx fees feeded as input
        let aux_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.aux_data.into()))?;
        aux_wit.inputize(&mut *cs)?;

        // Expected next state feeded as input
        let claimed_next_state_wit = AllocatedNum::alloc(&mut *cs, || Ok(self.next_state.into()))?;
        claimed_next_state_wit.inputize(&mut *cs)?;

        // Sum of tx fees as a linear-combination of tx fees
        let mut fee_sum = Number::zero();

        for trans in self.transitions.0.iter() {
            // If enabled, transaction is validated, otherwise neglected
            let enabled_wit = Boolean::Is(AllocatedBit::alloc(&mut *cs, Some(trans.enabled))?);

            let tx_src_token_index_wit = UnsignedInteger::alloc(
                &mut *cs,
                (trans.src_token_index as u64).into(),
                LOG4_TOKENS_TREE_SIZE as usize * 2,
            )?;

            let tx_src_fee_token_index_wit = UnsignedInteger::alloc(
                &mut *cs,
                (trans.src_fee_token_index as u64).into(),
                LOG4_TOKENS_TREE_SIZE as usize * 2,
            )?;

            let tx_dst_token_index_wit = UnsignedInteger::alloc(
                &mut *cs,
                (trans.dst_token_index as u64).into(),
                LOG4_TOKENS_TREE_SIZE as usize * 2,
            )?;

            let src_tx_nonce_wit =
                AllocatedNum::alloc(&mut *cs, || Ok((trans.src_before.tx_nonce as u64).into()))?;
            let src_withdraw_nonce_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok((trans.src_before.withdraw_nonce as u64).into())
            })?;

            let src_addr_wit = AllocatedPoint::alloc(&mut *cs, || Ok(trans.src_before.address))?;
            // Sender address should be on curve in case transaction slot is non-empty
            src_addr_wit.assert_on_curve(&mut *cs, &enabled_wit)?;

            let src_before_balances_hash =
                AllocatedNum::alloc(&mut *cs, || Ok(trans.src_before_balances_hash.into()))?;
            let dst_before_balances_hash =
                AllocatedNum::alloc(&mut *cs, || Ok(trans.dst_before_balances_hash.into()))?;

            let src_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.src_before_balance.token_id).into())
            })?;
            let src_balance_wit =
                UnsignedInteger::alloc_64(&mut *cs, trans.src_before_balance.amount.into())?;

            let src_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_token_id_wit.clone().into(),
                    &src_balance_wit.clone().into(),
                ],
            )?;

            let src_fee_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.src_before_fee_balance.token_id).into())
            })?;
            let src_fee_balance_wit =
                UnsignedInteger::alloc_64(&mut *cs, trans.src_before_fee_balance.amount.into())?;

            let src_fee_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_fee_token_id_wit.clone().into(),
                    &src_fee_balance_wit.clone().into(),
                ],
            )?;

            let mut src_balance_proof_wits = Vec::new();
            for b in trans.src_balance_proof.0.clone() {
                src_balance_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }
            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_src_token_index_wit.clone().into(),
                &src_token_balance_hash_wit.clone().into(),
                &src_balance_proof_wits,
                &src_before_balances_hash.clone().into(),
            )?;

            // Transaction amount and fee should at most have 64 bits
            let tx_amount_wit = UnsignedInteger::alloc_64(&mut *cs, trans.tx.amount.amount.into())?;
            let tx_fee_wit = UnsignedInteger::alloc_64(&mut *cs, trans.tx.fee.amount.into())?;

            let new_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_token_id_wit.clone().into(),
                    &(Number::from(src_balance_wit.clone()) - Number::from(tx_amount_wit.clone())),
                ],
            )?;
            let balance_middle_root = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_src_token_index_wit.clone().into(),
                &new_token_balance_hash_wit,
                &src_balance_proof_wits,
            )?;

            let mut src_fee_balance_proof_wits = Vec::new();
            for b in trans.src_fee_balance_proof.0.clone() {
                src_fee_balance_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }

            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_src_fee_token_index_wit.clone().into(),
                &src_fee_token_balance_hash_wit.clone().into(),
                &src_fee_balance_proof_wits,
                &balance_middle_root,
            )?;

            let new_fee_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_fee_token_id_wit.clone().into(),
                    &(Number::from(src_fee_balance_wit.clone()) - Number::from(tx_fee_wit.clone())),
                ],
            )?;

            let src_balance_final_root = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_src_fee_token_index_wit.clone().into(),
                &new_fee_token_balance_hash_wit,
                &src_fee_balance_proof_wits,
            )?;

            let tx_nonce_wit =
                AllocatedNum::alloc(&mut *cs, || Ok((trans.tx.nonce as u64).into()))?;

            // src and dst indices should only have 2 * LOG4_TREE_SIZE bits
            let tx_src_index_wit =
                UnsignedInteger::constrain_strict(&mut *cs, src_addr_wit.x.clone().into())?
                    .extract_bits(LOG4_TREE_SIZE as usize * 2);
            let tx_amount_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.tx.amount.token_id).into())
            })?;
            let tx_fee_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.tx.fee.token_id).into())
            })?;

            Number::from(accepted_fee_token.clone()).assert_equal_if_enabled(
                &mut *cs,
                &enabled_wit,
                &tx_fee_token_id_wit.clone().into(),
            )?;

            Number::from(src_token_id_wit.clone())
                .assert_equal(&mut *cs, &tx_amount_token_id_wit.clone().into());
            Number::from(src_fee_token_id_wit.clone())
                .assert_equal(&mut *cs, &tx_fee_token_id_wit.clone().into());

            let src_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &src_tx_nonce_wit.clone().into(),
                    &src_withdraw_nonce_wit.clone().into(),
                    &src_addr_wit.x.clone().into(),
                    &src_addr_wit.y.clone().into(),
                    &src_before_balances_hash.clone().into(),
                ],
            )?;

            let dst_token_id_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<ZkScalar>::into(trans.dst_before_balance.token_id).into())
            })?;
            // We also don't need to make sure dst balance is 64 bits. If everything works as expected
            // nothing like this should happen.
            let dst_balance_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok(Into::<u64>::into(trans.dst_before_balance.amount).into())
            })?;
            let dst_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &dst_token_id_wit.clone().into(),
                    &(Number::from(dst_balance_wit.clone())),
                ],
            )?;
            let new_dst_token_balance_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &tx_amount_token_id_wit.clone().into(),
                    &(Number::from(dst_balance_wit.clone()) + Number::from(tx_amount_wit.clone())),
                ],
            )?;

            let mut dst_balance_proof_wits = Vec::new();
            for b in trans.dst_balance_proof.0.clone() {
                dst_balance_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }
            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_dst_token_index_wit.clone().into(),
                &dst_token_balance_hash_wit.clone().into(),
                &dst_balance_proof_wits,
                &dst_before_balances_hash.clone().into(),
            )?;
            let dst_balance_final_root = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_dst_token_index_wit.clone().into(),
                &new_dst_token_balance_hash_wit,
                &dst_balance_proof_wits,
            )?;

            let mut src_proof_wits = Vec::new();
            for b in trans.src_proof.0.clone() {
                src_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }
            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_src_index_wit.clone().into(),
                &src_hash_wit,
                &src_proof_wits,
                &state_wit.clone().into(),
            )?;

            // Source nonce is incremented by one and balance is decreased by amount+fee
            let new_src_tx_nonce_wit =
                Number::from(src_tx_nonce_wit.clone()) + Number::constant::<CS>(BellmanFr::one());

            let new_src_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &new_src_tx_nonce_wit,
                    &src_withdraw_nonce_wit.clone().into(),
                    &src_addr_wit.x.clone().into(),
                    &src_addr_wit.y.clone().into(),
                    &src_balance_final_root,
                ],
            )?;

            // Root of the merkle tree after src account is updated
            let middle_root_wit = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_src_index_wit.clone().into(),
                &new_src_hash_wit,
                &src_proof_wits,
            )?;

            let tx_dst_addr_wit =
                AllocatedPoint::alloc(&mut *cs, || Ok(trans.tx.dst_pub_key.0.decompress()))?;
            // Destination address should be on curve in case transaction slot is non-empty
            tx_dst_addr_wit.assert_on_curve(&mut *cs, &enabled_wit)?;

            let tx_dst_index_wit =
                UnsignedInteger::constrain_strict(&mut *cs, tx_dst_addr_wit.x.clone().into())?
                    .extract_bits(LOG4_TREE_SIZE as usize * 2);

            let dst_tx_nonce_wit =
                AllocatedNum::alloc(&mut *cs, || Ok((trans.dst_before.tx_nonce as u64).into()))?;
            let dst_withdraw_nonce_wit = AllocatedNum::alloc(&mut *cs, || {
                Ok((trans.dst_before.withdraw_nonce as u64).into())
            })?;

            // Destination address doesn't necessarily need to reside on curve as it might be empty
            let dst_addr_wit = AllocatedPoint::alloc(&mut *cs, || Ok(trans.dst_before.address))?;

            let dst_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &dst_tx_nonce_wit.clone().into(),
                    &dst_withdraw_nonce_wit.clone().into(),
                    &dst_addr_wit.x.clone().into(),
                    &dst_addr_wit.y.clone().into(),
                    &dst_before_balances_hash.clone().into(),
                ],
            )?;
            let mut dst_proof_wits = Vec::new();
            for b in trans.dst_proof.0.clone() {
                dst_proof_wits.push([
                    AllocatedNum::alloc(&mut *cs, || Ok(b[0].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[1].into()))?,
                    AllocatedNum::alloc(&mut *cs, || Ok(b[2].into()))?,
                ]);
            }

            // Address of destination account slot can either be empty or equal with tx destination
            let is_dst_null = dst_addr_wit.is_null(&mut *cs)?;
            let is_dst_and_tx_dst_equal = dst_addr_wit.is_equal(&mut *cs, &tx_dst_addr_wit)?;
            let addr_valid = common::boolean_or(&mut *cs, &is_dst_null, &is_dst_and_tx_dst_equal)?;
            common::assert_true(&mut *cs, &addr_valid);

            // Check merkle proofs
            merkle::check_proof_poseidon4(
                &mut *cs,
                &enabled_wit,
                &tx_dst_index_wit.clone().into(),
                &dst_hash_wit,
                &dst_proof_wits,
                &middle_root_wit,
            )?;

            let new_dst_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &dst_tx_nonce_wit.clone().into(),
                    &dst_withdraw_nonce_wit.clone().into(),
                    &tx_dst_addr_wit.x.clone().into(),
                    &tx_dst_addr_wit.y.clone().into(),
                    &dst_balance_final_root,
                ],
            )?;

            // Calculate next-state hash and update state if tx is enabled
            let next_state_wit = merkle::calc_root_poseidon4(
                &mut *cs,
                &tx_dst_index_wit.clone().into(),
                &new_dst_hash_wit,
                &dst_proof_wits,
            )?;

            state_wit = common::mux(&mut *cs, &enabled_wit, &state_wit.into(), &next_state_wit)?;

            // tx amount+fee should be <= src balance
            let tx_balance_plus_fee_64 = UnsignedInteger::constrain(
                &mut *cs,
                Number::from(tx_amount_wit.clone()) + Number::from(tx_fee_wit.clone()),
                64,
            )?;
            let is_lte = tx_balance_plus_fee_64.lte(&mut *cs, &src_balance_wit)?;
            common::assert_true(&mut *cs, &is_lte);

            // Check tx nonce is equal with account nonce to prevent double spending
            Number::from(tx_nonce_wit.clone()).assert_equal_if_enabled(
                &mut *cs,
                &enabled_wit,
                &(Number::from(src_tx_nonce_wit.clone())
                    + Number::constant::<CS>(BellmanFr::one())),
            )?;

            // Fee is zero if transaction slot is empty, otherwise it equals to transaction fee
            // TODO: Check if fee token type is correct!
            let final_fee = common::mux(
                &mut *cs,
                &enabled_wit,
                &Number::zero(),
                &tx_fee_wit.clone().into(),
            )?;
            fee_sum.add_num(BellmanFr::one(), &final_fee);

            let tx_hash_wit = poseidon::poseidon(
                &mut *cs,
                &[
                    &tx_nonce_wit.clone().into(),
                    &tx_dst_addr_wit.x.clone().into(),
                    &tx_dst_addr_wit.y.clone().into(),
                    &tx_amount_token_id_wit.clone().into(),
                    &tx_amount_wit.clone().into(),
                    &tx_fee_token_id_wit.clone().into(),
                    &tx_fee_wit.clone().into(),
                ],
            )?;

            let tx_sig_r_wit = AllocatedPoint::alloc(&mut *cs, || Ok(trans.tx.sig.r))?;
            // Check if sig_r resides on curve
            tx_sig_r_wit.assert_on_curve(&mut *cs, &enabled_wit)?;

            let tx_sig_s_wit = AllocatedNum::alloc(&mut *cs, || Ok(trans.tx.sig.s.into()))?;

            // Check EdDSA signature
            eddsa::verify_eddsa(
                &mut *cs,
                &enabled_wit,
                &src_addr_wit,
                &tx_hash_wit,
                &tx_sig_r_wit,
                &tx_sig_s_wit,
            )?;
        }

        let fee_sum_and_token_hash = poseidon::poseidon(
            &mut *cs,
            &[&accepted_fee_token.clone().into(), &fee_sum.clone().into()],
        )?;

        // Check if sum of tx fees is equal with the feeded aux
        cs.enforce(
            || "",
            |lc| lc + aux_wit.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + fee_sum_and_token_hash.get_lc(),
        );

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
