mod deposit;
mod function_call;
mod withdraw;

use super::*;
use crate::crypto::jubjub;
use crate::zk::ZkScalar;

pub fn index_mpn_accounts<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    delta: &zk::ZkDeltaPairs,
) -> Result<(), BlockchainError> {
    let mut acc_count = chain.get_mpn_account_count()?;
    let mut org = HashMap::<u64, HashMap<u64, ZkScalar>>::new();
    for (k, v) in delta.0.iter() {
        if k.0.len() == 2 && (k.0[1] == 2 || k.0[1] == 3) {
            org.entry(k.0[0])
                .or_default()
                .entry(k.0[1])
                .or_insert(v.unwrap_or_default());
        }
    }
    for (index, data) in org.iter() {
        let x = data.get(&2).ok_or(BlockchainError::Inconsistency)?;
        let y = data.get(&3).ok_or(BlockchainError::Inconsistency)?;
        let address = MpnAddress {
            pub_key: jubjub::PublicKey(jubjub::PointAffine(*x, *y).compress()),
        };
        chain.database.update(&[WriteOp::Put(
            keys::MpnAccountIndexDbKey {
                address,
                index: *index,
            }
            .into(),
            ().into(),
        )])?;
    }
    let mut inds = org.keys().cloned().collect::<Vec<_>>();
    inds.sort();
    for ind in inds {
        if ind as u64 == acc_count {
            acc_count += 1;
        } else if ind as u64 > acc_count {
            return Err(BlockchainError::Inconsistency);
        }
    }
    chain
        .database
        .update(&[WriteOp::Put(keys::mpn_account_count(), acc_count.into())])?;
    Ok(())
}

pub fn update_contract<K: KvStore>(
    internal: bool,
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    contract_id: &ContractId,
    updates: &[ContractUpdate],
    delta: &Option<zk::ZkDeltaPairs>,
) -> Result<(), BlockchainError> {
    let contract = chain.get_contract(*contract_id)?;
    let mut executor_fees = Vec::new();

    let prev_account = chain.get_contract_account(*contract_id)?;

    // Increase height
    let mut cont_account = chain.get_contract_account(*contract_id)?;
    cont_account.height += 1;
    chain.database.update(&[WriteOp::Put(
        keys::contract_account(contract_id),
        cont_account.into(),
    )])?;

    for update in updates {
        let (circuit, aux_data, next_state, proof) = match update {
            ContractUpdate::Deposit {
                deposit_circuit_id,
                deposits,
                next_state,
                proof,
            } => {
                let (circuit, aux_data) = deposit::deposit(
                    chain,
                    &contract_id,
                    &contract,
                    deposit_circuit_id,
                    deposits,
                    &mut executor_fees,
                )?;
                (circuit, aux_data, next_state.clone(), proof.clone())
            }
            ContractUpdate::Withdraw {
                withdraw_circuit_id,
                withdraws,
                next_state,
                proof,
            } => {
                let (circuit, aux_data) = withdraw::withdraw(
                    chain,
                    &contract_id,
                    &contract,
                    withdraw_circuit_id,
                    withdraws,
                    &mut executor_fees,
                )?;
                (circuit, aux_data, next_state.clone(), proof.clone())
            }
            ContractUpdate::FunctionCall {
                function_id,
                next_state,
                proof,
                fee,
            } => {
                let (circuit, aux_data) = function_call::function_call(
                    chain,
                    &contract_id,
                    &contract,
                    function_id,
                    fee,
                    &mut executor_fees,
                )?;
                (circuit, aux_data, next_state.clone(), proof.clone())
            }
        };

        let mut cont_account = chain.get_contract_account(*contract_id)?;
        if !zk::check_proof(
            &circuit,
            prev_account.height,
            cont_account.compressed_state.state_hash,
            aux_data.state_hash,
            next_state.state_hash,
            &proof,
        ) {
            return Err(BlockchainError::IncorrectZkProof);
        }
        cont_account.compressed_state = next_state;
        chain.database.update(&[WriteOp::Put(
            keys::contract_account(contract_id),
            cont_account.into(),
        )])?;
    }

    for fee in executor_fees {
        let mut acc_bal = chain.get_balance(tx_src.clone(), fee.token_id)?;
        acc_bal += fee.amount;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, fee.token_id),
            acc_bal.into(),
        )])?;
    }

    let cont_account = chain.get_contract_account(*contract_id)?;

    let delta = delta.clone().ok_or(BlockchainError::StateNotGiven)?;
    if *contract_id == chain.config().mpn_config.mpn_contract_id {
        index_mpn_accounts(chain, &delta)?;
    }

    zk::KvStoreStateManager::<CoreZkHasher>::update_contract(
        &mut chain.database,
        *contract_id,
        &delta,
        cont_account.height,
    )?;
    if zk::KvStoreStateManager::<CoreZkHasher>::root(&mut chain.database, *contract_id)?
        != cont_account.compressed_state
    {
        return Err(BlockchainError::InvalidState);
    }
    Ok(())
}
