use std::path::PathBuf;

use bazuka::wallet::WalletCollection;

pub fn reset(mut wallet: WalletCollection, wallet_path: &PathBuf) -> () {
    wallet.user(0).reset();
    wallet.save(wallet_path).unwrap();
}
