use crate::core::{Address, Block, Header, ProofOfWork, Signature, Transaction, TransactionData};

pub fn get_genesis_block() -> Block {
    let mut blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            state_root: Default::default(),
            block_root: Default::default(),
            proof_of_work: ProofOfWork {
                timestamp: 0,
                target: 0x02ffffff,
                nonce: 0,
            },
        },
        body: vec![Transaction {
            src: Address::Treasury,
            data: TransactionData::RegularSend {
                dst: "0x215d9af3a1bfa2a87929b6e8265e95c61c36f91493f3dbd702215255f68742552"
                    .parse()
                    .unwrap(),
                amount: 123,
            },
            nonce: 1,
            fee: 0,
            sig: Signature::Unsigned,
        }],
    };
    blk.header.block_root = blk.merkle_tree().root();
    blk
}

pub fn get_test_genesis_block() -> Block {
    let mut blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            state_root: Default::default(),
            block_root: Default::default(),
            proof_of_work: ProofOfWork {
                timestamp: 0,
                target: 0x007fffff,
                nonce: 0,
            },
        },
        body: vec![],
    };
    blk.header.block_root = blk.merkle_tree().root();
    blk
}
