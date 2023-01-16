use super::{UNIT, UNIT_ZEROS};

use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::common::*;
use crate::consensus::pow::Difficulty;
use crate::core::{
    Address, Amount, Block, ContractId, Header, Money, ProofOfWork, Signature, Token, TokenId,
    Transaction, TransactionAndDelta, TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::TxBuilder;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 15;
const MPN_LOG4_PAYMENT_CAPACITY: u8 = 3;
pub const MPN_LOG4_TOKEN_CAPACITY: u8 = 3;

const TESTNET_HEIGHT_LIMIT: u64 = 12000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000002c3cd6dd9a4bf92fe811d29b126fdd96545e8ba2dfda335ff23092657b71071c84655b49cac87cbcdfc72bf82686d80d6aa00dd1f4e654111d89d5cd1420035d1b441f289e1ba0df0be1daaef70ca11b6cce0839b59f27d3eb9fb671f290ee0800a085a78bd5204906c2ee7f067407e3c9bc438f3e2eaa8fb359c4593e885c953f5d431eeafb66c0ea0806b7e4aca11813f015a60baea18094f3a269eaa7e7599f426471250ea7ca4980ea9164f8a9432a07845a136f8e01ba4b362d59f54dc80800b29fc5391eb1997a52263cd5af0c1f627146ff0f096b35d5501af5014590aaf06db9ae3a01670246f7eb571a33aed6040656e00cf318c5a4ab8bc6d53e0e744149df13afddc32f6115568bcef7b5f0f6d5e00fe99fe585271a6b700fdf5edb170047396c2005c5a0d58d4fb365d9d2d5f0169ca4f85c0cd98babf694539969d9ebad93e3e590095f4250408c11604c2f181f37f07f90ca2b5d8c6d86d43fd54669b684c1431ffa5b5a5e51bf4471ed5241ca3774d68e9a21fc0c19b58b89fc4704001b12ff4069e386e61b65e4dd91ba6cf586fecaeae1abb22b0972a8e39722eeb463507cae0d8c24c1da941cadbddd82114e8d5f81a7cf64deaf496d1fd428a71c77349a5a3ee5e3711b4c0c822f09774790df3774541942e6d2e0f37113204b0a00").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd385280300050000000000000053744a95367ecd368994e8880c810c722b684cba829d16ade64e3a5fa6f2c0cdb478909ef380a0b9ae3c86416d7f1303660929066d9218b831d4dddb790422683c39793075636c76a1441b49f58170a0efa7d8a7374ef825a0e1a5ac235f511000b64eb11afb61522c3dcf1a6efbdb840cfdafddd2fac68e2618e8889b7ee5dc582ac0178c6fb38894f2d4a80a75e37917d8f88aa2aab7eba1d904abff7221843c9a1411cc46ed7874c45ff23a2fb06b5c799297a97e093a8ea1ce309a59677b0d00ba75413371d3ac0564c65ac117fadfae687d29a92a784d6d585d7d97b3f70fa0a14ac411fad13a53a92e334928aef50b79ca589b9898bf7b0917ef608922d94ba76ec464ecb24d75b6af8dedd66a6eeaf7ddd2323f2c379c1f825a5afd977c1800e8dd7293987d60866473492e8b5efce274b12de07ff2bca832d6485a6a5996551d8a59f466bb1ab2f3ed6c90759e3c04447c8acb776f8a683198301081edec7904cddfe0c8fd52ab14c4552d94d196f4f0ae40a1a24182c3f53c4255f2be5b1900e0ed2d43e72fa5c5c71eb3cc3b216040598f1167c0ee1d7b3f59f1e466bac4ca076cf0990b8eda9695b678b1c4bd16110f5bfe97179dd215dc052ff2f6020135c985a8e64a495e6fb9fbb0fe7e6545c4b7d40f667d4f7787859123f0bea3ab0f00").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000f85d7a9418cef172bfb155561966fb1978f4a27569f2187964d93828824ea453ef131a512eaa4292eec5fb9069183110ccb6d2300144d5a89c3d85b7eb481272442678354671e88e46df7b0b1b1aeab06187c12abe7bda331952fafddee51a1300ca993bfef2e7a6680b3d9f35e073b3a9951f6d191f44436db4edd1f413fbb54eba5713d395987ccc937a0b646b21c802236b88d3658fa4dbf7cfdde6abd0c56f936753deb02ca4ca4ea6f57ba53e3228c7f9b14c044934fca0b2a513fe3ea510004b710575e4e79f71970e7a88f4935a34cea42bfd0a415b78557c1664e0e53e14eb156a7dbac5c83a88264c2501f53315a462194bfaba4125e2870f4f2dd6a4f1fecb22173ec1b82db95cd6c849bcf3e6d490523c1ce9ec15ca9364d01647430600ce6ee3547b59cb629b43a2afeadce961aaf1003cba71172c96d8eb8d96d4548779a87b06fcc3e4d78c079da529e944186335d815301862bbd3eaf22e1e1e3bfa387d5e40f1e7336facd285be3cf9e02801f061748477c7071e34a759c3ef661800fd51c582d128a1f4d508e37d02caa492f5726bceed6ede47de19e4134dec5515da51b7255779ad2a27ef58883043e80fb0e22eaac9598d142438a82678a3bc918495c86e5d64575c6a732ac2bf14295a0f42faa55fdd25f23510c8e3f7125f0c00").unwrap()).unwrap();
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
        src: Address::Treasury,
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
        src: Address::Treasury,
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
    let min_diff = Difficulty(0x020fffff);

    let ziesha_token_creation_tx = get_ziesha_token_creation_tx();
    let ziesha_token_id = TokenId::new(&ziesha_token_creation_tx);

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
        max_block_size: (1 * MB) as usize,
        max_delta_count: 1024, // Only allow max of 1024 ZkScalar cells to be added per block
        block_time: 120,       // Seconds
        difficulty_calc_interval: 64, // Blocks

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
    use crate::core::RegularSendEntry;
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

    conf.genesis.block.body[1] = get_test_mpn_contract().tx;
    let abc = TxBuilder::new(&Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        src: Address::Treasury,
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
