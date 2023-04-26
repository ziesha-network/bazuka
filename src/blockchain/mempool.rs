use super::{Blockchain, BlockchainError, TransactionStats};
use crate::core::{
    Address, Amount, GeneralAddress, GeneralTransaction, MpnDeposit, MpnWithdraw, NonceGroup,
    Signature, TokenId, TransactionAndDelta,
};
use crate::db::KvStore;
use crate::zk::MpnTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// Allow transaction senders to commit on the time they submitted their transaction, as a
// solution for selecting the next tx from the sender in case there are txs with equal nonces.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TimestampCommit {
    pub timestamp: u32,
    pub sig: Signature,
}

trait Nonced {
    fn nonce(&self) -> u32;
}

impl Nonced for MpnTransaction {
    fn nonce(&self) -> u32 {
        self.nonce
    }
}

impl Nonced for MpnWithdraw {
    fn nonce(&self) -> u32 {
        self.zk_nonce
    }
}

impl Nonced for MpnDeposit {
    fn nonce(&self) -> u32 {
        self.payment.nonce
    }
}

impl Nonced for TransactionAndDelta {
    fn nonce(&self) -> u32 {
        self.tx.nonce
    }
}

#[derive(Debug, Clone)]
pub struct SingleMempool {
    nonce: u32,
    txs: VecDeque<(GeneralTransaction, TransactionStats)>,
}

impl SingleMempool {
    fn new(nonce: u32) -> Self {
        Self {
            nonce,
            txs: Default::default(),
        }
    }
    fn len(&self) -> usize {
        self.txs.len()
    }
    fn first_tx(&self) -> Option<&(GeneralTransaction, TransactionStats)> {
        self.txs.front()
    }
    fn first_nonce(&self) -> Option<u32> {
        self.first_tx().map(|(tx, _)| tx.nonce())
    }
    fn last_nonce(&self) -> Option<u32> {
        self.txs.back().map(|(tx, _)| tx.nonce())
    }
    fn applicable(&self, tx: &GeneralTransaction) -> bool {
        if let Some(last_nonce) = self.last_nonce() {
            tx.nonce() == last_nonce + 1
        } else {
            self.nonce + 1 == tx.nonce()
        }
    }
    fn insert(&mut self, tx: GeneralTransaction, stats: TransactionStats) {
        if self.applicable(&tx) {
            self.txs.push_back((tx, stats));
        }
    }
    fn update_nonce(&mut self, nonce: u32) {
        while let Some(first_nonce) = self.first_nonce() {
            if first_nonce <= nonce {
                self.txs.pop_front();
            } else {
                break;
            }
        }
        if self.first_nonce() != Some(nonce + 1) {
            self.txs.clear();
        }
        self.nonce = nonce;
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
}

#[derive(Clone, Debug)]
pub struct Mempool {
    min_balance_per_tx: Amount,
    txs: HashMap<NonceGroup, SingleMempool>,
    rejected: HashMap<GeneralTransaction, TransactionStats>,
}

impl Mempool {
    pub fn new(min_balance_per_tx: Amount) -> Self {
        Self {
            min_balance_per_tx,
            txs: Default::default(),
            rejected: Default::default(),
        }
    }
}

impl Mempool {
    #[allow(dead_code)]
    pub fn refresh<K: KvStore, B: Blockchain<K>>(
        &mut self,
        blockchain: &B,
        _local_ts: u32,
        _max_time_alive: Option<u32>,
        _max_time_remember: Option<u32>,
    ) -> Result<(), BlockchainError> {
        let mpn_contract_id = blockchain.config().mpn_config.mpn_contract_id;
        for (ng, mempool) in self.txs.iter_mut() {
            let nonce = match ng.clone() {
                NonceGroup::TransactionAndDelta(addr) => blockchain.get_nonce(addr)?,
                NonceGroup::MpnDeposit(addr) => {
                    blockchain.get_deposit_nonce(addr, mpn_contract_id)?
                }
                NonceGroup::MpnTransaction(addr) => blockchain.get_mpn_account(addr)?.tx_nonce,
                NonceGroup::MpnWithdraw(addr) => blockchain.get_mpn_account(addr)?.withdraw_nonce,
            };
            mempool.update_nonce(nonce);
        }
        Ok(())
    }
    pub fn chain_address_limit(&self, _addr: Address) -> usize {
        100
    }
    pub fn add_tx<K: KvStore, B: Blockchain<K>>(
        &mut self,
        blockchain: &B,
        tx: GeneralTransaction,
        is_local: bool,
        now: u32,
    ) -> Result<(), BlockchainError> {
        let mpn_contract_id = blockchain.config().mpn_config.mpn_contract_id;

        match &tx {
            GeneralTransaction::MpnDeposit(tx) => {
                if tx.payment.contract_id != mpn_contract_id || tx.payment.deposit_circuit_id != 0 {
                    return Ok(());
                }
            }
            GeneralTransaction::MpnWithdraw(tx) => {
                if tx.payment.contract_id != mpn_contract_id || tx.payment.withdraw_circuit_id != 0
                {
                    return Ok(());
                }
            }
            _ => {}
        }

        if is_local {
            self.rejected.remove(&tx);
        }
        if self.rejected.contains_key(&tx) || !tx.verify_signature() {
            return Ok(());
        }
        let nonce = match tx.nonce_group() {
            NonceGroup::TransactionAndDelta(addr) => blockchain.get_nonce(addr)?,
            NonceGroup::MpnDeposit(addr) => blockchain.get_deposit_nonce(addr, mpn_contract_id)?,
            NonceGroup::MpnTransaction(addr) => blockchain.get_mpn_account(addr)?.tx_nonce,
            NonceGroup::MpnWithdraw(addr) => blockchain.get_mpn_account(addr)?.withdraw_nonce,
        };
        if self
            .txs
            .get_mut(&tx.nonce_group())
            .map(|all| {
                all.update_nonce(nonce);
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
        if tx.nonce() <= nonce {
            return Ok(());
        }

        let ziesha_balance = match tx.sender() {
            GeneralAddress::ChainAddress(addr) => blockchain.get_balance(addr, TokenId::Ziesha)?,
            GeneralAddress::MpnAddress(mpn_addr) => {
                let acc = blockchain.get_mpn_account(mpn_addr)?;
                acc.tokens
                    .get(&0)
                    .map(|m| {
                        if m.token_id == TokenId::Ziesha {
                            m.amount
                        } else {
                            0.into()
                        }
                    })
                    .unwrap_or_default()
            }
        };

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
            .txs
            .entry(tx.nonce_group().clone())
            .or_insert(SingleMempool::new(nonce));

        if is_local || all.len() < limit {
            all.insert(tx.clone(), TransactionStats::new(is_local, now));
        }
        Ok(())
    }
    pub fn all(&self) -> impl Iterator<Item = &(GeneralTransaction, TransactionStats)> {
        self.txs.iter().map(|(_, c)| c.txs.iter()).flatten()
    }
    pub fn tx_deltas(&self) -> impl Iterator<Item = (&TransactionAndDelta, &TransactionStats)> {
        self.txs
            .iter()
            .filter(|(n, _)| match n {
                NonceGroup::TransactionAndDelta(_) => true,
                _ => false,
            })
            .map(|(_, c)| {
                c.txs.iter().filter_map(|(t, s)| {
                    if let GeneralTransaction::TransactionAndDelta(tx) = t {
                        Some((tx, s))
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
    pub fn mpn_deposits(&self) -> impl Iterator<Item = (&MpnDeposit, &TransactionStats)> {
        self.txs
            .iter()
            .filter(|(n, _)| match n {
                NonceGroup::MpnDeposit(_) => true,
                _ => false,
            })
            .map(|(_, c)| {
                c.txs.iter().filter_map(|(t, s)| {
                    if let GeneralTransaction::MpnDeposit(tx) = t {
                        Some((tx, s))
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
    pub fn mpn_withdraws(&self) -> impl Iterator<Item = (&MpnWithdraw, &TransactionStats)> {
        self.txs
            .iter()
            .filter(|(n, _)| match n {
                NonceGroup::MpnWithdraw(_) => true,
                _ => false,
            })
            .map(|(_, c)| {
                c.txs.iter().filter_map(|(t, s)| {
                    if let GeneralTransaction::MpnWithdraw(tx) = t {
                        Some((tx, s))
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
    pub fn mpn_txs(&self) -> impl Iterator<Item = (&MpnTransaction, &TransactionStats)> {
        self.txs
            .iter()
            .filter(|(n, _)| match n {
                NonceGroup::MpnTransaction(_) => true,
                _ => false,
            })
            .map(|(_, c)| {
                c.txs.iter().filter_map(|(t, s)| {
                    if let GeneralTransaction::MpnTransaction(tx) = t {
                        Some((tx, s))
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
    pub fn len(&self) -> usize {
        self.txs.values().map(|c| c.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::KvStoreChain;
    use crate::core::Money;
    use crate::db::RamKvStore;
    use crate::wallet::TxBuilder;

    fn dummy_tx(wallet: &TxBuilder, nonce: u32) -> GeneralTransaction {
        GeneralTransaction::TransactionAndDelta(wallet.create_transaction(
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
            let mut mempool = Mempool::new(Amount(1));
            mempool.add_tx(&chain, dummy_tx(&abc, i), false, 0).unwrap();

            let snapshot = mempool.all().collect::<Vec<_>>();
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
        let mut mempool = Mempool::new(Amount(1));

        mempool.add_tx(&chain, dummy_tx(&abc, 1), false, 0).unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 1);
        mempool.add_tx(&chain, dummy_tx(&abc, 2), false, 0).unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 2);
        mempool.add_tx(&chain, dummy_tx(&abc, 4), false, 0).unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 2);
        mempool.add_tx(&chain, dummy_tx(&abc, 3), false, 0).unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 3);
        mempool.add_tx(&chain, dummy_tx(&abc, 4), false, 0).unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 4);

        mempool
            .add_tx(&chain, dummy_tx(&other, 10), false, 0)
            .unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 4);
        mempool
            .add_tx(&chain, dummy_tx(&other, 1), false, 0)
            .unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 5);
        mempool
            .add_tx(&chain, dummy_tx(&other, 3), false, 0)
            .unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 5);
        mempool
            .add_tx(&chain, dummy_tx(&other, 2), false, 0)
            .unwrap();
        assert_eq!(mempool.all().collect::<Vec<_>>().len(), 6);
    }
}
