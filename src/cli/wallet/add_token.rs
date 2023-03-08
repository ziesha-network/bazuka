use std::path::PathBuf;

use crate::{core::TokenId, wallet::Wallet};

pub fn add_token(token_id: TokenId, wallet: &mut Wallet, wallet_path: &PathBuf) -> () {
    wallet.add_token(token_id);
    wallet.save(wallet_path).unwrap();
}
