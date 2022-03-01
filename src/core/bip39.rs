use hmac::Hmac;
use pbkdf2::pbkdf2;
use schnorrkel::keys::MiniSecretKey;
use sha2::Sha512;
use zeroize::Zeroize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum Error {
    #[error("entropy has an invalid length")]
    InvalidEntropy,
}

pub fn mini_secret_from_entropy(entropy: &[u8], password: &str) -> Result<MiniSecretKey, Error> {
    let seed = seed_from_entropy(entropy, password)?;
    Ok(MiniSecretKey::from_bytes(&seed[..32]).expect("Length is always correct;"))
}

pub fn seed_from_entropy(entropy: &[u8], password: &str) -> Result<[u8; 64], Error> {
    if entropy.len() < 16 || entropy.len() > 32 || entropy.len() % 4 != 0 {
        return Err(Error::InvalidEntropy);
    }

    let mut salt = String::with_capacity(8 + password.len());
    salt.push_str("mnemonic");
    salt.push_str(password);

    let mut seed = [0u8; 64];

    pbkdf2::<Hmac<Sha512>>(entropy, salt.as_bytes(), 2048, &mut seed);

    salt.zeroize();

    Ok(seed)
}
