use crate::zk::ZkScalar;
use ff::PrimeField;
use num_bigint::BigUint;
use num_traits::Num;

const PARAM_FILES: [&str; 16] = [
    include_str!("poseidon_params_n255_t2_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t3_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t4_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t5_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t6_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t7_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t8_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t9_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t10_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t11_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t12_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t13_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t14_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t15_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t16_alpha5_M128.txt"),
    include_str!("poseidon_params_n255_t17_alpha5_M128.txt"),
];

pub const MAX_ARITY: usize = PARAM_FILES.len();

fn read_constants(line: &str) -> Vec<ZkScalar> {
    let mut constants_str = line.to_string().replace("0x", "");
    constants_str.retain(|c| c != '\'' && c != '[' && c != ']' && c != ' ');
    constants_str
        .split(',')
        .map(|s| {
            ZkScalar::from_str_vartime(&BigUint::from_str_radix(s, 16).unwrap().to_string())
                .unwrap()
        })
        .collect()
}

fn parse_params(source: &str) -> PoseidonParams {
    let lines = source.lines().collect::<Vec<_>>();
    let opts = lines[0].split(",").map(|s| s.trim()).collect::<Vec<_>>();
    let capacity: usize = opts[1].split("=").collect::<Vec<_>>()[1].parse().unwrap();
    let full_rounds: usize = opts[4].split("=").collect::<Vec<_>>()[1].parse().unwrap();
    let partial_rounds: usize = opts[5].split("=").collect::<Vec<_>>()[1].parse().unwrap();
    let round_constants = read_constants(lines[3]);
    let mds_constants = read_constants(lines[15])
        .chunks(capacity)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    PoseidonParams {
        capacity,
        full_rounds,
        partial_rounds,
        round_constants,
        mds_constants,
    }
}

#[derive(Debug, Clone)]
pub struct PoseidonParams {
    pub capacity: usize,
    pub full_rounds: usize,
    pub partial_rounds: usize,
    pub round_constants: Vec<ZkScalar>,
    pub mds_constants: Vec<Vec<ZkScalar>>,
}

lazy_static! {
    static ref PARAMS: [PoseidonParams; 16] = PARAM_FILES
        .iter()
        .map(|src| parse_params(src))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
}

impl PoseidonParams {
    pub fn for_width(width: usize) -> Option<&'static Self> {
        PARAMS.get(width - 2)
    }
}
