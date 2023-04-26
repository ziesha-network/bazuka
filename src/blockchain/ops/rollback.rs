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

        chain.database.update(&rollback)?;
        chain
            .database
            .update(&[WriteOp::Remove(keys::rollback(height - 1))])?;

        Ok(())
    })?;
    chain.database.update(&ops)?;
    Ok(())
}
