use super::*;

pub fn generate_state_patch<K: KvStore>(
    chain: &KvStoreChain<K>,
    heights: HashMap<ContractId, u64>,
    to: <Hasher as Hash>::Output,
) -> Result<ZkBlockchainPatch, BlockchainError> {
    let height = chain.get_height()?;
    let last_header = chain.get_header(height - 1)?;

    if last_header.hash() != to {
        return Err(BlockchainError::StatesUnavailable);
    }

    let outdated_contracts = chain.get_outdated_contracts()?;

    let mut blockchain_patch = ZkBlockchainPatch {
        patches: HashMap::new(),
    };
    for (cid, height) in heights {
        if !outdated_contracts.contains(&cid) {
            let local_height =
                zk::KvStoreStateManager::<CoreZkHasher>::height_of(&chain.database, cid)?;
            if height > local_height {
                return Err(BlockchainError::StatesUnavailable);
            }
            let away = local_height - height;
            blockchain_patch.patches.insert(
                cid,
                if let Some(delta) =
                    zk::KvStoreStateManager::<CoreZkHasher>::delta_of(&chain.database, cid, away)?
                {
                    zk::ZkStatePatch::Delta(delta)
                } else {
                    zk::ZkStatePatch::Full(zk::KvStoreStateManager::<CoreZkHasher>::get_full_state(
                        &chain.database,
                        cid,
                    )?)
                },
            );
        }
    }

    Ok(blockchain_patch)
}
