use crate::core::{Address, Block, Signature, Transaction, TransactionData};

pub fn get_genesis_block() -> Block {
    Block {
        header: Default::default(),
        body: vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: "0x215d9af3a1bfa2a87929b6e8265e95c61c36f91493f3dbd702215255f68742552"
                    .parse()
                    .unwrap(),
                amount: 123,
            },
            nonce: 0,
            fee: 0,
            sig: Signature::Unsigned,
        }],
    }
}
