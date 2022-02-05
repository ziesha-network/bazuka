pub type Signature = u8;
pub type Address = u8;
pub type Hash = u8;
pub type Money = u8;

pub enum Transaction {
    RegularSend {
        src: Address,
        dst: Address,
        amount: Money,
        sig: Signature,
    },
}

pub struct BlockHeader {
    pub prev_hash: Hash,
    pub body_hash: Hash,
    pub leader: Address,
    pub sig: Signature,
}

pub struct Block {
    pub header: BlockHeader,
    pub body: Vec<Transaction>,
}
