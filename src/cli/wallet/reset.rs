use crate::cli::{get_wallet, get_wallet_path};

pub fn reset() -> () {
    let wallet = get_wallet();
    let wallet_path = get_wallet_path();
    let mut wallet = wallet.expect("Bazuka is not initialized!");
    wallet.reset();
    wallet.save(wallet_path).unwrap();
}
