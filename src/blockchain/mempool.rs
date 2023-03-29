use super::{Blockchain, BlockchainError, TransactionStats};
<<<<<<< HEAD
use crate::core::{Account, Address, ChainSourcedTx, MpnAddress, MpnSourcedTx, TokenId};
=======
use crate::core::{Account, Address, Amount, ChainSourcedTx, MpnAddress, MpnSourcedTx, TokenId};
use crate::db::KvStore;
>>>>>>> master
use crate::zk;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct MpnAccountMempool {
    account: zk::MpnAccount,
    txs: VecDeque<(MpnSourcedTx, TransactionStats)>,
}

impl MpnAccountMempool {
    fn new(account: zk::MpnAccount) -> Self {
        Self {
            account,
            txs: Default::default(),
        }
    }
    fn len(&self) -> usize {
        self.txs.len()
    }
    fn first_nonce(&self) -> Option<u64> {
        self.txs.front().map(|(tx, _)| tx.nonce())
    }
    fn last_nonce(&self) -> Option<u64> {
        self.txs.back().map(|(tx, _)| tx.nonce())
    }
    fn applicable(&self, tx: &MpnSourcedTx) -> bool {
        if let Some(last_nonce) = self.last_nonce() {
            tx.nonce() == last_nonce + 1
        } else {
            self.account.nonce == tx.nonce()
        }
    }
    fn insert(&mut self, tx: MpnSourcedTx, stats: TransactionStats) {
        if self.applicable(&tx) {
            self.txs.push_back((tx, stats));
        }
    }
    fn update_account(&mut self, account: zk::MpnAccount) {
        while let Some(first_nonce) = self.first_nonce() {
            if first_nonce < account.nonce {
                self.txs.pop_front();
            } else {
                break;
            }
        }
        if self.first_nonce() != Some(account.nonce) {
            self.txs.clear();
        }
        self.account = account;
    }
}

#[derive(Debug, Clone)]
pub struct AccountMempool {
    account: Account,
    txs: VecDeque<(ChainSourcedTx, TransactionStats)>,
}

impl AccountMempool {
    fn new(account: Account) -> Self {
        Self {
            account,
            txs: Default::default(),
        }
    }
    fn len(&self) -> usize {
        self.txs.len()
    }
    fn first_tx(&self) -> Option<&(ChainSourcedTx, TransactionStats)> {
        self.txs.front()
    }
    fn first_nonce(&self) -> Option<u32> {
        self.txs.front().map(|(tx, _)| tx.nonce())
    }
    fn last_nonce(&self) -> Option<u32> {
        self.txs.back().map(|(tx, _)| tx.nonce())
    }
    fn applicable(&self, tx: &ChainSourcedTx) -> bool {
        if let Some(last_nonce) = self.last_nonce() {
            tx.nonce() == last_nonce + 1
        } else {
            self.account.nonce + 1 == tx.nonce()
        }
    }
    fn reset(&mut self, nonce: u32) {
        if nonce == 0 {
            self.txs.clear();
            return;
        }
        while let Some(last_nonce) = self.last_nonce() {
            if last_nonce > nonce - 1 {
                self.txs.pop_back();
            } else {
                break;
            }
        }
        if self.last_nonce() != Some(nonce - 1) {
            self.txs.clear();
        }
    }
    fn insert(&mut self, tx: ChainSourcedTx, stats: TransactionStats) {
        if self.applicable(&tx) {
            self.txs.push_back((tx, stats));
        }
    }
    fn update_account(&mut self, account: Account) {
        while let Some(first_nonce) = self.first_nonce() {
            if first_nonce <= account.nonce {
                self.txs.pop_front();
            } else {
                break;
            }
        }
        if self.first_nonce() != Some(account.nonce + 1) {
            self.txs.clear();
        }
        self.account = account;
    }
}

#[derive(Clone, Debug)]
pub struct Mempool {
    min_balance_per_tx: Amount,
    mpn_log4_account_capacity: u8,
    chain_sourced: HashMap<Address, AccountMempool>,
    rejected_chain_sourced: HashMap<ChainSourcedTx, TransactionStats>,
    mpn_sourced: HashMap<MpnAddress, MpnAccountMempool>,
    rejected_mpn_sourced: HashMap<MpnSourcedTx, TransactionStats>,
}

impl Mempool {
    pub fn new(mpn_log4_account_capacity: u8, min_balance_per_tx: Amount) -> Self {
        Self {
            mpn_log4_account_capacity,
            min_balance_per_tx,
            chain_sourced: Default::default(),
            rejected_chain_sourced: Default::default(),
            mpn_sourced: Default::default(),
            rejected_mpn_sourced: Default::default(),
        }
    }
}

impl Mempool {
    #[allow(dead_code)]
    pub fn refresh(
        &mut self,
        _blockchain: Box<dyn Blockchain>,
        _local_ts: u32,
        _max_time_alive: Option<u32>,
        _max_time_remember: Option<u32>,
    ) -> Result<(), BlockchainError> {
        Ok(())
    }
    pub fn chain_address_limit(&self, _addr: Address) -> usize {
        100
    }
    pub fn mpn_sourced_len(&self) -> usize {
        self.mpn_sourced.values().map(|c| c.len()).sum()
    }
    pub fn chain_sourced_len(&self) -> usize {
        self.chain_sourced.values().map(|c| c.len()).sum()
    }
    pub fn add_chain_sourced(
        &mut self,
        blockchain: &dyn Blockchain,
        tx: ChainSourcedTx,
        is_local: bool,
        now: u32,
    ) -> Result<(), BlockchainError> {
        if is_local {
            self.rejected_chain_sourced.remove(&tx);
        }
        if !self.rejected_chain_sourced.contains_key(&tx) {
            if !tx.verify_signature() {
                return Ok(());
            }
            let chain_acc = blockchain.get_account(tx.sender())?;
            if self
                .chain_sourced
                .get_mut(&tx.sender())
                .map(|all| {
                    all.update_account(chain_acc.clone());
                    if is_local && !all.applicable(&tx) {
                        all.reset(tx.nonce());
                    }
                    if let Some((first_tx, stats)) = all.first_tx() {
                        // TODO: config.replace_tx_threshold instead of 60
                        if now > stats.first_seen + 60 && first_tx != &tx {
                            log::info!(
                                "{} replaced its transaction on nonce {}",
                                tx.sender(),
                                tx.nonce()
                            );
                            all.reset(tx.nonce());
                        }
                    }
                    !all.applicable(&tx)
                })
                .unwrap_or_default()
            {
                return Ok(());
            }

            // Do not accept old txs in the mempool
            if tx.nonce() <= chain_acc.nonce {
                return Ok(());
            }

            let ziesha_balance = blockchain.get_balance(tx.sender(), TokenId::Ziesha)?;

            // Allow 1tx in mempool per Ziesha
            // Min: 1 Max: 1000
            let limit = std::cmp::max(
                std::cmp::min(
                    Into::<u64>::into(ziesha_balance) / self.min_balance_per_tx.0,
                    1000,
                ),
                1,
            ) as usize;

            let all = self
                .chain_sourced
                .entry(tx.sender().clone())
                .or_insert(AccountMempool::new(chain_acc));

            if is_local || all.len() < limit {
                all.insert(tx.clone(), TransactionStats::new(is_local, now));
            }
        }
        Ok(())
    }
    pub fn add_mpn_sourced(
        &mut self,
        blockchain: &dyn Blockchain,
        tx: MpnSourcedTx,
        is_local: bool,
        now: u32,
    ) -> Result<(), BlockchainError> {
        if is_local {
            self.rejected_mpn_sourced.remove(&tx);
        }
        if !self.rejected_mpn_sourced.contains_key(&tx) {
            let mpn_acc = blockchain
                .get_mpn_account(tx.sender().account_index(self.mpn_log4_account_capacity))?;
            if self
                .mpn_sourced
                .get_mut(&tx.sender())
                .map(|all| {
                    all.update_account(mpn_acc.clone());
                    !all.applicable(&tx)
                })
                .unwrap_or_default()
            {
                return Ok(());
            }

            // Do not accept txs from non-existing accounts
            if tx.sender().pub_key.0.decompress() != mpn_acc.address {
                return Ok(());
            }

            // Do not accept old txs in the mempool
            if tx.nonce() < mpn_acc.nonce {
                return Ok(());
            }

            // Do not accept txs coming from accounts that their 0th slot has no Ziesha
            if let Some(money) = mpn_acc.tokens.get(&0) {
                if money.token_id != TokenId::Ziesha {
                    return Ok(());
                }

                // Allow 1tx in mempool per Ziesha
                // Min: 1 Max: 1000
                let limit = std::cmp::max(
                    std::cmp::min(
                        Into::<u64>::into(money.amount) / self.min_balance_per_tx.0,
                        1000,
                    ),
                    1,
                ) as usize;

                let all = self
                    .mpn_sourced
                    .entry(tx.sender().clone())
                    .or_insert(MpnAccountMempool::new(mpn_acc));
                if tx.verify_signature() {
                    if is_local || all.len() < limit {
                        all.insert(tx.clone(), TransactionStats::new(is_local, now));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn chain_sourced(&self) -> impl Iterator<Item = &(ChainSourcedTx, TransactionStats)> {
        self.chain_sourced
            .iter()
            .map(|(_, c)| c.txs.iter())
            .flatten()
    }
    pub fn mpn_sourced(&self) -> impl Iterator<Item = &(MpnSourcedTx, TransactionStats)> {
        self.mpn_sourced.iter().map(|(_, c)| c.txs.iter()).flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::KvStoreChain;
    use crate::core::Money;
    use crate::db::RamKvStore;
    use crate::wallet::TxBuilder;

    fn dummy_tx(wallet: &TxBuilder, nonce: u32) -> ChainSourcedTx {
        ChainSourcedTx::TransactionAndDelta(wallet.create_transaction(
            "".into(),
            wallet.get_address(),
            Money::ziesha(200),
            Money::ziesha(0),
            nonce,
        ))
    }

    #[test]
    fn test_mempool_check_correct_account_nonce() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let abc = TxBuilder::new(&Vec::from("ABC"));

        for i in 0..5 {
            let mut mempool = Mempool::new(30, Amount(1));
            mempool
                .add_chain_sourced(&chain, dummy_tx(&abc, i), false, 0)
                .unwrap();

            let snapshot = mempool.chain_sourced().collect::<Vec<_>>();
            // Tx is only added if nonce is correct based on its account on the blockchain
            assert_eq!(snapshot.len(), if i == 1 { 1 } else { 0 });
        }
    }

    #[test]
    fn test_mempool_consecutive_nonces() {
        let chain = KvStoreChain::new(
            RamKvStore::new(),
            crate::config::blockchain::get_test_blockchain_config(),
        )
        .unwrap();
        let abc = TxBuilder::new(&Vec::from("ABC"));
        let other = TxBuilder::new(&Vec::from("DELEGATOR"));
        let mut mempool = Mempool::new(30, Amount(1));

        mempool
            .add_chain_sourced(&chain, dummy_tx(&abc, 1), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 1);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&abc, 2), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 2);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&abc, 4), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 2);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&abc, 3), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 3);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&abc, 4), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 4);

        mempool
            .add_chain_sourced(&chain, dummy_tx(&other, 1), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 4);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&other, 4), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 5);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&other, 6), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 5);
        mempool
            .add_chain_sourced(&chain, dummy_tx(&other, 5), false, 0)
            .unwrap();
        assert_eq!(mempool.chain_sourced().collect::<Vec<_>>().len(), 6);
    }
}
