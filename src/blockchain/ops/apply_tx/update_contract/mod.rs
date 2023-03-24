mod deposit;
mod function_call;
mod withdraw;

use super::*;

pub fn update_contract<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx_src: Address,
    contract_id: &ContractId,
    updates: &[ContractUpdate],
) -> Result<TxSideEffect, BlockchainError> {
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
            &cont_account.compressed_state,
            &aux_data,
            &next_state,
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

    chain.database.update(&[WriteOp::Put(
        keys::compressed_state_at(contract_id, cont_account.height),
        cont_account.compressed_state.into(),
    )])?;
    Ok(TxSideEffect::StateChange {
        contract_id: *contract_id,
        state_change: ZkCompressedStateChange {
            prev_height: prev_account.height,
            prev_state: prev_account.compressed_state,
            state: cont_account.compressed_state,
        },
    })
}
