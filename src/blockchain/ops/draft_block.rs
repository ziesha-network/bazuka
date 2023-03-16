use super::*;

pub fn draft_block<K: KvStore>(
    chain: &KvStoreChain<K>,
    timestamp: u32,
    mempool: &[TransactionAndDelta],
    wallet: &TxBuilder,
    check: bool,
) -> Result<Option<BlockAndPatch>, BlockchainError> {
    let height = chain.get_height()?;

    let validator_status = chain.validator_status(timestamp, wallet)?;
    if chain.config.check_validator && validator_status.is_unproven() {
        return Ok(None);
    }

    let outdated_contracts = chain.get_outdated_contracts()?;

    if !outdated_contracts.is_empty() {
        return Err(BlockchainError::StatesOutdated);
    }

    let last_header = chain.get_header(height - 1)?;

    let tx_and_deltas = chain.select_transactions(wallet.get_address(), mempool, check)?;

    let mut txs = Vec::new();

    let mut block_delta: HashMap<ContractId, zk::ZkStatePatch> = HashMap::new();
    for tx_delta in tx_and_deltas.iter() {
        if let Some(contract_id) = match &tx_delta.tx.data {
            TransactionData::CreateContract { .. } => Some(ContractId::new(&tx_delta.tx)),
            TransactionData::UpdateContract { contract_id, .. } => Some(*contract_id),
            _ => None,
        } {
            block_delta.insert(
                contract_id,
                zk::ZkStatePatch::Delta(
                    tx_delta
                        .state_delta
                        .clone()
                        .ok_or(BlockchainError::FullStateNotFound)?,
                ),
            );
        }
    }
    let block_delta = ZkBlockchainPatch {
        patches: block_delta,
    };

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
        chain.update_states(&block_delta)?;

        Ok(())
    }) {
        Err(BlockchainError::InsufficientMpnUpdates) => Ok(None),
        Err(e) => Err(e),
        Ok(_) => Ok(Some(BlockAndPatch {
            block: blk,
            patch: block_delta,
        })),
    }
}
