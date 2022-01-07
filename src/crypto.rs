// Bls12-381 signature scheme

// Key generation
// The key generation algorithm selects a random integer x
// such as 0 < x < r. The private key is x. The holder of
// the private key publishes the public key, g^x.

// Signing
// Given the private key x, and some message m, we compute
// the signature by hashing the bitstring m, as h = H(m).
// We output the signature sig = h^x.

// Verification
// Given a signature sig and a public key g^x, we verify
// that e(sig, g) = e(H(m), g^x)

trait SignatureScheme<Pub, Priv, Sig> {
    fn generate() -> (Pub, Priv);
    fn sign(sk: Priv, msg: Vec<u8>) -> Sig;
    fn verify(pk: Pub, msg: Vec<u8>, sig: Sig) -> bool;
}
