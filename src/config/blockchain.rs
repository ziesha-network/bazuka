use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 10;

lazy_static! {
    pub static ref MPN_CONTRACT_ID: ContractId = ContractId::new(&get_mpn_contract().tx);
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("beeb72575fa5f3760789d59bb3a364288b2aa748a750e52aaa74d475e4958999824d55256205e8d6627fd9752df2af1917c644509090843bf0ca21dd7fd5180e5d5a46232bcaead38ae9f3e95cf3c159ee53a7142f7a2b054fa704d903e7f10d004115e12acbb5df78b9a99c738cb72e5bd0631ac32bf7195d068114de9fab86acc57a18865a65100397a60b52134417152223bcda6966b329996f49df376c7e066be30709377ae771873238e4ea6200921b4c3cd389cd72ccea30f6a8172e850d00a7a0d5d43d324ff705023abbe3eaf6e3321cef4148ad4215abf2bc8929d2600aec3b73d01e5a0ab744aba5f14401fd0bb0c242282b79abe0fa22ece59d28c85c715b2b06a02f457c3db8dcc28bb2000fd47a4c8130994fa6c203f927b4501a0198496b3766693c6e065753ea1ef22f703fc2c6041834f15b682aab421c94d64b9549fa881ed9e91a50a93c7cf1113f129a07b84da46b3553241488f10e6c13151b84171cef5fef2ed4ad4f300888e48d9fd35bcd0b2696db3df43bda5a28e010005664407631efd224a2fee6f685463d38f2acc2de8fdac1d6345f07eb5b9987023402635f5587ba2c5745c0add12b181691d816b318caf606bb25a12d90003adb71bbc7a6229ef6f0f1a3cfd1c0d287c413f5dbade8deee401dcdee76d3d58018d54539bf850f44a3caa00e518341e73d16648276be8324b23eb65c091bc40b0705d02ce23ab48223cb2d3b785d4b70147ab3db66782fa72cbb33e57e75928fc0e05e5e47367ac9697f6f0300957bb2f5d33105802e2eea3039415ce49cad0b06003c90da718570dc7622d39ddf679b5f2b870ec26361438f377c41d8f25ea92266bec623183d6251eee3084dc8ce32fc12b66f01dbd48ea54d025df6020bb61b297dcbbbb49c6a0bed25eb732deb14c3ad11df6dcc4cbf00e278a183b49f568f080038c56de41a8054e4629e9b756533be377f1e393127da83c5090a007df9e91f39932f06afca7247c3a8ef43868aae350766531907b4086a0ac5c5fb1303654dcd47454dcaaea2710083a73016221569ffa6d801a9aac7c02898ed4d3fdfee9f09868884e532b3369d91e298c592d628792abe7c4efb46a73b0eb2650ddf442a553074afb56b51e877df3e0e718525b2057ceaa03945f7c9a9d10839d27c147dc7622182b7a1a12cc1e77b6de861ef2fe4622947046af138ffaf9af31fa9ce3711000400000000000000d735decf3950f0de8e1f1fee1092903698740c03af47efcdc8d62a70161e359abc8283ca8564c1b8196e2ccc01b55c0c3b733c1ab9171da0c61eaa840bfc50841dac69d3a89c97cc837f8f311c23accf71c89477654c22c05aaf60f632237c0e008464f07156a07f08b8ed10292e204be2a197a38978b7367a2ac476f908cdc5e1ceb7e9b2af8114874d1545180c590203d289026c8f5184ca95440c5c6eaf0dca06395c0b156b2f37c2dc5f20d8cc1afbecc8b94128e2920ac5d22d55899c0512001b1e30f67e45dd51c4bbbfcd06fa33e678bbd740969d360172df603eff1f755356532059de2021510196c3626bd56a03db0d4ba99263d9d67390c12b1938cc0f3c2ab50599fd673959fabef33ae602731299be71fe6a06181d2f6b7883987b180035aadcb41246d89e0cab2f50b2c8ad6e3b283f8f55fa8bfc057cb8faea176346c67bddb15bcb052347a007bf2b0c01189b486d7cee42b7cb8b026c544dbe1df0679057659c2d036e5071baa74e8185f49bd22ab60c67ccf61d185ead81574e0600").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("861db8fc29c617d79e282b240e61f8512de3c2daf69003ba5a4b377ed0d46623468ab339af0a307b18faf83b78db110a2a0a6fd98a3b75425a9fb7bc4c920b93b24b81432dfeecb7ad612a56ca3332fee598826c3d7f73a3f2a033434594650a0006e17b97036a6c1d1cba40a6f508297036cc61f857b2112f30ae6f2c3e9e4563d5a3e63850e76cfcb91e67261172a0155de16554e86451763d387e9966f57c5d65ad3419bc3d6bc4a7a6202de1581fe8d5db916595c66addda9972bb9dc0f6000047b6049fbc5c6d553dddf9810e8882cfdef4237ac100fe56ed3be290aa653bb5b1c092b904e70d3fab14632ee77b4908486f79adc03c5626b1cd7782e26b49b40ab833a3e3f761b431b158a609a38f4b504ed68c5e8d3a6d6de0afe07221e601c2c134eb80783ad317e709acb472c793083dad6f57b2e54fbdd7357ae44993ea411c1017266cfe7ac8db862efde5e20d6ab6feb73da73e6caba7e44235b729823cfefaaf193b0267d3e1fc8e23392b0c9e7af9fe1470cca957f74988a4c5af0b00309086df45abf5dcadf0b6f91ba3e25ea4ad2e24ce959f1a337c8efdc644bfeaf0e6fb5bb4aa522232a1978ac06c780576013dc42c4fd2c9d1dc44a1dfaff4c35740c3e8a14e199bf4054e0bfda8dfd63f7afe4907c2ca767923430afe95150d49cf5d2a12dff72db587f71a0f69b6a85006644595f2b8219e22cadf5a3cda80753ac983bff40db76e25ad4b1c6c7713389994598d41cddde9cf97a49ea55fc605d3af02d6f8472a119ee6fa3a7722332d40f6628e16ea627d03011ff4d8840300436b37bfc4bbab405351e92aaafb334cfe698b188f607c387716baa0a4af9a764c0525af621af32f643ccecb2ea97f0e5dc62c94e32b76d82214f960ee69c7c2e9844ad5c9dd337a09789ba5fa115616c00fbc1f3614276f8677d03e33367118008912642bb615023452d2c3def48521fba2abd297be24d9ef63b8ff00af22380bdd7b8a0cfa9cd26e85732e568911f9167062b3e00cda19cd0a21c2b33f38751c3431a401c85779b98764fb518c6e32bc01fbbe33a234765a52be862e67e147065be3424cc0a25436d9bea46f87a39c5941047946a0bdadc8f47caecc42045d7bb619f2ef7f0c87ab2955c22cf8fa981453f38c4e54d5a7a5fb8295d2deac60059c184b324d95276036aa8b8945cf5b9ad03a9c0b5f4ce52d85dbc49c47588e120004000000000000009b54b852c3a16525c45a90a94a496625219060268d88b9c0620ac23977d21625b139b6d6f0266ed3eb5dd6001283d41853b86fcf61f4513469cd56941c4c45c1cefcaee9d83f70c1c799de424324b70c11209d539269797ff084cae00d220b0800a33bed510933b4192138a0cb5cccd4fa6b94331f685b3e9aac82cbcead95825d28424406dca0fa0e21c4062577c6f71880715178388475628f1338cfbe482ae72c03602b243d80b2bf132be9ab27a5066b51d4acbd5b153d6af85deedb8f250500fd8637c8bc91ec45e08b42aa2c57b20456fd06f4fab973ef39813d877db0695ff88e0f7b5e784e281e905f8caf2fcf0f28cf56fbed44f54c6dbc0c89f3580c11f1717a174b94406adf0e45ffe33b2d9a4d73af541904671989e74d5fc9b69e0800344f0217f93a23cb023e31c873a7143e3522022b8649e0aa14862c1fe9fafd72508724ae1663bd3ccf8c0389e557e816f91d144899b45894814284621be215ccc5c6ad75a052a5e7083c9cf40e23b19a551bc874ee32a74d2c45be1b02c6720a00").unwrap()).unwrap();
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
        log4_payment_capacity: 1,
        payment_function: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
        functions: vec![zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone()))],
    };
    let mpn_contract_create_tx = Transaction {
        src: Address::Treasury,
        data: TransactionData::CreateContract {
            contract: mpn_contract,
        },
        nonce: 2,
        fee: 0,
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
            contract.payment_function = zk::ZkVerifierKey::Dummy;
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

    let blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            block_root: Default::default(),
            proof_of_work: ProofOfWork {
                timestamp: 0,
                target: 0x02ffffff,
                nonce: 0,
            },
        },
        body: vec![
            Transaction {
                src: Address::Treasury,
                data: TransactionData::RegularSend {
                    dst: "0x93dbba22f3bc954eb24cbe3fe697c70d3ab599c070ca057f0ed4690570db307c"
                        .parse()
                        .unwrap(),
                    amount: 100000000,
                },
                nonce: 1,
                fee: 0,
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
        total_supply: 2_000_000_000_000_000_000_u64, // 2 Billion ZIK
        reward_ratio: 100_000, // 1/100_000 -> 0.01% of Treasury Supply per block
        max_delta_size: 1024 * 1024, // Bytes
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
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);
    println!("CONT: {}", mpn_contract_id);

    let mut conf = get_blockchain_config();
    conf.mpn_num_contract_payments = 0;
    conf.mpn_num_function_calls = 0;
    conf.genesis.block.header.proof_of_work.target = 0x007fffff;
    conf.genesis.block.body[1] = get_test_mpn_contract().tx;
    let abc = Wallet::new(Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: abc.get_address(),
            amount: 10000,
        },
        nonce: 3,
        fee: 0,
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
