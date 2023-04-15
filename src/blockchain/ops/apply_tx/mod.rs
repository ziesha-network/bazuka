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

pub fn apply_tx<K: KvStore>(
    chain: &mut KvStoreChain<K>,
    tx: &Transaction,
    internal: bool,
) -> Result<TxSideEffect, BlockchainError> {
    let (ops, side_effect) = chain.isolated(|chain| {
        let mut side_effect = TxSideEffect::Nothing;

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
