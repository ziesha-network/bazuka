use super::*;

pub fn update_states<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    patch: &ZkBlockchainPatch,
) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        let mut outdated_contracts = chain.get_outdated_contracts()?;

        for cid in outdated_contracts.clone() {
            let contract_account = chain.get_contract_account(cid)?;
            let patch = patch
                .patches
                .get(&cid)
                .ok_or(BlockchainError::FullStateNotFound)?;
            match &patch {
                zk::ZkStatePatch::Full(full) => {
                    let mut expected_delta_targets = Vec::new();
                    for i in 0..full.rollbacks.len() {
                        expected_delta_targets.push(chain.get_compressed_state_at(
                            cid,
                            contract_account.height - 1 - i as u64,
                        )?);
                    }
                    zk::KvStoreStateManager::<CoreZkHasher>::reset_contract(
                        &mut chain.database,
                        cid,
                        contract_account.height,
                        full,
                        &expected_delta_targets,
                    )?;
                }
                zk::ZkStatePatch::Delta(delta) => {
                    zk::KvStoreStateManager::<CoreZkHasher>::update_contract(
                        &mut chain.database,
                        cid,
                        delta,
                        contract_account.height,
                    )?;
                }
            };

            if zk::KvStoreStateManager::<CoreZkHasher>::root(&chain.database, cid)?
                != contract_account.compressed_state
            {
                return Err(BlockchainError::FullStateNotValid);
            }
            outdated_contracts.retain(|&x| x != cid);
        }

        chain.database.update(&[if outdated_contracts.is_empty() {
            WriteOp::Remove(keys::outdated())
        } else {
            WriteOp::Put(keys::outdated(), outdated_contracts.clone().into())
        }])?;

        Ok(())
    })?;
    chain.database.update(&ops)?;
    Ok(())
}
