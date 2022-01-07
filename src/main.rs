mod net;

type Signature = u8;
type Address = u8;
type Hash = u8;
type Money = u8;

struct Transaction {
    src: Address,
    dst: Address,
    amount: Money,
    sig: Signature,
}

struct BlockHeader {
    prev_hash: Hash,
    body_hash: Hash,
    leader: Address,
    sig: Signature,
}

struct Block {
    header: BlockHeader,
    body: Vec<Transaction>,
}

trait Blockchain {
    fn get_balance(&self, addr: Address) -> Money;
    fn extend(&mut self, blocks: &Vec<Block>);
}

fn main() {
    println!("Hello Bazuka!");
    net::init().unwrap();
}
