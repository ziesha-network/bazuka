use crate::core::{Address, Block, Transaction};

pub fn get_genesis_block() -> Block {
    Block {
        header: Default::default(),
        body: vec![Transaction::RegularSend {
            src: Address::Nowhere,
            dst: Address::PublicKey(123),
            amount: 123,
            sig: 0,
        }],
    }
}
