use crate::core::{Address, Block, Signature, Transaction};

pub fn get_genesis_block() -> Block {
    Block {
        header: Default::default(),
        body: vec![Transaction::RegularSend {
            src: Address::Nowhere,
            dst: Address::Nowhere,
            amount: 123,
            sig: Signature::Unsigned,
        }],
    }
}
