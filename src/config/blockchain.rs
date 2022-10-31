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
use crate::wallet::TxBuilder;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 15;
const MPN_LOG4_PAYMENT_CAPACITY: u8 = 3;

const TESTNET_HEIGHT_LIMIT: u64 = 5000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("213f36c08dd39f6fc0bdbf4a0270597d91ade8f0399f36e85f7009310c126c3b02e2e44a43396c350645640daf7f630c1218d5362ded84bd320f577995dd6d1095f4ce9a07be8badcaba05dfae206631f6bdbadb3e8e183cbe48e5175dd14208005f70c17532fa40c6e275c04636399f27595ffcb353cdd6906192bc5d834e9475d271d49cbae1df8dc9de4b0537b070067aa0356819ce8d4b6009267c534a12e022845bc3f6511668807ac8ca094cc5501249c77a049cbb5378cc52b591e00e1900b03ea20ed68171935cbf8a1c3556f8d2f4588157b0c58b7c658db4f858d74e9f54f25dde23ca206add8d28478bee890aca353c4fc6517ee7c38b0caf134b9466583b2275c8b9ef5816084b78760d624a894cb491f8f85ef1b150b8751433c4183e373cd1724596cf68c099a8a2da9e8e26425393183f3f1ef7acb65a50c4476f4fef8323067a3123d5509bfd6d066713db23475cb9d826b29ca8f8d0bb71c594f1543288884fef9a3d7868e7ee32530db3a29e334be7745600446b44748d8e1600869f66ef34f41ccddde72995d39e624ebe092d7aeec6c2ce4444e69bdaad249f9b2b9a9a86a0ac3f48dd17abeaa9680bd689bdf47350f776c2c56a3c1efc7620d646bd88e8ffb88da90dae8ed5645667515ac684062008902067219000e6380cdf02a9c1fb194a703af32029571df3e91451943476bfc8c2b3cc4352be45c1fc59e1f7b54f43ce1cc635a9645e67d90c000d0480f67d4214de3681b86452cbc7966409ef61e78598bc134cbdf6fefc08bf71b5bafd41feafe2fb4f4da51d8107008f8c276c1277fff4820158a975fa9f71fdba96b879d25819eda2585b565914277fd11b94f6d226a1b1054bd78988460eecd6fcb706bf74287fe458f59c481d35ff1827fe63644a37d0ca00b0018c563645ed05ad08b26277445c8d4ee85e631500962f1ed43bf896c4db1dc55cac3192c49f6f540d0fbf2b194560846953b1e9a8e0bf0ae913d6d220e4d77ca083a31419024086eb0b47a6fd562b291a273817be990227de020e1409b88ee38127f989e8564e83be6415ef4d8fcde9edcc0d8e0b5a658fe5db59b717654a263b94ebb7260fb4c7d8055fd0b6677012d4e63a243ffbfcf54df708dcc6136510469cd561058191d79717174187a904d9f944b4c66ded36c3fe192b0f3583b03388fb7918234c51dd7f10d599af689e47e51daef2060005000000000000006b503338ed737da3d1a383e80a5234691d990645c793575eade783fe695fddfd0e0fb1adb5c26031bbf118ba4270d314c3250964c39cd6416d22062135287f4a9812453789e8938cc086df2f6b10dea3561472b7abe7e43c24920dbc2f1ac601000a1f3096b6d1740601896fb485f9f3cec84cf0781683a77eb41f8cea5cde6af138fa030e188ef073f5ca02420f8f9800202c2b946b5ddf7c874e78887e7d739749e0d14f5f9018527e80968e7d130bf996c77a59566885b1a1f491a6f83c3b0000832da2e63ac86fee0b1dc82143bdc0120c1dac3495afa99a8ee479773895f855e2624613651a146506afd5667cf63d06d567ae234072c52a3b4e1738bad7abc8e3e1698c833dede3e1cf5a3ff592532f741b1908ea39f712a654908c62ae3c08006806e976713659b3fc19e732f63309afefc20fc9cafb30c64ace10086e9235fc28fccef85c4bc781e874885810c2430b4f87ca0b591a75ad86bb6feef0dac7262dc3e81704656c43a892fbd09ab6a54e6d8b8f3f9d36f2d22f1af0fb30de3b1200474c37ac32c5bc3ea83e586ac01701d40c7acbd110d7bba20f78062eba0bd3a0570121ff1f121d1a58c3c68edc60d500f16478c740e43e8953f9ad1cce622c4ecef028cf4ab8999839c016f12fd0270296bb6363e7ef057fb71126fafd5a580c00").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("213f36c08dd39f6fc0bdbf4a0270597d91ade8f0399f36e85f7009310c126c3b02e2e44a43396c350645640daf7f630c1218d5362ded84bd320f577995dd6d1095f4ce9a07be8badcaba05dfae206631f6bdbadb3e8e183cbe48e5175dd14208005f70c17532fa40c6e275c04636399f27595ffcb353cdd6906192bc5d834e9475d271d49cbae1df8dc9de4b0537b070067aa0356819ce8d4b6009267c534a12e022845bc3f6511668807ac8ca094cc5501249c77a049cbb5378cc52b591e00e1900b03ea20ed68171935cbf8a1c3556f8d2f4588157b0c58b7c658db4f858d74e9f54f25dde23ca206add8d28478bee890aca353c4fc6517ee7c38b0caf134b9466583b2275c8b9ef5816084b78760d624a894cb491f8f85ef1b150b8751433c4183e373cd1724596cf68c099a8a2da9e8e26425393183f3f1ef7acb65a50c4476f4fef8323067a3123d5509bfd6d066713db23475cb9d826b29ca8f8d0bb71c594f1543288884fef9a3d7868e7ee32530db3a29e334be7745600446b44748d8e1600869f66ef34f41ccddde72995d39e624ebe092d7aeec6c2ce4444e69bdaad249f9b2b9a9a86a0ac3f48dd17abeaa9680bd689bdf47350f776c2c56a3c1efc7620d646bd88e8ffb88da90dae8ed5645667515ac684062008902067219000e6380cdf02a9c1fb194a703af32029571df3e91451943476bfc8c2b3cc4352be45c1fc59e1f7b54f43ce1cc635a9645e67d90c000d0480f67d4214de3681b86452cbc7966409ef61e78598bc134cbdf6fefc08bf71b5bafd41feafe2fb4f4da51d8107008f8c276c1277fff4820158a975fa9f71fdba96b879d25819eda2585b565914277fd11b94f6d226a1b1054bd78988460eecd6fcb706bf74287fe458f59c481d35ff1827fe63644a37d0ca00b0018c563645ed05ad08b26277445c8d4ee85e631500962f1ed43bf896c4db1dc55cac3192c49f6f540d0fbf2b194560846953b1e9a8e0bf0ae913d6d220e4d77ca083a31419024086eb0b47a6fd562b291a273817be990227de020e1409b88ee38127f989e8564e83be6415ef4d8fcde9edcc0d8e0b5a658fe5db59b717654a263b94ebb7260fb4c7d8055fd0b6677012d4e63a243ffbfcf54df708dcc6136510469cd561058191d79717174187a904d9f944b4c66ded36c3fe192b0f3583b03388fb7918234c51dd7f10d599af689e47e51daef206000500000000000000fde806d1247bde73f03d7805536a934b46cf08062d1a246ea46add80dbfcf1920f34f6f8e2ce187084a48cd9d0ee5f0c67002b5f56beae8c2c916b8ddd1ee4d262001f83be11ea3a34791d51e1c5efefdfa72041c4762e6d75f31d88e92cf40100bd3a1ace8b1d6d988282708e10024a88d02764c8fd84dfdff017e1c78bcaf37ef28e5692decaad121aa2189eceea7207b13a86cc9fd9764fd9fa80b5be6f20a7ee217fbd2e4278ade50aa4961793ae1613a2271f3ecd8c49aa77b4aebcba19110044418790e1922ea1c274fb4dd3ecd573ebca20dd246f2c7fb6b106bfd80f942050ec15713ac5a25bb9ee7a27b73bb01818bed1a98e59fd3b6b98f148a7fd2e6b57bafedc280bfca065eab4c282c4de429ff1fed01f610695f6cacd1cb3b2b50e00407485bb7fb6e0c1dcc709517e6cfe535928523750694e6c7c250ff4b5122357364be2e2dd60c0166ceed8f572cb3a17deb29e59e4816da4a4442645b1d0cb28810118ff0260e871bb39c98067e3eeff99bd7d4471722abae17e32ccaa0b5002006186e0d137adecd63f455abec1c368c04843b221a0ce0db10e907394caa945384165b10305205d6d701cb1c43a4b6300a93febf05d56e74139fd143289539c804ee118170ac11c178aba678cb33deeeffed8ef97666c20788d68f8d5b3bd4a0100").unwrap()).unwrap();
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
        deposit_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
            log4_payment_capacity: MPN_LOG4_PAYMENT_CAPACITY,
        }],
        withdraw_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
            log4_payment_capacity: MPN_LOG4_PAYMENT_CAPACITY,
        }],
        functions: vec![zk::ZkSingleInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone())),
        }],
    };
    let mpn_contract_create_tx = Transaction {
        src: Address::Treasury,
        data: TransactionData::CreateContract {
            contract: mpn_contract,
        },
        nonce: 1,
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
            contract.deposit_functions = vec![zk::ZkMultiInputVerifierKey {
                verifier_key: zk::ZkVerifierKey::Dummy,
                log4_payment_capacity: 1,
            }];
            contract.withdraw_functions = vec![zk::ZkMultiInputVerifierKey {
                verifier_key: zk::ZkVerifierKey::Dummy,
                log4_payment_capacity: 1,
            }];
            contract.functions = vec![zk::ZkSingleInputVerifierKey {
                verifier_key: zk::ZkVerifierKey::Dummy,
            }];
        }
        _ => panic!(),
    }
    mpn_tx_delta.state_delta = Some(init_state.as_delta());
    mpn_tx_delta
}

pub fn get_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let min_diff = Difficulty(0x020fffff);

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
        body: vec![mpn_tx_delta.tx],
    };

    BlockchainConfig {
        limited_miners: None,
        mpn_contract_id,
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
        mpn_num_function_calls: 1,
        mpn_num_contract_deposits: 1,
        mpn_num_contract_withdraws: 1,

        minimum_pow_difficulty: min_diff,

        testnet_height_limit: Some(TESTNET_HEIGHT_LIMIT),
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let min_diff = Difficulty(0x007fffff);

    let mut conf = get_blockchain_config();
    conf.limited_miners = None;
    conf.mpn_num_contract_deposits = 0;
    conf.mpn_num_contract_withdraws = 0;
    conf.mpn_num_function_calls = 0;
    conf.mpn_contract_id = mpn_contract_id;
    conf.minimum_pow_difficulty = min_diff;
    conf.genesis.block.header.proof_of_work.target = min_diff;
    conf.testnet_height_limit = None;

    conf.genesis.block.body[0] = get_test_mpn_contract().tx;
    let abc = TxBuilder::new(Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: abc.get_address(),
            amount: Money(10000),
        },
        nonce: 2,
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
