use super::UNIT;

use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::common::*;
use crate::consensus::pow::Difficulty;
use crate::core::{
    Address, Block, ContractId, Header, Money, ProofOfWork, Signature, Transaction,
    TransactionAndDelta, TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 10;

lazy_static! {
    pub static ref MPN_CONTRACT_ID: ContractId = ContractId::new(&get_mpn_contract().tx);
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("213f36c08dd39f6fc0bdbf4a0270597d91ade8f0399f36e85f7009310c126c3b02e2e44a43396c350645640daf7f630c1218d5362ded84bd320f577995dd6d1095f4ce9a07be8badcaba05dfae206631f6bdbadb3e8e183cbe48e5175dd14208005f70c17532fa40c6e275c04636399f27595ffcb353cdd6906192bc5d834e9475d271d49cbae1df8dc9de4b0537b070067aa0356819ce8d4b6009267c534a12e022845bc3f6511668807ac8ca094cc5501249c77a049cbb5378cc52b591e00e1900b03ea20ed68171935cbf8a1c3556f8d2f4588157b0c58b7c658db4f858d74e9f54f25dde23ca206add8d28478bee890aca353c4fc6517ee7c38b0caf134b9466583b2275c8b9ef5816084b78760d624a894cb491f8f85ef1b150b8751433c4183e373cd1724596cf68c099a8a2da9e8e26425393183f3f1ef7acb65a50c4476f4fef8323067a3123d5509bfd6d066713db23475cb9d826b29ca8f8d0bb71c594f1543288884fef9a3d7868e7ee32530db3a29e334be7745600446b44748d8e1600869f66ef34f41ccddde72995d39e624ebe092d7aeec6c2ce4444e69bdaad249f9b2b9a9a86a0ac3f48dd17abeaa9680bd689bdf47350f776c2c56a3c1efc7620d646bd88e8ffb88da90dae8ed5645667515ac684062008902067219000e6380cdf02a9c1fb194a703af32029571df3e91451943476bfc8c2b3cc4352be45c1fc59e1f7b54f43ce1cc635a9645e67d90c000d0480f67d4214de3681b86452cbc7966409ef61e78598bc134cbdf6fefc08bf71b5bafd41feafe2fb4f4da51d8107008f8c276c1277fff4820158a975fa9f71fdba96b879d25819eda2585b565914277fd11b94f6d226a1b1054bd78988460eecd6fcb706bf74287fe458f59c481d35ff1827fe63644a37d0ca00b0018c563645ed05ad08b26277445c8d4ee85e631500962f1ed43bf896c4db1dc55cac3192c49f6f540d0fbf2b194560846953b1e9a8e0bf0ae913d6d220e4d77ca083a31419024086eb0b47a6fd562b291a273817be990227de020e1409b88ee38127f989e8564e83be6415ef4d8fcde9edcc0d8e0b5a658fe5db59b717654a263b94ebb7260fb4c7d8055fd0b6677012d4e63a243ffbfcf54df708dcc6136510469cd561058191d79717174187a904d9f944b4c66ded36c3fe192b0f3583b03388fb7918234c51dd7f10d599af689e47e51daef20600040000000000000078ea9d67dcaf4f97dbf5f5894a378d6b6453cdfb031cf1e209ad214ad5913560bdd74d2f3bb26643dfcd81cc37e390065e01c9ea9b4d03235d6824bc0f7d56869d48b22c737f003da045ec3d389310f4b0a6c8c232dc241ef513cebf06c9ea1300557196c5844aa6633fa3c6acd91179f1e6b1c3a6e03f1fcf1bb621b119cdcd1e9787c01f91a55cdca8cea74ef0e49a0b26d5f339131e357ec49a55ac31c2fb64c34fac12cec2c55f4a1116335502030343af48f98213d85a746e0fbdce09ac160035ebbd711d9b44721c3c7d328c602d7c961cc6d1c2090620c9551c22ed3d87c1a75bfe63ab9650e0823577c234a5bb0025e55b23ca56ceccac936d7ec1b3969f54614979ec73ab2df8663a1006a7b7ea0441d18b9c6bad44992f10f8edfe0f0c005c798bff5733d99098087f995317d243ebdb39316dc4647a962b0f6ec47a7a47d29c8bd3b5e87136bb8dbfe5755c63190bfc77868a1750d88fbecd1f9eb8686c2a9f716e93de5c06da2d498beb3e5d7070964f2e1d178d947aceabbd58df880900").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("213f36c08dd39f6fc0bdbf4a0270597d91ade8f0399f36e85f7009310c126c3b02e2e44a43396c350645640daf7f630c1218d5362ded84bd320f577995dd6d1095f4ce9a07be8badcaba05dfae206631f6bdbadb3e8e183cbe48e5175dd14208005f70c17532fa40c6e275c04636399f27595ffcb353cdd6906192bc5d834e9475d271d49cbae1df8dc9de4b0537b070067aa0356819ce8d4b6009267c534a12e022845bc3f6511668807ac8ca094cc5501249c77a049cbb5378cc52b591e00e1900b03ea20ed68171935cbf8a1c3556f8d2f4588157b0c58b7c658db4f858d74e9f54f25dde23ca206add8d28478bee890aca353c4fc6517ee7c38b0caf134b9466583b2275c8b9ef5816084b78760d624a894cb491f8f85ef1b150b8751433c4183e373cd1724596cf68c099a8a2da9e8e26425393183f3f1ef7acb65a50c4476f4fef8323067a3123d5509bfd6d066713db23475cb9d826b29ca8f8d0bb71c594f1543288884fef9a3d7868e7ee32530db3a29e334be7745600446b44748d8e1600869f66ef34f41ccddde72995d39e624ebe092d7aeec6c2ce4444e69bdaad249f9b2b9a9a86a0ac3f48dd17abeaa9680bd689bdf47350f776c2c56a3c1efc7620d646bd88e8ffb88da90dae8ed5645667515ac684062008902067219000e6380cdf02a9c1fb194a703af32029571df3e91451943476bfc8c2b3cc4352be45c1fc59e1f7b54f43ce1cc635a9645e67d90c000d0480f67d4214de3681b86452cbc7966409ef61e78598bc134cbdf6fefc08bf71b5bafd41feafe2fb4f4da51d8107008f8c276c1277fff4820158a975fa9f71fdba96b879d25819eda2585b565914277fd11b94f6d226a1b1054bd78988460eecd6fcb706bf74287fe458f59c481d35ff1827fe63644a37d0ca00b0018c563645ed05ad08b26277445c8d4ee85e631500962f1ed43bf896c4db1dc55cac3192c49f6f540d0fbf2b194560846953b1e9a8e0bf0ae913d6d220e4d77ca083a31419024086eb0b47a6fd562b291a273817be990227de020e1409b88ee38127f989e8564e83be6415ef4d8fcde9edcc0d8e0b5a658fe5db59b717654a263b94ebb7260fb4c7d8055fd0b6677012d4e63a243ffbfcf54df708dcc6136510469cd561058191d79717174187a904d9f944b4c66ded36c3fe192b0f3583b03388fb7918234c51dd7f10d599af689e47e51daef206000400000000000000333e3befd5e90c348d615fe3cdc2cad5b0b3fb771e158ec1b2d27956217d38f930afd0589bd0b291c9662c6d9711ad0152dcb5e1847ba1f028ba9b2b425d2cea9c7eb8e74e0b7f544200cfc052feb43f9aeff0b327cdde07d2bd8fcf5190370700a3e4996fe685c592602276b2b63e7ebf1fb2232e66ce9fe96949213c526a0c54fce46c784493126ac5b50df610f35418e779e2b2e7c4818d6386aae9fbbad0b2577000ecf90e67c876f9026d46151e89404b6efa727d97d021997d1d23fbf70f00613e81bbc265e72dfa5ead257b0de55ad0451887af62f3d165c2acaa890573a3f9f9509003911847509f29deb45a79076580cded1c21a3faf5115c2f6109963168960bd61140a4e1578b43e65494cb09f19183c34f2468e73c3098b645f35918008a7c8c7abec246a59740c919aef169e6ba26417f0283d4f09e13eb319656650907a904901b9858817fd049b396e735030ac66a9bb77a437d47f316fd15b74c7ca1d327302686372f3e2ec9c6fbb694a5f14a55a71f36dff3ee58b54dc0bbac1300").unwrap()).unwrap();
}

fn get_mpn_contract() -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::List {
        log4_size: MPN_LOG4_ACCOUNT_CAPACITY,
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Nonce
                zk::ZkStateModel::Scalar, // Pub-key X
                zk::ZkStateModel::Scalar, // Pub-key Y
                zk::ZkStateModel::Scalar, // Balance
            ],
        }),
    };
    let mpn_contract = zk::ZkContract {
        state_model: mpn_state_model.clone(),
        initial_state: zk::ZkCompressedState::empty::<ZkHasher>(mpn_state_model),
        payment_functions: vec![zk::ZkPaymentVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
            log4_payment_capacity: 1,
        }],
        functions: vec![zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone()))],
    };
    let mpn_contract_create_tx = Transaction {
        src: Address::Treasury,
        data: TransactionData::CreateContract {
            contract: mpn_contract,
        },
        nonce: 2,
        fee: Money(0),
        sig: Signature::Unsigned,
    };
    TransactionAndDelta {
        tx: mpn_contract_create_tx,
        state_delta: Some(zk::ZkDeltaPairs::default()),
    }
}

#[cfg(test)]
fn get_test_mpn_contract() -> TransactionAndDelta {
    let mut mpn_tx_delta = get_mpn_contract();
    let init_state = zk::ZkDataPairs(
        [(zk::ZkDataLocator(vec![100]), zk::ZkScalar::from(200))]
            .into_iter()
            .collect(),
    );
    match &mut mpn_tx_delta.tx.data {
        TransactionData::CreateContract { contract } => {
            contract.state_model = zk::ZkStateModel::List {
                log4_size: 5,
                item_type: Box::new(zk::ZkStateModel::Scalar),
            };
            contract.initial_state = contract
                .state_model
                .compress::<ZkHasher>(&init_state)
                .unwrap();
            contract.payment_functions = vec![zk::ZkPaymentVerifierKey {
                verifier_key: zk::ZkVerifierKey::Dummy,
                log4_payment_capacity: 1,
            }];
            contract.functions = vec![zk::ZkVerifierKey::Dummy];
        }
        _ => panic!(),
    }
    mpn_tx_delta.state_delta = Some(init_state.as_delta());
    mpn_tx_delta
}

pub fn get_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let min_diff = Difficulty(0x02ffffff);

    let blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            block_root: Default::default(),
            proof_of_work: ProofOfWork {
                timestamp: 0,
                target: min_diff,
                nonce: 0,
            },
        },
        body: vec![
            Transaction {
                src: Address::Treasury,
                data: TransactionData::RegularSend {
                    dst: "0x62f58b091997c0b85a851e08b3cbc5e86ac285b9bd4392ffc4cb5391cad98671"
                        .parse()
                        .unwrap(),
                    amount: Money(100000000),
                },
                nonce: 1,
                fee: Money(0),
                sig: Signature::Unsigned,
            },
            mpn_tx_delta.tx,
        ],
    };

    BlockchainConfig {
        genesis: BlockAndPatch {
            block: blk,
            patch: ZkBlockchainPatch {
                patches: [(
                    mpn_contract_id,
                    zk::ZkStatePatch::Delta(mpn_tx_delta.state_delta.unwrap()),
                )]
                .into_iter()
                .collect(),
            },
        },
        total_supply: Money(2_000_000_000_u64 * UNIT), // 2 Billion ZIK
        reward_ratio: 100_000, // 1/100_000 -> 0.01% of Treasury Supply per block
        max_block_size: (1 * MB) as usize,
        max_delta_count: 1024, // Only allow max of 1024 ZkScalar cells to be added per block
        block_time: 60,        // Seconds
        difficulty_calc_interval: 128, // Blocks

        // 0 63 -> BAZUKA BASE KEY
        // 64 2111 -> hash(blk#0)
        // 2112 4159 -> hash(blk#2048)
        // 4160 6207 -> hash(blk#4096)
        // ...
        pow_base_key: b"BAZUKA BASE KEY",
        pow_key_change_delay: 64,      // Blocks
        pow_key_change_interval: 2048, // Blocks

        // New block's timestamp should be higher than median
        // timestamp of 10 previous blocks
        median_timestamp_count: 10,

        // We expect a minimum number of MPN contract updates
        // in a block to consider it valid
        mpn_num_function_calls: 0,
        mpn_num_contract_payments: 1,

        minimum_pow_difficulty: min_diff,
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let min_diff = Difficulty(0x007fffff);

    let mut conf = get_blockchain_config();
    conf.mpn_num_contract_payments = 0;
    conf.mpn_num_function_calls = 0;
    conf.minimum_pow_difficulty = min_diff;
    conf.genesis.block.header.proof_of_work.target = min_diff;

    conf.genesis.block.body[1] = get_test_mpn_contract().tx;
    let abc = Wallet::new(Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: abc.get_address(),
            amount: Money(10000),
        },
        nonce: 3,
        fee: Money(0),
        sig: Signature::Unsigned,
    });
    conf.genesis.patch = ZkBlockchainPatch {
        patches: [(
            mpn_contract_id,
            zk::ZkStatePatch::Delta(mpn_tx_delta.state_delta.unwrap()),
        )]
        .into_iter()
        .collect(),
    };
    conf
}
