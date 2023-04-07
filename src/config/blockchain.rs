use super::{UNIT, UNIT_ZEROS};

use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::common::*;
use crate::core::{
    Amount, Block, ContractId, Header, Money, ProofOfStake, Signature, Token, TokenId, Transaction,
    TransactionAndDelta, TransactionData, ValidatorProof, ZkHasher,
};
use crate::wallet::TxBuilder;

use crate::mpn::MpnConfig;
use crate::zk;

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
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd385280300050000000000000043a6f266a47b03bf88f2e8fb15fb3b62d20e12b405a07f68d84b69de8a9c52b77149e9856ace5913d91955911dad1e060fe5fba59c82c7629ac19db1b1760672a3d34215e22c77c7b481297d7efe062e487d8db8643909a2654c79874516261100c9b604e4c7f22c43dee2283240d0c2674fb85723a67db1d3d5c155377aa3292cee05b6c43b27a1fe04d8878e288a6e0a61949e5da8f5b7a99af76f5fa65241b915d72a16bbf0d955895a08928ef87e887edd499f717722976e8b480fa78a0604009ec0881aefe81b75e6929d68c56910dad75d478c31bbb967a6955b557035d78de677634e91302adf5ab14acab5f5830e879b2724d16efe5e3bb96dd4e142ff07307a8a0dd9bd773383ef040084214e1fce0e4392ae140897bb4ea42bcce0a10d00c3666e30e7404067260d54ecb1602bcb9430efc437b303ad37f07b23879c3034600674f6312d98b4b609ef49eecb19166dd619041ce11112f183d7e8489c384c2279bd3cd78ef06b34a47e27e7f2239460faae4add23039d5878a5e73834c809009aa92b0e69272ad538986fd0ae0888df53c71fb9eae573e0eccba522cc3f75e28a0bf56b875197857359966c032da811041ad13ca15a76d9eafe616b0e1b4581ec8bb2dd44f77f92140ea154f5dd3f0f6ad196e19b08bf67743751b7e9727b1100").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000fc2bb82de4e64ed68a318be1fd04643567b2453cc9c6a4d7858a69022dafb97857534eb5db1744fcd9fc7e15caf7c1023c50b228324830fdff420f2d77eb8ec0c383b11135459178e9d35b0fb4b0ad181d60e411917761a9225e10785d05121000f6aa6abae9f840884133cd887c01d668b8f773962a6e84d40e3d22824f0a7da81c03bcabf9fa0bb027f834342deec7100a8b37e7080e90c14787f8f84e6d6da064a2453ffd7f303a627f89ea6f5a53cf3339540011ddee7753bc940fa5cd8e0200454e2720a2c5c80a915b677648ea8ce983c2e86e8e9f9a29b6a0410e5f6e0c028550374e82f94cf14ece889bd7de2c0c9984bf5ed92155ca29f283e828af920964ca54cf92550b7c9042424b3f6a80559159e4f1c9495363d43779445b7f6d09005a138f44b5a571fb34f6cceb5bd492860d84e090e9faa00068ec6c263b5d210a65c10bf44acd1c3620bec0e2ecfc21125dc6ba51e58498cf49d0a89a7fb78bad5655592456c42b5d774020ea3b643ac09642d84e681992317092b3fcc870a21500aa74023e03bfe3ce19205b517d07421406ea93235f9b62b8a9c53feb419aab6c15deafe65a307078c67934f0e954050f68b93fcdc0db45a295acc8ce15d80c9df416a66b6e632586439cfbedeb00d1ee7aaf3c43e4f8ae4c781e40b4878cef1700").unwrap()).unwrap();
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
        TransactionData::CreateContract { contract } => {
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
        nonce: 1,
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
            "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
                .parse()
                .unwrap(),
        ),
        data: TransactionData::UpdateStaker {
            vrf_pub_key: "vrf666384dd335e559a564d432b0623f6c2791e794ecd964845d47b1a350ade6866"
                .parse()
                .unwrap(),
            commision: 12, // 12/255 ~= 5%
        },
        nonce: 1,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    let delegate_to_staker = Transaction {
        memo: "Very first delegation!".into(),
        src: None,
        data: TransactionData::Delegate {
            to: "edae9736792cbdbab2c72068eb41c6ef2e6cab372ca123f834bd7eb59fcecad640"
                .parse()
                .unwrap(),
            amount: Amount(1000000000000),
            reverse: false,
        },
        nonce: 3,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };

    let blk = Block {
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

        testnet_height_limit: Some(TESTNET_HEIGHT_LIMIT),
        max_memo_length: 64,
        slot_duration: 60,
        slot_per_epoch: 10,
        chain_start_timestamp: CHAIN_START_TIMESTAMP,
        check_validator: true,
        max_validator_commision: 26, // 26 / 255 ~= 10%
    }
}

pub fn get_dev_blockchain_config(validator: &TxBuilder) -> BlockchainConfig {
    let mut conf = get_blockchain_config();

    conf.mpn_config.mpn_num_update_batches = 0;
    conf.mpn_config.mpn_num_deposit_batches = 0;
    conf.mpn_config.mpn_num_withdraw_batches = 0;

    conf.genesis.block.body[2] = Transaction {
        memo: "Very first staker created!".into(),
        src: Some(validator.get_address()),
        data: TransactionData::UpdateStaker {
            vrf_pub_key: validator.get_vrf_public_key(),
            commision: 12, // 12/255 ~= 5%
        },
        nonce: 1,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    conf.genesis.block.body[3] = Transaction {
        memo: "Very first delegation!".into(),
        src: None,
        data: TransactionData::Delegate {
            to: validator.get_address(),
            amount: Amount(1000000000000),
            reverse: false,
        },
        nonce: 3,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    conf
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    use crate::core::RegularSendEntry;
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

    conf.genesis.block.body[1] = get_test_mpn_contract().tx;
    conf.genesis.block.body.drain(2..);
    conf.genesis.block.header.proof_of_stake.timestamp = 0;

    let abc = TxBuilder::new(&Vec::from("ABC"));
    let validator_1 = TxBuilder::new(&Vec::from("VALIDATOR"));
    let validator_2 = TxBuilder::new(&Vec::from("VALIDATOR2"));
    let validator_3 = TxBuilder::new(&Vec::from("VALIDATOR3"));
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

    let delegator = TxBuilder::new(&Vec::from("DELEGATOR"));
    conf.genesis.block.body.push(Transaction {
        memo: "".into(),
        src: None,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: delegator.get_address(),
                amount: Money::ziesha(100),
            }],
        },
        nonce: 4,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    });

    for (i, val) in [validator_1, validator_2, validator_3]
        .into_iter()
        .enumerate()
    {
        conf.genesis.block.body.push(
            val.register_validator(
                "Test validator".into(),
                12, // 12/256 ~= 5%
                Money::ziesha(0),
                1,
            )
            .tx,
        );
        conf.genesis.block.body.push(
            delegator
                .delegate(
                    "".into(),
                    val.get_address(),
                    Amount(25),
                    false,
                    Money::ziesha(0),
                    (i + 1) as u32,
                )
                .tx,
        );
    }

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
