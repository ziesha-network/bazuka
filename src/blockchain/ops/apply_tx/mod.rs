mod create_contract;
mod create_token;
mod delegate;
mod regular_send;
mod update_contract;
mod update_staker;
mod update_token;

use super::*;

pub fn apply_tx<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx: &Transaction,
    allow_treasury: bool,
) -> Result<TxSideEffect, BlockchainError> {
    let (ops, side_effect) = chain.isolated(|chain| {
        let mut side_effect = TxSideEffect::Nothing;

        if tx.src == None && !allow_treasury {
            return Err(BlockchainError::IllegalTreasuryAccess);
        }

        if tx.fee.token_id != TokenId::Ziesha {
            return Err(BlockchainError::OnlyZieshaFeesAccepted);
        }

        if tx.memo.len() > chain.config.max_memo_length {
            return Err(BlockchainError::MemoTooLong);
        }

        let tx_src = tx.src.clone().unwrap_or_default(); // Default is treasury account!

        let mut acc_src = chain.get_account(tx_src.clone())?;
        let mut acc_bal = chain.get_balance(tx_src.clone(), tx.fee.token_id)?;

        if tx.nonce != acc_src.nonce + 1 {
            return Err(BlockchainError::InvalidTransactionNonce);
        }

        if acc_bal < tx.fee.amount {
            return Err(BlockchainError::BalanceInsufficient);
        }

        acc_bal -= tx.fee.amount;
        acc_src.nonce += 1;

        chain
            .database
            .update(&[WriteOp::Put(keys::account(&tx_src), acc_src.into())])?;
        chain.database.update(&[WriteOp::Put(
            keys::account_balance(&tx_src, tx.fee.token_id),
            acc_bal.into(),
        )])?;

        match &tx.data {
            TransactionData::UpdateStaker {
                vrf_pub_key,
                commision,
            } => {
                update_staker::update_staker(chain, tx_src, vrf_pub_key.clone(), *commision)?;
            }
            TransactionData::Delegate {
                amount,
                to,
                reverse,
            } => {
                delegate::delegate(chain, tx_src, *amount, to.clone(), *reverse)?;
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
            TransactionData::CreateContract { contract } => {
                let contract_id = ContractId::new(tx);
                side_effect = create_contract::create_contract(chain, contract_id, contract)?;
            }
            TransactionData::UpdateContract {
                contract_id,
                updates,
            } => {
                side_effect =
                    update_contract::update_contract(chain, tx_src, contract_id, updates)?;
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

        Ok(side_effect)
    })?;

    chain.database.update(&ops)?;
    Ok(side_effect)
}
