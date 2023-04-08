use super::*;
use crate::zk::groth16::Bls12;
use crate::zk::ZkScalar;
use bellman::gadgets::num::AllocatedNum;
use bellman::{groth16, Circuit, ConstraintSystem, SynthesisError};
use rand::rngs::OsRng;

struct TestPoseidon4Circuit {
    pub a: Option<BellmanFr>,
    pub b: Option<BellmanFr>,
    pub c: Option<BellmanFr>,
    pub d: Option<BellmanFr>,
    pub e: Option<BellmanFr>,
    pub out1: Option<BellmanFr>,
    pub out2: Option<BellmanFr>,
    pub out3: Option<BellmanFr>,
    pub out4: Option<BellmanFr>,
    pub out5: Option<BellmanFr>,
}

impl Circuit<BellmanFr> for TestPoseidon4Circuit {
    fn synthesize<CS: ConstraintSystem<BellmanFr>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let out1 = AllocatedNum::alloc(&mut *cs, || {
            self.out1.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let out2 = AllocatedNum::alloc(&mut *cs, || {
            self.out2.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let out3 = AllocatedNum::alloc(&mut *cs, || {
            self.out3.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let out4 = AllocatedNum::alloc(&mut *cs, || {
            self.out4.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let out5 = AllocatedNum::alloc(&mut *cs, || {
            self.out5.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let a = AllocatedNum::alloc(&mut *cs, || self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = AllocatedNum::alloc(&mut *cs, || self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = AllocatedNum::alloc(&mut *cs, || self.c.ok_or(SynthesisError::AssignmentMissing))?;
        let d = AllocatedNum::alloc(&mut *cs, || self.d.ok_or(SynthesisError::AssignmentMissing))?;
        let e = AllocatedNum::alloc(&mut *cs, || self.e.ok_or(SynthesisError::AssignmentMissing))?;

        let res1 = poseidon(&mut *cs, &[&a.clone().into()])?;
        let res2 = poseidon(&mut *cs, &[&a.clone().into(), &b.clone().into()])?;
        let res3 = poseidon(
            &mut *cs,
            &[&a.clone().into(), &b.clone().into(), &c.clone().into()],
        )?;
        let res4 = poseidon(
            &mut *cs,
            &[
                &a.clone().into(),
                &b.clone().into(),
                &c.clone().into(),
                &d.clone().into(),
            ],
        )?;
        let res5 = poseidon(
            &mut *cs,
            &[&a.into(), &b.into(), &c.into(), &d.into(), &e.into()],
        )?;
        cs.enforce(
            || "",
            |lc| lc + out1.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + res1.get_lc(),
        );
        cs.enforce(
            || "",
            |lc| lc + out2.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + res2.get_lc(),
        );
        cs.enforce(
            || "",
            |lc| lc + out3.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + res3.get_lc(),
        );
        cs.enforce(
            || "",
            |lc| lc + out4.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + res4.get_lc(),
        );
        cs.enforce(
            || "",
            |lc| lc + out5.get_variable(),
            |lc| lc + CS::one(),
            |lc| lc + res5.get_lc(),
        );
        Ok(())
    }
}

#[test]
fn test_poseidon_circuit() {
    let params = {
        let c = TestPoseidon4Circuit {
            a: None,
            b: None,
            c: None,
            d: None,
            e: None,
            out1: None,
            out2: None,
            out3: None,
            out4: None,
            out5: None,
        };
        groth16::generate_random_parameters::<Bls12, _, _>(c, &mut OsRng).unwrap()
    };

    let pvk = groth16::prepare_verifying_key(&params.vk);

    let expecteds = (0..5)
        .map(|i| {
            crate::zk::poseidon::poseidon(
                &[
                    ZkScalar::from(123),
                    ZkScalar::from(234),
                    ZkScalar::from(345),
                    ZkScalar::from(456),
                    ZkScalar::from(567),
                ]
                .into_iter()
                .take(i + 1)
                .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let c = TestPoseidon4Circuit {
        a: Some(ZkScalar::from(123).into()),
        b: Some(ZkScalar::from(234).into()),
        c: Some(ZkScalar::from(345).into()),
        d: Some(ZkScalar::from(456).into()),
        e: Some(ZkScalar::from(567).into()),
        out1: Some(expecteds[0].into()),
        out2: Some(expecteds[1].into()),
        out3: Some(expecteds[2].into()),
        out4: Some(expecteds[3].into()),
        out5: Some(expecteds[4].into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(groth16::verify_proof(&pvk, &proof, &[]).is_ok());

    let c = TestPoseidon4Circuit {
        a: Some(ZkScalar::from(123).into()),
        b: Some(ZkScalar::from(234).into()),
        c: Some(ZkScalar::from(345).into()),
        d: Some(ZkScalar::from(457).into()),
        e: Some(ZkScalar::from(567).into()),
        out1: Some(expecteds[0].into()),
        out2: Some(expecteds[1].into()),
        out3: Some(expecteds[2].into()),
        out4: Some(expecteds[3].into()),
        out5: Some(expecteds[4].into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(!groth16::verify_proof(&pvk, &proof, &[]).is_ok());
}
