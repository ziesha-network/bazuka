use super::*;

pub fn rollback<K: KvStore>(chain: &mut KvStoreChain<K>) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        let height = chain.get_height()?;

        if height == 0 {
            return Err(BlockchainError::NoBlocksToRollback);
        }

        let rollback: Vec<WriteOp> = match chain.database.get(keys::rollback(height - 1))? {
            Some(b) => b.try_into()?,
            None => {
                return Err(BlockchainError::Inconsistency);
            }
        };

        let mut outdated = chain.get_outdated_contracts()?;
        let changed_states = chain.get_changed_states()?;

        for (cid, comp) in changed_states {
            if comp.prev_height == 0 {
                zk::KvStoreStateManager::<CoreZkHasher>::delete_contract(&mut chain.database, cid)?;
                outdated.retain(|&x| x != cid);
                continue;
            }

            if !outdated.contains(&cid) {
                let (ops, result) = chain.isolated(|fork| {
                    Ok(zk::KvStoreStateManager::<CoreZkHasher>::rollback_contract(
                        &mut fork.database,
                        cid,
                    )?)
                })?;

                if result != Some(comp.prev_state) {
                    outdated.push(cid);
                } else {
                    chain.database.update(&ops)?;
                }
            } else {
                let local_compressed_state =
                    zk::KvStoreStateManager::<CoreZkHasher>::root(&chain.database, cid)?;
                let local_height =
                    zk::KvStoreStateManager::<CoreZkHasher>::height_of(&chain.database, cid)?;
                if local_compressed_state == comp.prev_state && local_height == comp.prev_height {
                    outdated.retain(|&x| x != cid);
                }
            }
        }

        chain.database.update(&rollback)?;
        chain.database.update(&[
            WriteOp::Remove(keys::rollback(height - 1)),
            if outdated.is_empty() {
                WriteOp::Remove(keys::outdated())
            } else {
                WriteOp::Put(keys::outdated(), outdated.clone().into())
            },
        ])?;

        Ok(())
    })?;
    chain.database.update(&ops)?;
    Ok(())
}
