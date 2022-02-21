use crate::core::blocks::Block;
use crate::core::header::Header;
use crate::core::number::U256;
use crate::core::{Address, Hash, Transaction};

pub fn get_genesis_block() -> Block {
    let mut b = Block {
        // header: BlockHeader {
        //     index: 0,
        //     prev_hash: Hash::empty(),
        //     merkle_root: Hash::empty(),
        //     leader: Address::Nowhere,
        //     sig: 0,
        // },
        // body: Vec::new(),
        header: Header {
            parent_hash: [0; 32],
            number: U256::zero(),
            state_root: [0; 32],
            block_root: [0; 32],
            digests: Default::default(),
        },
        body: vec![],
    };
    // b.body.push(Transaction::RegularSend {
    //     src: Address::Nowhere,
    //     dst: Address::PublicKey(123),
    //     amount: 123,
    //     sig: 0,
    // });
    b
}
