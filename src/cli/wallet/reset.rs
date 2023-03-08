use std::path::PathBuf;

use crate::wallet::Wallet;

pub fn reset(wallet: &mut Wallet, wallet_path: &PathBuf) -> () {
    wallet.reset();
    wallet.save(wallet_path).unwrap();
}
