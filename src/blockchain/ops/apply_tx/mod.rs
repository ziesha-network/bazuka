mod auto_delegate;
mod create_contract;
mod create_token;
mod delegate;
mod regular_send;
mod undelegate;
mod update_contract;
mod update_staker;
mod update_token;

use super::*;

use crate::crypto::jubjub;
use crate::zk::ZkScalar;

pub fn index_mpn_accounts<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    delta: &zk::ZkDeltaPairs,
) -> Result<(), BlockchainError> {
    let mut acc_count = chain.get_mpn_account_count()?;
    let mut org = HashMap::<u64, HashMap<u64, ZkScalar>>::new();
    for (k, v) in delta.0.iter() {
        if k.0.len() == 2 && (k.0[1] == 2 || k.0[1] == 3) {
            org.entry(k.0[0])
                .or_default()
                .entry(k.0[1])
                .or_insert(v.unwrap_or_default());
        }
    }
    for (index, data) in org.iter() {
        let x = data.get(&2).ok_or(BlockchainError::Inconsistency)?;
        let y = data.get(&3).ok_or(BlockchainError::Inconsistency)?;
        let address = MpnAddress {
            pub_key: jubjub::PublicKey(jubjub::PointAffine(*x, *y).compress()),
        };
        chain.database.update(&[WriteOp::Put(
            keys::MpnAccountIndexDbKey {
                address,
                index: *index,
            }
            .into(),
            ().into(),
        )])?;
    }
    let mut inds = org.keys().cloned().collect::<Vec<_>>();
    inds.sort();
    for ind in inds {
        if ind as u64 == acc_count {
            acc_count += 1;
        } else if ind as u64 > acc_count {
            return Err(BlockchainError::Inconsistency);
        }
    }
    chain
        .database
        .update(&[WriteOp::Put(keys::mpn_account_count(), acc_count.into())])?;
    Ok(())
}

pub fn apply_tx<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx: &Transaction,
    internal: bool,
) -> Result<(), BlockchainError> {
    let (ops, _) = chain.isolated(|chain| {
        if tx.src == None && !internal {
            return Err(BlockchainError::IllegalTreasuryAccess);
        }

        if tx.fee.token_id != TokenId::Ziesha {
            return Err(BlockchainError::OnlyZieshaFeesAccepted);
        }

        if tx.memo.len() > chain.config.max_memo_length {
            return Err(BlockchainError::MemoTooLong);
        }

        let tx_src = tx.src.clone().unwrap_or_default(); // Default is treasury account!

        let mut acc_nonce = chain.get_nonce(tx_src.clone())?;
        let mut acc_bal = chain.get_balance(tx_src.clone(), tx.fee.token_id)?;

        if (!internal && tx.nonce != acc_nonce + 1) || (internal && tx.nonce != 0) {
            return Err(BlockchainError::InvalidTransactionNonce);
        }

        if acc_bal < tx.fee.amount {
            return Err(BlockchainError::BalanceInsufficient);
        }

        if !internal {
            acc_nonce += 1;
            chain
                .database
                .update(&[WriteOp::Put(keys::nonce(&tx_src), acc_nonce.into())])?;
        }

        acc_bal -= tx.fee.amount;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, tx.fee.token_id),
            acc_bal.into(),
        )])?;

        match &tx.data {
            TransactionData::UpdateStaker {
                vrf_pub_key,
                commission,
            } => {
                update_staker::update_staker(chain, tx_src, vrf_pub_key.clone(), *commission)?;
            }
            TransactionData::Delegate { amount, to } => {
                delegate::delegate(chain, tx_src, *amount, to.clone())?;
            }
            TransactionData::AutoDelegate { to, ratio } => {
                auto_delegate::auto_delegate(chain, tx_src, to.clone(), *ratio)?;
            }
            TransactionData::Undelegate { amount, from } => {
                let undelegation_id = UndelegationId::new(tx);
                undelegate::undelegate(chain, undelegation_id, tx_src, *amount, from.clone())?;
            }
            TransactionData::CreateToken { token } => {
                let token_id = {
                    let tid = TokenId::new(tx);
                    if tid == chain.config.ziesha_token_id {
                        TokenId::Ziesha
                    } else {
                        tid
                    }
                };
                create_token::create_token(chain, tx_src, token_id, token)?;
            }
            TransactionData::UpdateToken { token_id, update } => {
                update_token::update_token(chain, tx_src, token_id, update)?;
            }
            TransactionData::RegularSend { entries } => {
                regular_send::regular_send(chain, tx_src, entries)?;
            }
            TransactionData::CreateContract { contract, state } => {
                let contract_id = ContractId::new(tx);
                create_contract::create_contract(chain, contract_id, contract, state)?;
            }
            TransactionData::UpdateContract {
                contract_id,
                updates,
                delta,
            } => {
                update_contract::update_contract(
                    internal,
                    chain,
                    tx_src,
                    contract_id,
                    updates,
                    delta,
                )?;
            }
        }

        // Fees go to the Treasury account first
        if tx.src != None {
            let mut treasury_balance = chain.get_balance(Default::default(), tx.fee.token_id)?;
            treasury_balance += tx.fee.amount;
            chain.database.update(&[WriteOp::Put(
                keys::account_balance(&Default::default(), tx.fee.token_id),
                treasury_balance.into(),
            )])?;
        }

        Ok(())
    })?;

    chain.database.update(&ops)?;
    Ok(())
}
