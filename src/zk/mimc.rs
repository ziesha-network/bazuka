use crate::crypto::Fr;

pub fn mimc(a: Fr, b: Fr) -> Fr {
    a + b
}
