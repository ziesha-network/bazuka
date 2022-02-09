use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum KeyIdError {
    IllegalLength,
}

impl Display for KeyIdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyIdError::IllegalLength => write!(f, "length of the raw in key-id must be four"),
            _ => unreachable!(),
        }
    }
}

impl std::error::Error for KeyIdError {}

#[derive(Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(pub [u8; 4]);

impl From<u32> for KeyId {
    fn from(u: u32) -> Self {
        Self(u.to_le_bytes())
    }
}

impl From<KeyId> for u32 {
    fn from(k: KeyId) -> Self {
        u32::from_le_bytes(k.0)
    }
}

impl<'a> TryFrom<&'a [u8]> for KeyId {
    type Error = KeyIdError;

    fn try_from(val: &'a [u8]) -> Result<Self, Self::Error> {
        conv_to_key_id(val)
    }
}

impl<'a> TryFrom<&'a str> for KeyId {
    type Error = KeyIdError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let dat = value.as_bytes();
        conv_to_key_id(dat)
    }
}

fn conv_to_key_id(dat: &[u8]) -> Result<KeyId, KeyIdError> {
    match dat.len() {
        4 => {
            let mut key_id = KeyId::default();
            key_id.0.copy_from_slice(dat);
            Ok(key_id)
        }
        _ => Err(KeyIdError::IllegalLength),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
