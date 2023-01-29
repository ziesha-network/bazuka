use super::{UNIT, UNIT_ZEROS};

use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::common::*;
use crate::consensus::pow::Difficulty;
use crate::core::{
    Amount, Block, ContractId, Header, Money, ProofOfWork, Signature, Token, TokenId, Transaction,
    TransactionAndDelta, TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::TxBuilder;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 15;
const MPN_LOG4_PAYMENT_CAPACITY: u8 = 1;
pub const MPN_LOG4_TOKEN_CAPACITY: u8 = 1;

const TESTNET_HEIGHT_LIMIT: u64 = 12000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000e402955a7b62419e1903eea6a5159665258d0ec2b314d42637b1ec00e0f0e78e19a2a7c9a094b0d65ee953c8b07b5c0b76195408ff58ecf21b5975f5adccd49779fa38ccb557162c27f856a95d9405883f5115097d478f0ffa51a84a519ba801005be316496a25f0ee4b1e8017641b54d479714f9b5925ca9074d22dfe23afe4c4d84be0b813b3c7579e501153161e9b1704784b365f97ff1f00e0e2152edb35fdd3bcd7b3cd37ae0997e3ac866cd8a72c7908ea866fa0663f5ec9d01ed725510e00b8769d7202993f3838b4d00c3941d06a71d8602555ddc5326caf6ad1bff24add5052f12d653797a77e0c87af5f07b90c6db35d28214c21cbd87075ad7703f4b4103011e3f39ede308c109af2e0b50a41fe6a5606fb0f3b03fd7e31b268039c0100a2cfc18e15b2c56b6e4c11e9bcd937c06f5a444c003794ea1c52f91e9d851aed342ce4cd8ca1fdae1ee7702e84a177128162c962d7b64677f76e729424c77451e79906fd74412302ba87da4cd4ceaa5b4e263fbf35063639c4664b3836293f06006e2e45c6f604a91ea59057e4276dee9040f92c35c3131d58880ee6b8ef1edadb2f4807dbebe5121500a6a1d2c53dba01e84dfa3c17ddf28d17768b36b0bb24e9d3b1000d2c82b021429e8b28da4220f4b5c8ff74a3e4932e47b22498321a931800").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000460d8c166e124266cc5b9f1bcf96143620d88038c166591a040f6013fddaca41157ae3ead0b71d68644dcba9adc7600d3f56541c90a3151ee825ecca93e514844a3950d46b00cd22c5bcaa72c903b5e68cc833d8edf7aaa548a898bdb9c68614003f4ef0cf7d742eff40bb2e06852a52d0a3e13168d425592d8f57fbf376421e6799f0b92220b503cc4758e395b055aa0de6a090280d24b3f4d8dd2039929b9c1a13df070feb47202efe08be88b641264050b1963bf96848110269e48f63fb0302001436fa856acd9b3937612849e1fe55f5bf793d06ba3974571d339c986f1af56bd33d4dbffcd3fd49b3a537b8e37a3900997ae18eee39fbff865e7a7e00b293952b3324e4b3eba14110c26d53b3192d3b150569d07e85c757f9c7f2d6e6037e0b00034cdbf9bbb35fd9575e50fcff48a2ef714d57cf022e46c64d72e1a6096b092bdd7511941c3248200d7db06e15c5a013dc1c5cd1c7f0032ba770f9c3d6ebee10911d6429b481eaf9f93089b9099cc9e19ece80690fe908e51aa75ed753b4b7050028b307c9639641dded3458b633bef8a2eea1d3f0d68a1a50d14f36c7a9c5496fd447d8a380e1478501f046d015d65704600e7999ce4a624798bcabd8b451e58b504bc7765608da767f955e5f41231761e66b258b0bee86dc5a606d2dd5b0a01600").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000004d2cc0157f52969976dce0ef34771e94918d274349b942a3ceaf43a39eebde7bd8aba2804b05a41cc20ee418644b9f069cd25457bb073b117bcbfd24f2bb9bb4a79f07ec271a56bb19bf9f355f3df89b9d7995cdf6ea10dbda09c30325f6cb15001a20279f8b6173da96f7215eb50c7e6a1e343b34b81fe18dce505ddea72af6ca330151eb3041b0cf8d021a2d5ec78a09aa67c18fcbade6cf6e15fe087c392d3817df63f6745df92f6b7e90781a6d4f2eda1df1b867f48930755c62f21775160f00c59a7a646fbfb345031bf2c47b5d28f54d088c3ddb354a90b5b6fee2e4ec0d2f065e0d82289c6a5f3453af5853e49613fb30eba3247cb081161f9ddec9d5f95b1f2e71dd80135cf14a092ff9f3634dab410ec5085cf0f97b0c6012e41a6df40000789cac874603aabe324086e71725ac9f20701e09db5608fac17997b1d041909c63743078b95e9985589a7c8dc371d301d0ff7acb35b567c7d604bb1e2fe9990548e2b8de478689e9a2ffc245d18fe6058eec2977e75f484bf178f965f81cfb0f0005bc400df9894271743c7a048d1aefda8c942770d737b35a9d6153f0b686435db04e19c8cf737f6ab3af45dce06f2b112465c9d539d0845b01d951dcc2caa4d8d2563183fbaf8f3ebb72f41cfc6f9d1f98d6f9540c087140f20eda219270330c00").unwrap()).unwrap();
}

fn get_mpn_contract() -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::List {
        log4_size: MPN_LOG4_ACCOUNT_CAPACITY,
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Nonce
                zk::ZkStateModel::Scalar, // Pub-key X
                zk::ZkStateModel::Scalar, // Pub-key Y
                zk::ZkStateModel::List {
                    log4_size: MPN_LOG4_TOKEN_CAPACITY,
                    item_type: Box::new(zk::ZkStateModel::Struct {
                        field_types: vec![
                            zk::ZkStateModel::Scalar, // Token-Id
                            zk::ZkStateModel::Scalar, // Balance
                        ],
                    }),
                },
            ],
        }),
    };
    let mpn_contract = zk::ZkContract {
        state_model: mpn_state_model.clone(),
        initial_state: zk::ZkCompressedState::empty::<ZkHasher>(mpn_state_model),
        deposit_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_DEPOSIT_VK.clone())),
            log4_payment_capacity: MPN_LOG4_PAYMENT_CAPACITY,
        }],
        withdraw_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_WITHDRAW_VK.clone())),
            log4_payment_capacity: MPN_LOG4_PAYMENT_CAPACITY,
        }],
        functions: vec![zk::ZkSingleInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone())),
        }],
    };
    let mpn_contract_create_tx = Transaction {
        memo: "A Payment-Network to rule them all!".into(),
        src: None,
        data: TransactionData::CreateContract {
            contract: mpn_contract,
        },
        nonce: 2, // MPN contract is created after Ziesha token is created
        fee: Money::ziesha(0),
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

fn get_ziesha_token_creation_tx() -> Transaction {
    Transaction {
        memo: "Happy Birthday Ziesha!".into(),
        src: None,
        data: TransactionData::CreateToken {
            token: Token {
                name: "Ziesha".into(),
                symbol: "ZSH".into(),
                supply: Amount(2_000_000_000_u64 * UNIT),
                decimals: UNIT_ZEROS,
                minter: None,
            },
        },
        nonce: 1,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    }
}

pub fn get_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let ziesha_token_creation_tx = get_ziesha_token_creation_tx();
    let ziesha_token_id = TokenId::new(&ziesha_token_creation_tx);

    let blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            block_root: Default::default(),
            proof_of_work: ProofOfWork {
                timestamp: 0,
                target: Difficulty(0x00ffffff),
                nonce: 0,
            },
        },
        body: vec![ziesha_token_creation_tx, mpn_tx_delta.tx],
    };

    BlockchainConfig {
        limited_miners: None,
        mpn_contract_id,
        ziesha_token_id,
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
        reward_ratio: 100_000, // 1/100_000 -> 0.01% of Treasury Supply per block
        max_block_size: MB as usize,
        max_delta_count: 1024, // Only allow max of 1024 ZkScalar cells to be added per block
        block_time: 120,       // Seconds
        difficulty_window: 150, // Blocks
        difficulty_lag: 10,    // Blocks
        difficulty_cut: 15,    // Blocks

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
        mpn_log4_account_capacity: MPN_LOG4_ACCOUNT_CAPACITY,
        mpn_proving_time: 30, // Seconds

        minimum_pow_difficulty: Difficulty(0x02ffffff),

        testnet_height_limit: Some(TESTNET_HEIGHT_LIMIT),
        max_memo_length: 64,
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    use crate::core::RegularSendEntry;
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let mut conf = get_blockchain_config();
    conf.limited_miners = None;
    conf.mpn_num_contract_deposits = 0;
    conf.mpn_num_contract_withdraws = 0;
    conf.mpn_num_function_calls = 0;
    conf.mpn_proving_time = 0;
    conf.mpn_contract_id = mpn_contract_id;
    conf.minimum_pow_difficulty = Difficulty(0x007fffff);
    conf.testnet_height_limit = None;

    conf.genesis.block.body[1] = get_test_mpn_contract().tx;
    let abc = TxBuilder::new(&Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        memo: "Dummy tx".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: abc.get_address(),
                amount: Money::ziesha(10000),
            }],
        },
        nonce: 3,
        fee: Money::ziesha(0),
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
