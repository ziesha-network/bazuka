pub trait Account<Mon, Addr> {
    fn get_addr(&self) -> Addr;
    fn get_balance(&self) -> Mon;
}

pub trait Transaction<Mon, Addr, Sig> {
    fn get_src(&self) -> Addr;
    fn get_dst(&self) -> Addr;
    fn get_amount(&self) -> Mon;
    fn get_sig(&self) -> Sig;
}

pub trait Bank<Wit, Addr, Hash, Mon, Sig, Tx: Transaction<Mon, Addr, Sig>, Acc: Account<Mon, Addr>>
{
    // Apply a set of transactions, move to a new state, and provide its witness.
    fn apply_transactions(&mut self, txs: &Vec<Tx>) -> Wit;

    // Update the state through a chain of witnesses and switch to a new state.
    fn update_state(&mut self, witnesses: &Vec<Wit>, new_state: Vec<Acc>);

    // Get state of the bank..
    fn get_state(&self) -> &Vec<Acc>;

    // Get hash of state.
    fn get_hash(&self) -> Hash;
}
