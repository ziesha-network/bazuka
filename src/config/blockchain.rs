use super::{UNIT, UNIT_ZEROS};

use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::common::*;
use crate::core::{
    Amount, Block, ContractId, Header, Money, ProofOfStake, Signature, Token, TokenId, Transaction,
    TransactionAndDelta, TransactionData, ValidatorProof, ZkHasher,
};
use crate::mpn::MpnConfig;
use crate::zk;

#[cfg(test)]
use crate::wallet::TxBuilder;

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
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000009e32c1b2ae4dec38f992b57354fa4ee427cda9f96029f73423802a33698a59956b11b2647df14923f8d589d287015204b4045595d42bb170b1987a3054ce421fa2e915e6f89f2f0d87a9b6c21b9b331dd21d95156d997f96d3a4c8d2ce6fad140081068a7c574d8645884f272481c7058213d179056dc5e8ee351d2f112ad9e61663b4adae6082f8eb4a02e7b77b903313a1626183dad961db4f49f328db076050de2fd561c58e35056923a5903b8f2d120dfbbe4682135c3d6ae07f10ad31291800ea0dfc087e0a0c940427257f4ab184cce8b4f414b43020a4a684e623291011f1c3c6f45fd932939d0aa347ec3a60aa06b20495b8d5d893659dabd0693df0e20dededcaf9e0bb628a8c9d64b2c75f8f69909afe2354fc789689c0dfe932363e02007ab29705c8363a5adf22597298614892a2480c1168c17d69a222ddce7ce792a78036e59b55a7b54fd021f2433e3a1a1119f178ce2d7744cba196e512420462b33f21007f83b4c9ea8b506dbdbe51b24a670ab29d85a44fe011b0326fc8eb800f0095515c8ce5dad806470d0700135d47e0bb114137150e8eb7f69a072a5af8223c4c4c2122f7a2a2158efde1a3914a5d04019bb405ec2ebf18cb35099b870454fb4de30b935859f5b62570d06a309fbfaa88b5edfc04af29f7563a000d4c49180300").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000fc2bb82de4e64ed68a318be1fd04643567b2453cc9c6a4d7858a69022dafb97857534eb5db1744fcd9fc7e15caf7c1023c50b228324830fdff420f2d77eb8ec0c383b11135459178e9d35b0fb4b0ad181d60e411917761a9225e10785d05121000f6aa6abae9f840884133cd887c01d668b8f773962a6e84d40e3d22824f0a7da81c03bcabf9fa0bb027f834342deec7100a8b37e7080e90c14787f8f84e6d6da064a2453ffd7f303a627f89ea6f5a53cf3339540011ddee7753bc940fa5cd8e0200454e2720a2c5c80a915b677648ea8ce983c2e86e8e9f9a29b6a0410e5f6e0c028550374e82f94cf14ece889bd7de2c0c9984bf5ed92155ca29f283e828af920964ca54cf92550b7c9042424b3f6a80559159e4f1c9495363d43779445b7f6d09005a138f44b5a571fb34f6cceb5bd492860d84e090e9faa00068ec6c263b5d210a65c10bf44acd1c3620bec0e2ecfc21125dc6ba51e58498cf49d0a89a7fb78bad5655592456c42b5d774020ea3b643ac09642d84e681992317092b3fcc870a21500aa74023e03bfe3ce19205b517d07421406ea93235f9b62b8a9c53feb419aab6c15deafe65a307078c67934f0e954050f68b93fcdc0db45a295acc8ce15d80c9df416a66b6e632586439cfbedeb00d1ee7aaf3c43e4f8ae4c781e40b4878cef1700").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000004cac599e421ab5b7d91e96742df84f3b218935f5d2c63ad2e345495002a72594b075161287daf145bc089b19c9d86c046db3e94f1b288ffd778e83ab3b198410a2038802eaa74dde50ff4a3d372701d9ffebf70e92a415e1ce8d2f924121aa0200c9e7ff8a6fdf715546003a73bfff721b03370984cf0596ccc87ebc31e6f82d610891d6b4f8601aa3cfc67a4d0c2f0e13c856e3cce10dfe23efd16d9a516f11c4c19f15e773f2265cdb0525eda50069c01ef70812999d4fa9bdada1b0caa6330c0074d419a5e80f2be388854cad27f3935f141d67bbdcad61eee847cddfd7b743ad6b51b2b2b96d1a088646c149f97898088687c0f4611a8ea2e56cb6442e951bf2055e3310004ad9082c3143535713899ad179b99fbb1304cd33ce3e25dabcc20700f74fa51b74c32e590ee93ea89d9f3624e0762c90aab9f16ccdb6469cd9cdab1a9ca5999b1f126adeee4f1017ac0965175963ebc1cf64ccb074b1135f97dc2f29cc76fbc533e285985082ca092a0d8cf1064bed6a043113e14c93faba76122e1600bc097e9e2eb7a81f95a483427b8dc3681dbf085bbb23309889cb7f58f65af095d84cd03083d73744e5a2885b2e471a177062929a0bb2b2dd1d1ca13479595e10164555faa00d958dde62422600b06c696235b9785f21de8d3d1b8abb3d6e5a0300").unwrap()).unwrap();
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
