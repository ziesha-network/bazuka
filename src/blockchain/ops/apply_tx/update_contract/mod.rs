mod deposit;
mod function_call;
mod mint;
mod withdraw;

use super::*;

pub fn update_contract<K: KvStore>(
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
        let commit = zk::ZkScalar::new(
            Hasher::hash(&bincode::serialize(&(update.prover.clone(), update.reward)).unwrap())
                .as_ref(),
        );

        // Pay prover reward from tx_src
        let mut src_bal = chain.get_balance(tx_src.clone(), ContractId::Ziesha)?;
        if src_bal < update.reward {
            return Err(BlockchainError::BalanceInsufficient);
        }
        src_bal -= update.reward;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, ContractId::Ziesha),
            src_bal.into(),
        )])?;
        let mut prover_bal = chain.get_balance(update.prover.clone(), ContractId::Ziesha)?;
        prover_bal += update.reward;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&update.prover, ContractId::Ziesha),
            prover_bal.into(),
        )])?;

        let (next_state, proof) = (update.next_state.clone(), update.proof.clone());
        let (circuit, aux_data) = match &update.data {
            ContractUpdateData::Deposit { deposits } => {
                let (circuit, aux_data) = deposit::deposit(
                    chain,
                    &contract_id,
                    &contract,
                    &update.circuit_id,
                    deposits,
                    &mut executor_fees,
                )?;
                (circuit, aux_data)
            }
            ContractUpdateData::Withdraw { withdraws } => {
                let (circuit, aux_data) = withdraw::withdraw(
                    chain,
                    &contract_id,
                    &contract,
                    &update.circuit_id,
                    withdraws,
                    &mut executor_fees,
                )?;
                (circuit, aux_data)
            }
            ContractUpdateData::FunctionCall { fee } => {
                let (circuit, aux_data) = function_call::function_call(
                    chain,
                    &contract_id,
                    &contract,
                    &update.circuit_id,
                    fee,
                    &mut executor_fees,
                )?;
                (circuit, aux_data)
            }
            ContractUpdateData::Mint { amount } => {
                let (circuit, aux_data) = mint::mint(
                    chain,
                    &contract_id,
                    &contract,
                    &update.circuit_id,
                    amount,
                    &mut executor_fees,
                )?;
                (circuit, aux_data)
            }
        };

        let mut cont_account = chain.get_contract_account(*contract_id)?;
        if !zk::check_proof(
            &circuit,
            commit,
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
