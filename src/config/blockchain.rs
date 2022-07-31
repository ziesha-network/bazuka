use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::zk;
use std::str::FromStr;

#[cfg(test)]
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 10;

lazy_static! {
    pub static ref MPN_CONTRACT_ID :ContractId =ContractId::from_str(
        "96a4f133131b9ada171fef48549cdfac34a67e4518cb438f52ad9c12d735d7c5",
    ).unwrap();
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b60697319bb284a8cd34518def7a98ee19bd0f55302b4c0f35caae68491e41546f777df93610b8946dd378720eb5990b4b3b4bed9d13ef008014c7ad59c73b0816d97719e0946ecfa37bf576c5098124998191b564f69655ae03199689b5440c000ae9f97b5bc22598ea12c2d266cfc7edb373dbdfdcadf1ab51d63e53501419066b2c4a728d4b0a2e0705826fc774b10c6d1ee0e73bcc55c5eb54553d35f6912031e8b514d8a36c524726e2df928b43a83ede1a9d0a6e05067d9f91306e11f511006167f9945a4b53cc86e735667a4f371e0799f085d3aa024598fc3ce3eaa56daaa800180e4dc7caad5090549a1babe911af5bc7bfa42f195289d1023fa4731f585913277229780e546c31ebc5af0c9859fbd690af11cfca0fa9e2ecc7b541900cc1621e87198197759b9b8202f2486388fa6402565579d8afc4d0b68ba433027bee6f737ce9d3cae5e38a2c2027a00a02c8be3607afbee65b3e4aab64bf3dae19b03c82909931ac340ff42fe78ec8d1976e0058639530059fe9d5c8aa11f3780b000b43a69e0d4f4db173fa231eac34eba39043034a31aacb45b65d881b6312b3c22bd5e8c125f968fb14645af1267dfc0b48d350b11dec75a1c1e5f283922608af20fef1f2623a34a3dcf20c81684cafa26769b41e27c0ed2c8b32b709d2206e0aa199597498ef4e6822ccaea14a9c210ebfc0ac1b0105cb41059ccb5ff24ef3b69a7ea4c022ff95dcd002b13a3aea480158fa19932b5704dc09ba6b9a8080207d2b69d05726ed6ebdece16942c3e3ea5a2e03819cd22a6e695a80362e7157631600d5bf79693bb900fff268630450f227fc88d12ebb444bcceb6d8093daba7c76d1a7ae4dc570edebf9cf33e54cbbeae510bcef8b1a02bd9c71f2d7edfff31023bd012c9d4a33468b35e160c5b272c88f40ea9ea0719b1040a76e4d30e745fd400600fbb79bb8b77027f139d6cae39f3f4c3c9a4ba08b0fca1ef1c6192b210e052f094e9eddf9061c90f1bff43712b7301401843cc1a523d01c7cb5230ad6dcd69bf178a0d6286d4442a730e3b89658526cc2e6b923b46bcade2db7553b5dc769881066064af038abdc53ba4eaef69c9d1f781114cd85ed92cd40f26974cfe5b7ba57e013e8fab8f82eb167c927b6b4258e08b1d127fc192df721ac7810406640802f0517cd29d2b7ec0bbc2ae19c7cd918bef153c920d47c7ec7cb1b7a3d1f827218000400000000000000d4a464c494f08677feeb76c391cf50730a35aaafe331836ca11094e5740f9f0d42281ce7720eb122a260b5ad69f6df12f3df6b831021825cbe41c72f79082fc0391a9a7bf90947913ca31c3626feff8baf2301ba4070ed38b839260078f02c0b00f28f64275e612d572fe70b1889bdbb9d9a5cb89875eb02e86b01bcd05d5b250a9f7fbee719d1e6865f92f13a6c8a0c1607380a98c5254a0b11c0f4f88684371bed8ca898abd9adf2a53e327e0bec247bc3b62bddc785696e3ab234be81973e180069fa563966917f3dc20db3a021305b8d8e7a9faa74599abc608453ea59d89c38fcff657edc71eb207fe42154b39dbc00d60b16b8dc31f92572b05e9be608d8c7fe03341a56889aebac3b73aa18ba655056a85736b807aed8d498120bfacaba0e00c3d83b5371c995970e7e9324d4e06617744c2e85bb70159965df25314910f86ec1fb961e5f2cc0122a10da309df820072ee69036bf41bec4cecc7e6509eba72af1ddf615d01da198f584699606c5422fe7e71ed6d038581e5f2f009cab3b760300").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("131689c0505c961c08bfc519929503bd8e4a1644ddb1ebb49fe04eecbf876abdc23b78d4a2ebcbc954d766c4478fe703d756f773e8eece1251311d6221f1cab9518f816dc7d08c0d68a16a8a5f749d4bda66419313245bf5ea16da564a98aa0100e1cf9860fac0afb7a08da860a5cd2a8dfd2d67010d73f6031236945189a65e17bfbdf113c6502889479d6459da777c148fec398252f6ae19bd791109d5998d77fed6a9be3544825d7aac380943c36cb0e72c8f239211ac37f383644a73ba491900c44c337aaa16e64cf361c723d267630a0fe9113bcdf80a879f4d510b48340eb4128262d63c8407a1ec90b9890bdeb10085aa7d953f87e55f3ae8870fae5665723e3dbfa60d2f4ea987081d89078c70bdb487086376d2d2aec652840df8ee9405e73c6b65a18261454eab49925ea40d38ed4c28f6b668797c2e542dfa024da57241f16302f79fc396bf627b986e84831857b9ac4215cd6d20a0d1c00e4ed38398af9279bf8fc7c02dc648eaeeb8a14f22ecf77543bca9c6f9f79aae8685a8d501005d8db0776635d85120bec70e9afeda08c267efbc8fdc5f402e7d4e27383445f49835c69d99fa192bff941808c402c701c2675a5ab550f55f0dca9afe16607ed964a81e85425d6ce70c6e5e23569fa19ac628d6c80f9c78675b49f9a446a6c610cd3108a4d547f5a3bddc536a1fe8a401696c5edf2fbc575d8a0b92d69ff1e4d62ffa12b764514ccee7ce4093146f0a0178b0b1f5ffa7a775a0151156d69b1528cac88a9c11e6d5b59132f543e3ccc7a7f36c4763c4e5e7b21198e03c978be915009cee9e47c9747d1a61b1741931257b6431889dc18a5c48c3d9034ec0b6f9d12bb341605a59e9ed7dd9f07c4eeac8ab19fd6e14bca6e5c1aa86eabfdbec9b8064f3763c610ed82b9db5389831d62d339134bca5430cd62c5f73d9fa761d1e2617005faa5d9cb208807eda710b9adbb45f841d141649eecc39339acee0009b6e0fa6ddfbf3dd8d310a06146744ca6a5108165923facafac4c73aeabfbef699802c4792020e210b3f43e0ef4380175a8438cd7986235c8b026a3d5d42b45ccffb0c1667bb1ec57c04abae492e240b372358b509045f6547bb8c5566d503715ea0dac4a1b3389c3e2d3887173d9b8c0e79ac19f2b9f0a75b2782a33aa43ac64e02ebff25f52df58b4d0c55c687ff01812b813a7084a44e6a6a5bead87d837735140a0a000400000000000000bb6d76d51b5db3cf7f222f9bf864a935b523137f015908ec393117d1de9d78c29f36fc259fdf54b3c45a1875b5657511744052e3d62e9a687f4896ebbfe51c4e2698cd6e249d727194507eab02a1cb6dbe93607509a651f6ff4cc64acafaa914002399c1a07c7406d51943f4f4fe57d488ad8dfa8a55c7a67af3477b199a13624fc3feed4e40753f39b7d5226495036006a2cc4a5ff532fe0b1076b8000f15bbdceb4ad748512615c8a4ef6e030f39515f132d6cf8a2b47a7b8eaa74b4d2ef680900f6d03e44cebcee9478519b78d136a4236dffdfcb2c605640a22847d619aab2bffd815445f6084185724cf14da0b3b7102746520963cfd0556a3d31e6ebef664d3ae533522e2f850d781e23aafd49b5509f803d130388bb2f6367a3d380959c06002211f87df552d38d2b67037e23617b82510897c22dcbeb2c6cfb0a5b5b403d919085352c6921683c7ad0be7a2b2a2f0ee708b7d48da536c76ffd6a17f3935b62f208738372c4a29d1b506f0e361156dba790eca4f3cddb59c3ef23b03509c20e00").unwrap()).unwrap();
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
    println!("{}", ContractId::new(&mpn_contract_create_tx));
    assert_eq!(
        MPN_CONTRACT_ID.clone(),
        ContractId::new(&mpn_contract_create_tx)
    );
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
