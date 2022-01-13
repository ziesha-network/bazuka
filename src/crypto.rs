pub trait SignatureScheme<Pub, Priv, Sig> {
    fn generate() -> (Pub, Priv);
    fn sign(sk: Priv, msg: Vec<u8>) -> Sig;
    fn verify(pk: Pub, msg: Vec<u8>, sig: Sig) -> bool;
}

pub trait VerifiableRandomFunction<Pub, Priv, Output, Proof> {
    fn generate() -> (Pub, Priv);
    fn evaluate(sk: Priv, input: Vec<u8>) -> (Output, Proof);
    fn verify(pk: Pub, input: Vec<u8>, output: Output, proof: Proof) -> bool;
}
