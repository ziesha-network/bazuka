use super::*;
use crate::zk::groth16::Bls12;
use crate::zk::{
    PoseidonHasher, ZkDataLocator, ZkDeltaPairs, ZkScalar, ZkStateBuilder, ZkStateModel,
};
use bellman::gadgets::num::AllocatedNum;
use bellman::{groth16, Circuit, ConstraintSystem, SynthesisError};
use rand::rngs::OsRng;

struct TestPoseidon4MerkleProofCircuit {
    index: Option<BellmanFr>,
    val: Option<BellmanFr>,
    root: Option<BellmanFr>,
    proof: Vec<[Option<BellmanFr>; 3]>,
}

impl Circuit<BellmanFr> for TestPoseidon4MerkleProofCircuit {
    fn synthesize<CS: ConstraintSystem<BellmanFr>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let index = AllocatedNum::alloc(&mut *cs, || {
            self.index.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let val = AllocatedNum::alloc(&mut *cs, || {
            self.val.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let root = AllocatedNum::alloc(&mut *cs, || {
            self.root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let mut proof = Vec::new();
        for p in self.proof {
            proof.push([
                AllocatedNum::alloc(&mut *cs, || p[0].ok_or(SynthesisError::AssignmentMissing))?,
                AllocatedNum::alloc(&mut *cs, || p[1].ok_or(SynthesisError::AssignmentMissing))?,
                AllocatedNum::alloc(&mut *cs, || p[2].ok_or(SynthesisError::AssignmentMissing))?,
            ]);
        }

        let enabled = Boolean::Is(AllocatedBit::alloc(&mut *cs, Some(true))?);
        let index = UnsignedInteger::constrain(&mut *cs, index.into(), 6)?;

        check_proof_poseidon4(
            &mut *cs,
            &enabled,
            &index.into(),
            &val.into(),
            &proof,
            &root.into(),
        )?;

        Ok(())
    }
}

#[test]
fn test_poseidon4_merkle_proofs() {
    let params = {
        let c = TestPoseidon4MerkleProofCircuit {
            index: None,
            val: None,
            proof: vec![[None; 3]; 3],
            root: None,
        };
        groth16::generate_random_parameters::<Bls12, _, _>(c, &mut OsRng).unwrap()
    };

    let pvk = groth16::prepare_verifying_key(&params.vk);

    let model = ZkStateModel::List {
        log4_size: 3,
        item_type: Box::new(ZkStateModel::Scalar),
    };
    let mut builder = ZkStateBuilder::<PoseidonHasher>::new(model);
    for i in 0..64 {
        builder
            .batch_set(&ZkDeltaPairs(
                [(ZkDataLocator(vec![i]), Some(ZkScalar::from(i as u64)))].into(),
            ))
            .unwrap();
    }
    for i in 0..64 {
        let proof: Vec<[Option<BellmanFr>; 3]> = builder
            .prove(ZkDataLocator(vec![]), i)
            .unwrap()
            .into_iter()
            .map(|p| [Some(p[0].into()), Some(p[1].into()), Some(p[2].into())])
            .collect();

        let index = ZkScalar::from(i as u64);
        let val = ZkScalar::from(i as u64);
        let root = builder.get(ZkDataLocator(vec![])).unwrap();

        let c = TestPoseidon4MerkleProofCircuit {
            index: Some(index.into()),
            val: Some(val.into()),
            proof,
            root: Some(root.into()),
        };
        let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
        assert!(groth16::verify_proof(&pvk, &proof, &[]).is_ok());
    }
}
