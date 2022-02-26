use crate::core::{Address, Block, Signature, Transaction, TransactionData};

pub fn get_genesis_block() -> Block {
    Block {
        header: Default::default(),
        body: vec![Transaction {
            src: Address::Nowhere,
            data: TransactionData::RegularSend {
                dst: Address::Nowhere,
                amount: 123,
            },
            sig: Signature::Unsigned,
        }],
    }
}
