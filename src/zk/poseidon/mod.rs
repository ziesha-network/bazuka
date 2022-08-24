mod params;
pub use params::*;

use super::ZkScalar;
use ff::Field;
use std::ops::MulAssign;

#[derive(Debug, Clone, PartialEq)]
struct PoseidonState {
    constants_offset: usize,
    present_elements: u64,
    elements: Vec<ZkScalar>,
}

impl PoseidonState {
    pub fn new(elems: &[ZkScalar]) -> Self {
        let mut elements = elems.to_vec();
        elements.insert(0, ZkScalar::zero());
        Self {
            present_elements: 0u64,
            constants_offset: 0,
            elements,
        }
    }

    pub fn hash(&mut self) -> ZkScalar {
        let params = params_for_width(self.elements.len()).unwrap();

        self.elements[0] = ZkScalar::from(self.present_elements);

        for _ in 0..params.full_rounds / 2 {
            self.full_round(params);
        }

        for _ in 0..params.partial_rounds {
            self.partial_round(params);
        }

        for _ in 0..params.full_rounds / 2 {
            self.full_round(params);
        }

        self.elements[1]
    }

    fn full_round(&mut self, params: &PoseidonParams) {
        self.add_round_constants(params);
        self.elements.iter_mut().for_each(quintic_s_box);
        self.product_mds(params);
    }

    fn partial_round(&mut self, params: &PoseidonParams) {
        self.add_round_constants(params);
        quintic_s_box(&mut self.elements[0]);
        self.product_mds(params);
    }

    fn add_round_constants(&mut self, params: &PoseidonParams) {
        self.elements.iter_mut().for_each(|l| {
            *l += params.round_constants[self.constants_offset];
            self.constants_offset += 1;
        });
    }

    fn product_mds(&mut self, params: &PoseidonParams) {
        let mut result = vec![ZkScalar::from(0u64); self.elements.len()];
        for j in 0..self.elements.len() {
            for k in 0..self.elements.len() {
                result[j] += params.mds_constants[j][k] * self.elements[k];
            }
        }
        self.elements.copy_from_slice(&result);
    }
}

fn quintic_s_box(l: &mut ZkScalar) {
    let mut tmp = *l;
    tmp = tmp.square(); // l^2
    tmp = tmp.square(); // l^4
    l.mul_assign(&tmp); // l^5
}

pub fn poseidon(vals: &[ZkScalar]) -> ZkScalar {
    let mut h = PoseidonState::new(vals);
    h.hash()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ff::{Field, PrimeField};

    #[test]
    fn test_hash_deterministic() {
        let mut h = PoseidonState::new(&[
            ZkScalar::one(),
            ZkScalar::one(),
            ZkScalar::one(),
            ZkScalar::one(),
        ]);

        let mut h2 = h.clone();
        let result = h.hash();

        assert_eq!(result, h2.hash());
    }

    #[test]
    fn test_hash_reflects_changes() {
        for arity in 1..MAX_ARITY + 1 {
            let mut vals = vec![ZkScalar::zero(); arity];
            let original = poseidon(&vals);
            for i in 0..vals.len() {
                vals[i] = ZkScalar::one();
                assert!(poseidon(&vals) != original);
            }
        }
    }

    #[test]
    fn test_hash_samples() {
        let expected = [
            "27570695323925995271701303589514430472678239829854264417883970952440292573348",
            "6587584068506488869767403662460111870851709789694140241572542699619538605403",
            "11065162352055215342882956665028806373710857144056793315618843991574034541745",
            "27235437669367044799899874028200860893259633691548428184978833555844239099210",
            "39122459949963443953695513827515422590145971775731164693081784821001500765271",
            "14822541353598610072073758561600133199190898904019472753356348939736178856242",
            "32119039894111509393883349238591117345166479914896997011437787663480858229324",
            "43492451727584886720328582747486156090763899250669626113572962177392830153672",
            "23782521420058920239581486714235942233162905749917547091367129332109148150964",
            "1950261058989975858181381159018748926889722679795466088362775920975943983890",
            "47763254094198808066374497304963224993617822320088130264863862435119574697678",
            "44035521596650126254580286193043646937530018324533162959282567836364656349620",
            "45248278075433906869650374149660178834237900630357739057386839430392516698709",
            "30558481537294127342952125056358924225581206938869947160862017954746718634085",
            "10702554392571105609953066033536365418563149392782994983402406449789876497692",
            "34319425623279664398659085846739236990635100324667226409415519671072072962346",
        ]
        .into_iter()
        .map(|s| ZkScalar::from_str_vartime(s).unwrap())
        .collect::<Vec<ZkScalar>>();

        let results = (1..MAX_ARITY + 1)
            .map(|count| {
                poseidon(
                    &(0..count)
                        .map(|i| ZkScalar::from(i as u64))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(results, expected);
    }
}
