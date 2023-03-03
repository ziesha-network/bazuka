use crate::core::TokenId;

use super::{get_wallet, get_wallet_path};

pub fn add_token(token_id: TokenId) -> () {
    let wallet = get_wallet();
    let mut wallet = wallet.expect("Bazuka is not initialized!");
    wallet.add_token(token_id);
    wallet.save(get_wallet_path()).unwrap();
}
