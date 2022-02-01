pub enum NetworkError {
    Down
}

pub trait Interface {
    fn send(data: Vec<u8>)->Result<(), NetworkError>;
    fn receive() -> Result<Vec<u8>, NetworkError>;
}
