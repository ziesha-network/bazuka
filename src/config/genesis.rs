use crate::core::{Address, Block, BlockHeader, Hash, Transaction};

pub fn get_genesis_block() -> Block {
    let mut b = Block {
        header: BlockHeader {
            index: 0,
            prev_hash: Hash::empty(),
            merkle_root: Hash::empty(),
            leader: Address::Nowhere,
            sig: 0,
        },
        body: Vec::new(),
    };
    b.body.push(Transaction::RegularSend {
        src: Address::Nowhere,
        dst: Address::PublicKey(123),
        amount: 123,
        sig: 0,
    });
    b
}
