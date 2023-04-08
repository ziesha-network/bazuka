use super::*;
use crate::core::ZkHasher;
use crate::zk::groth16::Bls12;
use crate::zk::{ZkDataLocator, ZkDataPairs, ZkScalar, ZkStateBuilder};
use bellman::gadgets::num::AllocatedNum;
use bellman::{groth16, Circuit, ConstraintSystem, SynthesisError};
use rand::rngs::OsRng;

struct TestRevealCircuit {
    state_model: ZkStateModel,
    data: Option<ZkDataPairs>,
    out: Option<BellmanFr>,
}

fn extract_witnesses<CS: ConstraintSystem<BellmanFr>>(
    cs: &mut CS,
    state_model: ZkStateModel,
    locator: ZkDataLocator,
    pairs: &Option<ZkDataPairs>,
) -> Result<AllocatedState, SynthesisError> {
    match state_model {
        ZkStateModel::Scalar => {
            let num = AllocatedNum::alloc(&mut *cs, || {
                if let Some(pairs) = pairs {
                    Ok(pairs
                        .0
                        .get(&locator)
                        .cloned()
                        .unwrap_or_else(|| state_model.compress_default::<ZkHasher>())
                        .into())
                } else {
                    Err(SynthesisError::AssignmentMissing)
                }
            })?;
            Ok(AllocatedState::Value(num.into()))
        }
        ZkStateModel::Struct { field_types } => {
            let mut children = Vec::new();
            for (i, field_type) in field_types.into_iter().enumerate() {
                children.push(extract_witnesses(
                    &mut *cs,
                    field_type,
                    locator.index(i as u64),
                    pairs,
                )?);
            }
            Ok(AllocatedState::Children(children))
        }
        ZkStateModel::List {
            log4_size,
            item_type,
        } => {
            let mut children = Vec::new();
            for i in 0..(1 << (2 * log4_size)) {
                children.push(extract_witnesses(
                    &mut *cs,
                    *item_type.clone(),
                    locator.index(i as u64),
                    pairs,
                )?);
            }
            Ok(AllocatedState::Children(children))
        }
    }
}

impl Circuit<BellmanFr> for TestRevealCircuit {
    fn synthesize<CS: ConstraintSystem<BellmanFr>>(
        self,
        cs: &mut CS,
    ) -> Result<(), SynthesisError> {
        let out = AllocatedNum::alloc(&mut *cs, || {
            self.out.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let alloc_state = extract_witnesses(
            &mut *cs,
            self.state_model.clone(),
            ZkDataLocator(vec![]),
            &self.data,
        )?;

        let root = reveal(&mut *cs, &self.state_model, &alloc_state)?;

        cs.enforce(
            || "",
            |lc| lc + root.get_lc(),
            |lc| lc + CS::one(),
            |lc| lc + out.get_variable(),
        );

        Ok(())
    }
}

#[test]
fn test_reveal_circuit() {
    let state_model = ZkStateModel::Struct {
        field_types: vec![
            ZkStateModel::Scalar,
            ZkStateModel::List {
                item_type: Box::new(ZkStateModel::Scalar),
                log4_size: 2,
            },
            ZkStateModel::Scalar,
            ZkStateModel::Scalar,
        ],
    };
    let params = {
        let c = TestRevealCircuit {
            state_model: state_model.clone(),
            data: None,
            out: None,
        };
        groth16::generate_random_parameters::<Bls12, _, _>(c, &mut OsRng).unwrap()
    };

    let pvk = groth16::prepare_verifying_key(&params.vk);

    let data = ZkDataPairs(
        [
            (ZkDataLocator(vec![1, 2]), ZkScalar::from(10)),
            (ZkDataLocator(vec![1, 4]), ZkScalar::from(10)),
            (ZkDataLocator(vec![1, 10]), ZkScalar::from(15)),
            (ZkDataLocator(vec![0]), ZkScalar::from(123)),
        ]
        .into(),
    );

    let mut builder = ZkStateBuilder::<ZkHasher>::new(state_model.clone());
    builder.batch_set(&data.as_delta()).unwrap();
    let expected = builder.compress().unwrap().state_hash;

    let c = TestRevealCircuit {
        state_model: state_model.clone(),
        data: Some(data),
        out: Some(expected.into()),
    };
    let proof = groth16::create_random_proof(c, &params, &mut OsRng).unwrap();
    assert!(groth16::verify_proof(&pvk, &proof, &[]).is_ok());
}
