use std::path::PathBuf;

use crate::{core::TokenId, wallet::WalletCollection};

pub fn add_token(token_id: TokenId, wallet: &mut WalletCollection, wallet_path: &PathBuf) -> () {
    wallet.user(0).add_token(token_id);
    wallet.save(wallet_path).unwrap();
}
