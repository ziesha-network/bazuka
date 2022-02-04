pub enum NetworkError {
    Down,
}

pub trait Interface {
    fn send(&self, data: Vec<u8>) -> Result<(), NetworkError>;
    fn receive(&self) -> Result<Vec<u8>, NetworkError>;
}
