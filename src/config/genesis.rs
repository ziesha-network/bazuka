use crate::core::{Address, Block, Signature, Transaction, TransactionData};

pub fn get_genesis_block() -> Block {
    Block {
        header: Default::default(),
        body: vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: Address::Treasury,
                amount: 123,
            },
            nonce: 0,
            sig: Signature::Unsigned,
        }],
    }
}
