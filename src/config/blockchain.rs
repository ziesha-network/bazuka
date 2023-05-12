use super::{initials, UNIT, UNIT_ZEROS};

use crate::blockchain::BlockchainConfig;
use crate::common::*;
use crate::core::{
    Amount, Block, ContractDeposit, ContractId, ContractUpdate, Header, Money, MpnAddress,
    ProofOfStake, Ratio, RegularSendEntry, Signature, Token, TokenId, Transaction,
    TransactionAndDelta, TransactionData, ValidatorProof, ZkHasher,
};
use crate::mpn::circuits::MpnCircuit;
use crate::mpn::MpnConfig;
use crate::wallet::TxBuilder;
use crate::zk;

use rand::SeedableRng;
use rand_chacha::ChaChaRng;

const CHAIN_START_TIMESTAMP: u32 = 1678976362;

const MPN_LOG4_TREE_SIZE: u8 = 15;
const MPN_LOG4_TOKENS_TREE_SIZE: u8 = 3;
const MPN_LOG4_DEPOSIT_BATCH_SIZE: u8 = 3;
const MPN_LOG4_WITHDRAW_BATCH_SIZE: u8 = 3;
const MPN_LOG4_UPDATE_BATCH_SIZE: u8 = 4;
//pub const LOG4_SUPER_UPDATE_BATCH_SIZE: u8 = 5;

const TESTNET_HEIGHT_LIMIT: u64 = 10000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000d27229e4a5b35a6e4a5b36779546cd8b4ce1158d94ac4afd1a8f2cac514a6a36b35aeed48662799cc9f7e1a0ec109214496eb0bf6c38b21592c0aca735e82cd708305c4b7fdf845fd931923976cf897bed002d4987eec63c24e8e136a9146e0000068a98e019da925258a729c4bf58cb60da38377a799f410ea47ca8842c3c57b2460a85ff10542210c5f51069592b5a137fa2945c52fa37806abc49dc1aaf1e6d02a6927b4bb9250ec648bc399fa2e9011180834b7450e4844bff1f9bbc13310800418685945f12d394672fbf688204b85815f7b1967a85744b892e27819927ff5d146a0ff41f4e4818485d5ccf1a16b40fb531d9872fb13ed338a895a10eb0ed7c5341ea9a86419de8314ca242e6fb38a77ee123464980b293c2b6d03db0e6ba0800e0a21c4593a1b2639acf058836ea2459432b64a06ad9c063c3c15adf158d6cf866652c0e90092eace3edc26a5bb54b0416b81234f683eb95f211afd4f37fa6072ffdc2a16378d301094f2a8e5784d6abb5365f29ac7724f804b4978b78230f0a00fcd60ec8d5417cb5743761ef4e6d75c4b7bdbbb6f73fcb04c394d765cabd692a54dfe2d4f13e4f0859229c314580db096f5dbc0f07388324525a433c154e52d2be36aa8ee59922297acd54bbc7b3ad47d7973cb6361514de7475470af34bf11300").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000520e13fbb9cd68bba031ad772ac3c90fd07bd1b89ed44a42b35f88ebce164439d10322b35a6222ff5f2eb85584c4960f246ade55f46373a6ae0d3833c30b2d62c38c6d8a6452d3a29d81c0ed0a406b4c93d746f38f88c4aa1f3d64f42f5f35110087850f500734ec095b11a729fe2f7a7703692fd74f42eb3b3f43335fb79c8f6fdca61fd455a58052d9b03d133bc3720132bf11e1d16c001236eb8c502d6c6d2942162366bfe6e2c2bf222c6d49af7400c3d0d14e212487c5056a4dd99339ad1000789dad758f3d58eb2f6e2164a5da684778656da6be68e9f9686ef48ec70580560a66ee715bb52271c0c824983f753b0a0ce001d54b2f2deb3b8192c08f5ef7464a893489b5d8941b23e51e454570264a0babaf49090c3b572756b08d81c4bb1800dd1cb82c2194fb073b90875a7c6c1563d3757c4111d1c8db2ef481d08bc44210ed073e40dadb11da936f4f61743c2f00faf3343f1c5740e88530c40b4da4253aac09da4bcc32b02ffe5d51dfacc08d68c6c3796f252afeff1c7661bf0982da0c002a959c75275d7585d7fa62a52e00d5b51ab25d0a86c72adb30e80210c26245d868be7ba79b148339025cc31ecbff940b80129c3b5fad3339ce5ba2fbd2dfe420c8d46c4f5dd00da11564a2dd8369f61619cf5489de20de2b77ca847b4d4df80100").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000002ba930a0767ba3d8d7c11ff3309bf2edcf2b9447b51d5fed2143fa9b3e6c693355a85885931e47270d81dac722c998075f8f27f5544f9704f048f6fe4f9da256cc3cb066f7291d09bbd88cc99fef894ab04dec6c72c450e28c979001da117607004f99be417796e07e92042fce2f5bae251310cabb2ee198e9928767d34b670d79813c037819c5ebbde56aeb020e387017d0535465c8d7a7b791d5fb3e1951ec8fc5bd9a11eb34f8ef531bfa52d040c80f6e3aa233a97714f1a04061208f0ebd19000662f6ed251eca04bbd44979d65c24b4c1de1f138cca608c4cf1fb226920e4e06a8f078cc6235764379eed23d15a4d145acb4349bc5c3fad4599258833c5f0c4d57cf5ae572d26cb20b2e055d4d6017e73435d6cf84ae07cdd82c620745a5b1500128ac10cd80810ada04a30c934b7fe1c3ad0ac876208b741793ec46efeb6ebaa3a0515ccf3d8de5f9e318464955d4b12e9407ef1950206bb6097c7030cc26064d7d01c2ef0e6b03fd4ab0252351fd202197670151490edb4d176c44683a928030095a10062e2d6e8f066dc069e844b68aa6a12b329508f62be9e494d8441dfc93e20d72b7249c4772293b3ce360fa77b1090cbe17ecb30eb535cb111a7b7e9bddb36fc62ea5b18a57d9b00b0c1b5ad8356d3337cbe61e789ecb0d57ae642afcd1700").unwrap()).unwrap();
}

fn get_mpn_contract(
    log4_tree_size: u8,
    log4_token_tree_size: u8,
    log4_deposit_batch_size: u8,
    log4_withdraw_batch_size: u8,
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
    let mpn_contract = zk::ZkContract {
        state_model: mpn_state_model.clone(),
        initial_state: zk::ZkCompressedState::empty::<ZkHasher>(mpn_state_model),
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
            state: Some(Default::default()), // Empty
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
    let mut mpn_tx_delta = get_mpn_contract(30, 1, 1, 1);
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
        data: TransactionData::CreateToken {
            token: Token {
                name: "Ziesha".into(),
                symbol: "ZSH".into(),
                supply: Amount(2_000_000_000_u64 * UNIT),
                decimals: UNIT_ZEROS,
                minter: None,
            },
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    }
}

pub fn get_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_mpn_contract(
        MPN_LOG4_TREE_SIZE,
        MPN_LOG4_TOKENS_TREE_SIZE,
        MPN_LOG4_DEPOSIT_BATCH_SIZE,
        MPN_LOG4_WITHDRAW_BATCH_SIZE,
    );
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let ziesha_token_creation_tx = get_ziesha_token_creation_tx();
    let ziesha_token_id = TokenId::new(&ziesha_token_creation_tx);

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
                proof: ValidatorProof::Unproven,
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
                        token_id: TokenId::Ziesha,
                        amount: amnt,
                    },
                }],
            },
            nonce: 0,
            fee: Money::ziesha(0),
            sig: Signature::Unsigned,
        });
    }

    let deps = initials::initial_mpn_balances()
        .chunks(64)
        .map(|chunk| ContractUpdate::Deposit {
            deposit_circuit_id: 0,
            deposits: chunk
                .iter()
                .map(|(addr, amount)| ContractDeposit {
                    memo: "".into(),
                    contract_id: mpn_contract_id,
                    deposit_circuit_id: 0,
                    calldata: Default::default(),
                    src: Default::default(),
                    amount: Money {
                        token_id: TokenId::Ziesha,
                        amount: *amount,
                    },
                    fee: Money::ziesha(0),
                    nonce: 0,
                    sig: None,
                })
                .collect::<Vec<_>>(),
            next_state: Default::default(),
            proof: Default::default(),
        })
        .collect::<Vec<_>>();
    blk.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::UpdateContract {
            contract_id: mpn_contract_id,
            updates: vec![],
            delta: Some(Default::default()),
        },
        nonce: 0,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

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
        reward_ratio: 100_000, // 1/100_000 -> 0.01% of Treasury Supply per block
        max_block_size: MB as usize,

        testnet_height_limit: Some(TESTNET_HEIGHT_LIMIT),
        max_memo_length: 64,
        slot_duration: 60,
        slot_per_epoch: 10,
        chain_start_timestamp: CHAIN_START_TIMESTAMP,
        check_validator: true,
        max_validator_commission: Ratio(26), // 26 / 255 ~= 10%
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

    let mut conf = get_blockchain_config();
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
