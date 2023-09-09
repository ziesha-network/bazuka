use crate::cli::BazukaConfig;
use bazuka::blockchain::Blockchain;
use bazuka::db::KvStore;
use bazuka::{
    blockchain::KvStoreChain,
    core::{Amount, ContractId},
    db::ReadOnlyLevelDbKvStore,
};
use colored::Colorize;

pub fn health_check(conf: &BazukaConfig) {
    let mpn_contract_id = bazuka::config::blockchain::get_blockchain_config()
        .mpn_config
        .mpn_contract_id;
    let rdb = ReadOnlyLevelDbKvStore::read_only(&conf.db, 64).unwrap();
    let db = rdb.snapshot();
    let chain = KvStoreChain::new(db, bazuka::config::blockchain::get_blockchain_config()).unwrap();
    let mut fork = chain.fork_on_ram();
    while fork.get_height().unwrap() != 0 {
        fork.rollback().unwrap();
    }
    let rollback_validity_check = fork
        .db()
        .pairs("".into())
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>()
        .is_empty();
    let acc_count = chain.get_mpn_account_count().unwrap();
    let mut sum_mpn: Amount = 0.into();
    for mpn_acc in chain.get_mpn_accounts(0, acc_count as usize).unwrap() {
        for money in mpn_acc.1.tokens.values() {
            if money.token_id == ContractId::Ziesha {
                sum_mpn += money.amount;
            }
        }
    }

    let expected_currency_in_circulation = Amount::from(2000000000000000000);

    let mpn_contract_balance_check = sum_mpn
        == chain
            .get_contract_balance(mpn_contract_id, ContractId::Ziesha)
            .unwrap();
    let currency_in_circulation_check =
        chain.currency_in_circulation().unwrap() == expected_currency_in_circulation;
    println!(
        "Rollback validity check: {}",
        if rollback_validity_check {
            "PASS".bright_green()
        } else {
            "FAIL".bright_red()
        }
    );
    println!(
        "MPN contract balance check: {}",
        if mpn_contract_balance_check {
            "PASS".bright_green()
        } else {
            "FAIL".bright_red()
        }
    );
    println!(
        "Currency in circulation check: {}",
        if currency_in_circulation_check {
            "PASS".bright_green()
        } else {
            "FAIL".bright_red()
        }
    );
}
