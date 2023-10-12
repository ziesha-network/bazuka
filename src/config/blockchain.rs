use super::{initials, UNIT, UNIT_ZEROS};

use crate::blockchain::BlockchainConfig;
use crate::common::*;
use crate::core::{
    Amount, Block, ContractId, Header, Money, MpnAddress, ProofOfStake, Ratio, RegularSendEntry,
    Signature, Token, Transaction, TransactionAndDelta, TransactionData, ZkHasher,
};
use crate::mpn::circuits::MpnCircuit;
use crate::mpn::MpnConfig;
use crate::wallet::TxBuilder;
use crate::zk;

use std::collections::HashMap;
use std::str::FromStr;

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

const CHAIN_START_TIMESTAMP: u32 = 1685348591;

const MPN_LOG4_TREE_SIZE: u8 = 15;
const MPN_LOG4_TOKENS_TREE_SIZE: u8 = 3;
const MPN_LOG4_DEPOSIT_BATCH_SIZE: u8 = 3;
const MPN_LOG4_WITHDRAW_BATCH_SIZE: u8 = 3;
const MPN_LOG4_UPDATE_BATCH_SIZE: u8 = 4;
//pub const LOG4_SUPER_UPDATE_BATCH_SIZE: u8 = 5;

const TESTNET_HEIGHT_LIMIT: u64 = 25000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000600000000000000efebe260ff95b41e2ec915897b75099f9bab0b7b7a1b6a4596d59534cf299aef81c152a8650016185e75446b760a78068e09befad5f94c97c6d3ebef4182704923cff6f933ad7f7cf2180165485df15cbea58825c758050c2257ef7ea09b90030072c6c34887efc8bb217dcd323da064b5763a253a1c58f98c0a3cf4fda9112a2b56adae72c23971aea393719a26217d0f65f76a6db1307fec19fd16d824fb099ab6c54e86b5d54b436c790f28defc107dc5c51fd3733d935e168cbc73c1518f0900dee94d22c33f0c9045b057bd9d38887c773b3660a2f404b9e178891d1c62b7ea6afe989092897a6728fa4f8109269504b3474db6d4df6794f92e13cf3b1f339ca36038108e3d632c161c2999569b5dae7dde45704367d94f3e5deeaab84afd1800b07919c6ca2bc646c5dbf62dcfcf03ccc240e7132727c357c9a5875854bd4cba55c3bac7009ffeadd7680ceb5c4e720a9f8b0e832858098a360dacdce46c6f6136c48e0d6d3d0ed99b15a1a8ffa4ccca84ae76b35c510bdd58c8046fbb1d3d1600814578ee88d04b6aea9f4d027a412d6c937da0561d9fec4d72f806b69da710646bd79c52b8946ad28dfc23413e93ad084b917a63a1e53c1d66c2d4e5cd19aa683be455b64c98101cd6258fe0dceff14714dd336360a3be774529d9439f505d1700a0d59e9a342eff575db55f913d0e6d10aeadaf3efd42b63a8613fda4b0592d0dcfa8fd024d4f2c8e478d3ba3942d071171ff8dff06b196a42b6556420012746bbddf07a74143a48840523f17902bf00c70201a6a2bf0ffc1698a56c99b01130a00").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030006000000000000006e891448e9af99e350285e740434dc8c4446a0618686b0f4859c5fd230ef944dd132f8770e846e9be074327fbca97e06b4e7921417d6bd661323ab5c4ec088ea35f0d9c3efb85531f13160a983e3f9d41553af2d37ccd78835620c47e092c90400b01f5a1aeb2619bb5b25bed3d31f3a6cd2c0e5fa2a8221d9cb418b92122386d5d4b559a3da0d886f19a7a1342e2f4a045083c8f2c418bc2e952dfa6e6f96378d4ef41bb6f14c8b4e679d6d051f7fb9c7c1466d382e5dd7b7a2ab8dca7e2e021500e43daafb0111cb52e44cb390956e47eb0572106950ed2cdcf21bdbbe61dedcc770b5f56f93b83a967dc1bc33056e3e09c372138a568be5df603565d870ece1027359a82c6c5346ef4d660f4927324b52a5e17cc10fb66bf9f10b7b640209c11900c3a14821159e050f53c67cca563cab3c1670c637ca45988946e6c56771b25fb642816746d5185ddfb0a92b068b2e49123fd31f44881a91b6e5aff99c69687c5f33e71038eedd6a9753eeaebb416df0d216d602c78c1eca47e3db1b5951bcdb08008cb30ebffb4d23b4dced31c94e4e229c2c59c31e7506b7edb67ef284b1647b73bdb5e252a68e3883fb062213da67660b419574744e5aa899974fbae1e2d97e1585aaacb0a0a36c0a8f5ed55b403d3fce6a61a6bcfe21e6951a9cdf55e221f60300175a2ddd60eb2052083bb6383568dc600042a5dc0a88c59ce7f2e9f7efde62ecec0f72bef10b244d629850b84fd3241890008f16e087650b2c45d93335c840a21009bc7ca316be38c3c6ddb7a8dfd78a704da25a525c0815b7cbc5466863af1500").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd385280300060000000000000011ea58b6bc76321b19fcaa2a33622ecbe6ea52107d16903edbf82540436e189d1a9ab770aff0eb9f89ff5f95073b1318faafd6d365d4399b14374dadc3c158f8d72947037664e662a32f50ac5aa957c1388000c0ab24e0787fd5498a23b754010017d350c6e661ed2fc16bf6eeb7b07849c6e5f2d651c8bfbc4e474d961ed5a983d77ec57039fc6a7c436b939bf722dc1071d3f024328cdcbc8f5226650c154df56b689d1a395a70a3cf02101a2ac0a186fd13c4a252fae8e66810b8cd6d63e80400cd04e1830a08cb06ac7c8ad1a8c7997ddcce273883ff8cfc6b6e28efa884aa27cfd5ce47a27f629730e30fc4c6cb97006e0629fd0602354e014fda8892b1882703c285999d60fba785796510d945d29040d92e6f95b7ff02b8a2f93641ba1f1200f37d5baf73c30469facd22e2ac46a236d5af926a06767f2898610bb4c1e9d4b0c21b96c817e57483043965f3ec87fa0788ad43d6287940bb2b41cbc88441a08adda1b7b47c40fc6216685e5eb28c95bb4ddd4ababd9651a1179a12c61332430800aa71b062dc4ff5c2415703df7209e747d0a96df5d7974be2a35a609617addd5b808099510f1cd3a73dc3e10c521721151e9dae43bc6ccbdc67ea222febdb84b79c1fec7d81b75d42c9614a460db9edca13582385ca51f9f711acb0b1b3c60e0e00a8a13413318e77b4952484a8006ff65cd604a7ff3a323863a1e7bde7169b2dda7c8e1d34a983a872eeda53a3e815f505b32034e2aa5b4f5785c62be937f243749874db9b6a41bc0df18f3e19a1afbe7a03f68f8723999cf88f69044b0aa8020000").unwrap()).unwrap();
}

fn get_mpn_contract(
    log4_tree_size: u8,
    log4_token_tree_size: u8,
    log4_deposit_batch_size: u8,
    log4_withdraw_batch_size: u8,
    initial_balances: &[(MpnAddress, Amount)],
) -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::List {
        log4_size: log4_tree_size,
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Tx-Nonce
                zk::ZkStateModel::Scalar, // Withdraw-Nonce
                zk::ZkStateModel::Scalar, // Pub-key X
                zk::ZkStateModel::Scalar, // Pub-key Y
                zk::ZkStateModel::List {
                    log4_size: log4_token_tree_size,
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

    let mut sum_amount = Amount(0);
    let mut data = zk::ZkDataPairs(HashMap::new());
    let mut state_builder = zk::ZkStateBuilder::<ZkHasher>::new(mpn_state_model.clone());
    for (i, (addr, amount)) in initial_balances.iter().enumerate() {
        sum_amount = sum_amount + *amount;
        let addr = addr.pub_key.0.decompress();
        data.0.insert(
            zk::ZkDataLocator(vec![i as u64, 2]),
            zk::ZkScalar::from(addr.0),
        );
        data.0.insert(
            zk::ZkDataLocator(vec![i as u64, 3]),
            zk::ZkScalar::from(addr.1),
        );
        data.0.insert(
            zk::ZkDataLocator(vec![i as u64, 4, 0, 0]),
            zk::ZkScalar::from(ContractId::Ziesha),
        );
        data.0.insert(
            zk::ZkDataLocator(vec![i as u64, 4, 0, 1]),
            zk::ZkScalar::from(*amount),
        );
        state_builder
            .batch_set(&zk::ZkDeltaPairs(
                [
                    (
                        zk::ZkDataLocator(vec![i as u64, 2]),
                        Some(zk::ZkScalar::from(addr.0)),
                    ),
                    (
                        zk::ZkDataLocator(vec![i as u64, 3]),
                        Some(zk::ZkScalar::from(addr.1)),
                    ),
                    (
                        zk::ZkDataLocator(vec![i as u64, 4, 0, 0]),
                        Some(zk::ZkScalar::from(ContractId::Ziesha)),
                    ),
                    (
                        zk::ZkDataLocator(vec![i as u64, 4, 0, 1]),
                        Some(zk::ZkScalar::from(*amount)),
                    ),
                ]
                .into(),
            ))
            .unwrap();
    }

    let mpn_contract = zk::ZkContract {
        token: None,
        state_model: mpn_state_model.clone(),
        initial_state: state_builder.compress().unwrap(),
        deposit_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_DEPOSIT_VK.clone())),
            log4_payment_capacity: log4_deposit_batch_size,
        }],
        withdraw_functions: vec![zk::ZkMultiInputVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_WITHDRAW_VK.clone())),
            log4_payment_capacity: log4_withdraw_batch_size,
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
            state: Some(data),
            money: Money::ziesha(sum_amount.into()),
        },
        nonce: 0, // MPN contract is created after Ziesha token is created
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
    let mut mpn_tx_delta = get_mpn_contract(30, 1, 1, 1, &[]);
    let mpn_state_model = zk::ZkStateModel::List {
        log4_size: 30,
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Tx-Nonce
                zk::ZkStateModel::Scalar, // Withdraw-Nonce
                zk::ZkStateModel::Scalar, // Pub-key X
                zk::ZkStateModel::Scalar, // Pub-key Y
                zk::ZkStateModel::List {
                    log4_size: 1,
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
    match &mut mpn_tx_delta.tx.data {
        TransactionData::CreateContract { contract, .. } => {
            contract.state_model = mpn_state_model;
            contract.initial_state =
                zk::ZkCompressedState::empty::<ZkHasher>(contract.state_model.clone());
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
    mpn_tx_delta.state_delta = Some(zk::ZkDeltaPairs::default());
    mpn_tx_delta
}

fn get_ziesha_token_creation_tx() -> Transaction {
    Transaction {
        memo: "Happy Birthday Ziesha!".into(),
        src: None,
        data: TransactionData::CreateContract {
            contract: zk::ZkContract {
                token: Some(zk::ZkTokenContract {
                    token: Token {
                        name: "Ziesha".into(),
                        symbol: "ZSH".into(),
                        supply: Amount(2_000_000_000_u64 * UNIT),
                        decimals: UNIT_ZEROS,
                        minter: None,
                    },
                    mint_functions: vec![],
                }),
                state_model: zk::ZkStateModel::Scalar,
                initial_state: zk::ZkCompressedState::empty::<ZkHasher>(zk::ZkStateModel::Scalar),
                deposit_functions: vec![],
                withdraw_functions: vec![],
                functions: vec![],
            },
            state: Some(Default::default()),
            money: Money::ziesha(0),
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    }
}

pub fn get_blockchain_config() -> BlockchainConfig {
    blockchain_config_template(true)
}

pub fn blockchain_config_template(initial_balances: bool) -> BlockchainConfig {
    let init_balances = initials::initial_mpn_balances();
    let mpn_tx_delta = get_mpn_contract(
        MPN_LOG4_TREE_SIZE,
        MPN_LOG4_TOKENS_TREE_SIZE,
        MPN_LOG4_DEPOSIT_BATCH_SIZE,
        MPN_LOG4_WITHDRAW_BATCH_SIZE,
        if initial_balances {
            &init_balances
        } else {
            &[]
        },
    );
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let ziesha_token_creation_tx = get_ziesha_token_creation_tx();
    let ziesha_token_id = ContractId::new(&ziesha_token_creation_tx);

    let create_staker = Transaction {
        memo: "Very first staker created!".into(),
        src: Some(
            "ed744735b5239d32a5b5b6441474bf65a6aaa6bfcf8905d4616f1acc14cf3847f0"
                .parse()
                .unwrap(),
        ),
        data: TransactionData::UpdateStaker {
            vrf_pub_key: "vrf2a3531b9978e7d1293fa58b4f04cb8d78c72f681b58cd664703c3b0f2a531e04"
                .parse()
                .unwrap(),
            commission: Ratio(12), // 12/255 ~= 5%
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    let delegate_to_staker = Transaction {
        memo: "Very first delegation!".into(),
        src: None,
        data: TransactionData::Delegate {
            to: "ed744735b5239d32a5b5b6441474bf65a6aaa6bfcf8905d4616f1acc14cf3847f0"
                .parse()
                .unwrap(),
            amount: Amount(1000000000000),
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };

    let mut blk = Block {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            block_root: Default::default(),
            proof_of_stake: ProofOfStake {
                timestamp: CHAIN_START_TIMESTAMP,
                validator: Default::default(),
                proof: None,
            },
        },
        body: vec![
            ziesha_token_creation_tx,
            mpn_tx_delta.tx,
            create_staker,
            delegate_to_staker,
        ],
    };

    for (dst, amnt) in initials::initial_balances().into_iter() {
        blk.body.push(Transaction {
            memo: "".into(),
            src: None,
            data: TransactionData::RegularSend {
                entries: vec![RegularSendEntry {
                    dst,
                    amount: Money {
                        token_id: ContractId::Ziesha,
                        amount: amnt,
                    },
                }],
            },
            nonce: 0,
            fee: Money::ziesha(0),
            sig: Signature::Unsigned,
        });
    }

    BlockchainConfig {
        limited_miners: None,
        mpn_config: MpnConfig {
            mpn_contract_id,
            log4_tree_size: MPN_LOG4_TREE_SIZE,
            log4_token_tree_size: MPN_LOG4_TOKENS_TREE_SIZE,
            log4_deposit_batch_size: MPN_LOG4_DEPOSIT_BATCH_SIZE,
            log4_withdraw_batch_size: MPN_LOG4_WITHDRAW_BATCH_SIZE,
            log4_update_batch_size: MPN_LOG4_UPDATE_BATCH_SIZE,
            mpn_num_update_batches: 1,
            mpn_num_deposit_batches: 1,
            mpn_num_withdraw_batches: 1,
            deposit_vk: zk::ZkVerifierKey::Groth16(Box::new(MPN_DEPOSIT_VK.clone())),
            withdraw_vk: zk::ZkVerifierKey::Groth16(Box::new(MPN_WITHDRAW_VK.clone())),
            update_vk: zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone())),
        },

        ziesha_token_id,
        genesis: blk,
        reward_ratio: 10_000_000, // 1/10_000_000 -> 0.0001% of Treasury Supply per block
        max_block_size: MB as usize,

        testnet_height_limit: Some(TESTNET_HEIGHT_LIMIT),
        max_memo_length: 64,
        slot_duration: 90,
        slot_per_epoch: 10,
        chain_start_timestamp: CHAIN_START_TIMESTAMP,
        check_validator: true,
        max_validator_commission: Ratio(26), // 26 / 255 ~= 10%

        teleport_log4_tree_size: 10,
        teleport_contract_id: ContractId::from_str(
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap(),
    }
}

pub fn get_dev_blockchain_config(
    validator: &TxBuilder,
    user: &TxBuilder,
    small_mpn: bool,
) -> BlockchainConfig {
    let mut conf = get_blockchain_config();

    if small_mpn {
        let log4_tree_size = 10;
        let log4_token_tree_size = 1;
        let log4_deposit_batch_size = 1;
        let log4_withdraw_batch_size = 1;
        let log4_update_batch_size = 1;

        let mut rng = ChaChaRng::from_seed([0u8; 32]);

        log::info!("Generating MPN params...");
        let deposit_params =
            bellman::groth16::generate_random_parameters::<bls12_381::Bls12, _, _>(
                crate::mpn::circuits::DepositCircuit::empty(
                    log4_tree_size,
                    log4_token_tree_size,
                    log4_deposit_batch_size,
                ),
                &mut rng,
            )
            .unwrap();
        let withdraw_params =
            bellman::groth16::generate_random_parameters::<bls12_381::Bls12, _, _>(
                crate::mpn::circuits::WithdrawCircuit::empty(
                    log4_tree_size,
                    log4_token_tree_size,
                    log4_withdraw_batch_size,
                ),
                &mut rng,
            )
            .unwrap();
        let update_params = bellman::groth16::generate_random_parameters::<bls12_381::Bls12, _, _>(
            crate::mpn::circuits::UpdateCircuit::empty(
                log4_tree_size,
                log4_token_tree_size,
                log4_update_batch_size,
            ),
            &mut rng,
        )
        .unwrap();
        log::info!("Done generating MPN params!");

        conf.mpn_config = MpnConfig {
            mpn_contract_id: conf.mpn_config.mpn_contract_id,
            log4_tree_size,
            log4_token_tree_size,
            log4_deposit_batch_size,
            log4_withdraw_batch_size,
            log4_update_batch_size,
            mpn_num_update_batches: 1,
            mpn_num_deposit_batches: 1,
            mpn_num_withdraw_batches: 1,
            deposit_vk: zk::ZkVerifierKey::Groth16(Box::new(deposit_params.vk.clone().into())),
            withdraw_vk: zk::ZkVerifierKey::Groth16(Box::new(withdraw_params.vk.clone().into())),
            update_vk: zk::ZkVerifierKey::Groth16(Box::new(update_params.vk.clone().into())),
        };
    }

    conf.genesis.body[2] = Transaction {
        memo: "Very first staker created!".into(),
        src: Some(validator.get_address()),
        data: TransactionData::UpdateStaker {
            vrf_pub_key: validator.get_vrf_public_key(),
            commission: Ratio(12), // 12/255 ~= 5%
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    conf.genesis.body[3] = Transaction {
        memo: "Very first delegation!".into(),
        src: None,
        data: TransactionData::Delegate {
            to: validator.get_address(),
            amount: Amount(1000000000000),
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    conf.genesis.body.push(Transaction {
        memo: "Initial user balance".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: user.get_address(),
                amount: Money::ziesha(100_000_000_000),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });
    conf
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let mut conf = blockchain_config_template(false);
    conf.limited_miners = None;
    conf.mpn_config = MpnConfig {
        mpn_contract_id,
        log4_tree_size: 30,
        log4_token_tree_size: 1,
        log4_deposit_batch_size: 1,
        log4_withdraw_batch_size: 1,
        log4_update_batch_size: 1,
        mpn_num_update_batches: 0,
        mpn_num_deposit_batches: 0,
        mpn_num_withdraw_batches: 0,
        deposit_vk: zk::ZkVerifierKey::Dummy,
        withdraw_vk: zk::ZkVerifierKey::Dummy,
        update_vk: zk::ZkVerifierKey::Dummy,
    };
    conf.testnet_height_limit = None;
    conf.chain_start_timestamp = 0;
    conf.check_validator = false;
    conf.slot_duration = 5;
    conf.reward_ratio = 100_000;

    conf.genesis.body[1] = get_test_mpn_contract().tx;
    conf.genesis.body.drain(2..);
    conf.genesis.header.proof_of_stake.timestamp = 0;

    let abc = TxBuilder::new(&Vec::from("ABC"));
    let validator_1 = TxBuilder::new(&Vec::from("VALIDATOR"));
    let validator_2 = TxBuilder::new(&Vec::from("VALIDATOR2"));
    let validator_3 = TxBuilder::new(&Vec::from("VALIDATOR3"));
    conf.genesis.body.push(Transaction {
        memo: "Dummy tx".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: abc.get_address(),
                amount: Money::ziesha(10000),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    let delegator = TxBuilder::new(&Vec::from("DELEGATOR"));
    conf.genesis.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: delegator.get_address(),
                amount: Money::ziesha(100),
            }],
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    for val in [validator_1, validator_2, validator_3].into_iter() {
        conf.genesis.body.push(
            val.register_validator(
                "Test validator".into(),
                Ratio(12), // 12/256 ~= 5%
                Money::ziesha(0),
                0,
            )
            .tx,
        );
        conf.genesis.body.push(
            delegator
                .delegate(
                    "".into(),
                    val.get_address(),
                    Amount(25),
                    Money::ziesha(0),
                    0,
                )
                .tx,
        );
    }
    conf
}
