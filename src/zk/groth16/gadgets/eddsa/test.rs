use super::*;
use crate::core::ZkHasher;
use crate::crypto::jubjub::*;
use crate::crypto::ZkSignatureScheme;
use crate::zk::groth16::Bls12;
use crate::zk::ZkScalar;
use bellman::gadgets::boolean::{AllocatedBit, Boolean};
use bellman::gadgets::num::AllocatedNum;
use bellman::{groth16, Circuit, ConstraintSystem, SynthesisError};
use rand::rngs::OsRng;

struct TestEddsaVerify {
    enabled: Option<bool>,
    pub_key: Option<PointAffine>,
    message: Option<BellmanFr>,
    sig_r: Option<PointAffine>,
    sig_s: Option<BellmanFr>,
}

impl Circuit<BellmanFr> for TestEddsaVerify {
    fn synthesize<CS: ConstraintSystem<BellmanFr>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let enabled = Boolean::Is(AllocatedBit::alloc(&mut *cs, self.enabled)?);
        let pub_key = AllocatedPoint::alloc(&mut *cs, || {
            self.pub_key.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let msg = AllocatedNum::alloc(&mut *cs, || {
            self.message.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let sig_r = AllocatedPoint::alloc(&mut *cs, || {
            self.sig_r.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let sig_s = AllocatedNum::alloc(&mut *cs, || {
            self.sig_s.ok_or(SynthesisError::AssignmentMissing)
        })?;

        verify_eddsa(&mut *cs, &enabled, &pub_key, &msg.into(), &sig_r, &sig_s)?;

        Ok(())
    }
}

#[test]
fn test_eddsa_verify() {
    let params = {
        let c = TestEddsaVerify {
            enabled: None,
            pub_key: None,
            message: None,
            sig_r: None,
            sig_s: None,
        };
        groth16::generate_random_parameters::<Bls12, _, _>(c, &mut OsRng).unwrap()
    };

    let pvk = groth16::prepare_verifying_key(&params.vk);

    let keys = JubJub::<ZkHasher>::generate_keys(b"salam");
    let msg = ZkScalar::from(1234);
    let sig = JubJub::<ZkHasher>::sign(&keys.1, msg);

    // Correct signature
    let c = TestEddsaVerify {
        enabled: Some(true),
        pub_key: Some(keys.0 .0.decompress()),
        message: Some(msg.into()),
        sig_r: Some(sig.r.into()),
        sig_s: Some(sig.s.into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(groth16::verify_proof(&pvk, &proof, &[]).is_ok());

    // Wrong signature
    let c = TestEddsaVerify {
        enabled: Some(true),
        pub_key: Some(keys.0 .0.decompress()),
        message: Some((msg + ZkScalar::from(1)).into()),
        sig_r: Some(sig.r.into()),
        sig_s: Some(sig.s.into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(!groth16::verify_proof(&pvk, &proof, &[]).is_ok());

    // Wrong signature but disabled
    let c = TestEddsaVerify {
        enabled: Some(false),
        pub_key: Some(keys.0 .0.decompress()),
        message: Some((msg + ZkScalar::from(1)).into()),
        sig_r: Some(sig.r.into()),
        sig_s: Some(sig.s.into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(groth16::verify_proof(&pvk, &proof, &[]).is_ok());
}
