use super::*;

pub fn draft_block<K: KvStore>(
    chain: &KvStoreChain<K>,
    timestamp: u32,
    mempool: &[TransactionAndDelta],
    wallet: &TxBuilder,
    check: bool,
) -> Result<Option<Block>, BlockchainError> {
    let height = chain.get_height()?;
    if height == 0 {
        return Err(BlockchainError::BlockchainEmpty);
    }

    let validator_status = chain.validator_status(timestamp, wallet)?;
    if chain.config.check_validator && validator_status.is_unproven() {
        return Ok(None);
    }

    let last_header = chain.get_header(height - 1)?;
    let tx_and_deltas = chain.select_transactions(wallet.get_address(), mempool, check)?;

    let mut txs = Vec::new();
    txs.extend(tx_and_deltas.iter().map(|tp| tp.tx.clone()));

    let mut blk = Block {
        header: Header {
            parent_hash: last_header.hash(),
            number: height as u64,
            block_root: Default::default(),
            proof_of_stake: ProofOfStake {
                timestamp,
                validator: wallet.get_address(),
                proof: validator_status,
            },
        },
        body: txs,
    };
    blk.header.block_root = blk.merkle_tree().root();

    match chain.isolated(|chain| {
        chain.apply_block(&blk)?; // Check if everything is ok
        Ok(())
    }) {
        Err(BlockchainError::InsufficientMpnUpdates) => Ok(None),
        Err(e) => Err(e),
        Ok(_) => Ok(Some(blk)),
    }
}
