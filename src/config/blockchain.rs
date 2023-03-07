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

const CHAIN_START_TIMESTAMP: u32 = 1677770301 - 60 * 5;

const MPN_LOG4_TREE_SIZE: u8 = 15;
const MPN_LOG4_TOKENS_TREE_SIZE: u8 = 3;
const MPN_LOG4_DEPOSIT_BATCH_SIZE: u8 = 3;
const MPN_LOG4_WITHDRAW_BATCH_SIZE: u8 = 3;
const MPN_LOG4_UPDATE_BATCH_SIZE: u8 = 4;
//pub const LOG4_SUPER_UPDATE_BATCH_SIZE: u8 = 5;

const TESTNET_HEIGHT_LIMIT: u64 = 10000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd38528030005000000000000005d1f971a1c209fdffcfc94c564d972ddcd980b421783d50810724bb935cfda56747e4198e25d4191441a960a57a4e818de1ba9ed81027186437897daf12b241e5d65b9856bcbeb50f03f3de6fe9689976c16d1e8168ec688be1e48770fe2e51400a730c02ba017e1d6e44fafeb802c88c126c4be2e39355de3f432070017b5ca9537d21b94f9179caccc2942506b245f0c10dee8b12d022cdcfd46fd2d0297724262532d4eb846f023115dbeca2fba7b0c0011aee0bb01270ec67c14ce656f2f0200278b42b32043309e1181306c032309d417aef342759dd8c06b5dd5993635d0c8b47633166df858fbf5d7bacedb39af10ccd89be5f30e585d8ff0c8a54c03ff4b69c585eb21c687517dbabc76720b2f32b4f5844e8f2d0baa23e36d90e1c635190003ba146c20bf2d3e8ccd7f9f67b2da3fcdc8248d43b4dc4c7c0832aa69064bafb876ed4f86f7c22de3c92df8788825149ad5e1bf741d303451d49419785f15c866f74e058e0ddb7a00fd9b0c460b3ed6069bb5f4cca4e111ba5acca063bd0e1900d50906b204f773ab4f7cde9983186f6e02666719ec0381c118f06ac589dbcd8ee2fb93a03925091a7a5524fa33b7f60527acfd34bf39e553554d4a1eed1e6be5d97a090bbf78974af8b789c76e0acfa9cf4311a24e71abea818812ee209c001a00").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd385280300050000000000000006ceb2c072c2c5cad57c4a13e5fc709f5bdb9bc78485a17310d1e136759cc6d33557787145f779e195fa316b471eae112b23f70fb7a706998d14bf8124c6a99849121b76f8e3fd2b721c422ecf1618eeb09599565f5fa16cf4706ceab885f71000ca9b7d82a17613fb1dc570f1354cdf70da997dd5a2987cf749680840fb8b27dd506c73bd9f2955c3a65c82b8bbd32b055a91551f16d78ec05b61a30348b2eb0af792fcb783e590c087626f9a1fcdbb97956579e2e1958d1504a992aaa69ce90300e1139dcef2fd46ae0cc8ae6d05c9cab9a0326326241134811afe80be613e78b9e1ea282e195b3d3e84746a51698a190523fb2d1f6c74f27432a8a25ea776bfd9cf07391ae7fb6635af218f33888e42301e9dda6da1f777f8e0b826f58834990700c3d8a191183bc273957aa8e3c81b4b85a7cc60bd135abcc1f0fa902ff6232e21ae3fd43b13819043549024e1aaff4808d6fcf01b70545624b9f8ea2e106658c301be7292acc82fd8fdb4b3c76ba9f5ec3233792051de118870b180a3424c841200a8c8d99ad796752704e3ef418540d1532997fb2633fd29f62d9c2685f3477a9e9168256d4971affefb4fa22575419717d7dbde1e2692c6c156d529b8b4e3e5adf1442594cc8da5a9b530a55991fbc7dc2dad3ac599af56341a19da7b6aab210c00").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("b02701884fb4065e5dec5456f29cbbf7b093b5847c56b7f6c1fb103851b674f9122395c01b2ac3015bbffddd0ccce114a8c239c56aa3543ba593e69f94a411230b6138bbfade4ac527e990466b1b625617f415f58d572e2b0f559e590180ee17005001160b651af92d477bc900a6f468abe5a03d8d16667e104721d84053149b8c8e6dbaaa04f767fe3480adf9ec4e2501948c01cd4d17416f97407c9b1b69bd004dbeefb3ab8a56893eb0efd44d13f740d479eb3b43d4b11b0e23f9bed985ac0a0033316f8dbcea7ba33a2e6e3225c09f3db359b808dcd316f27ac309886060cda95c63b1f274d2f15731dd2e54027173182b5f79b1b1875c11669b2a89584308f461ce1becda321c0ede1c8e060e3dea7255d464c93ce846d65d200327888a320043ba1a5d14a41af8c158ed640c8d3ea06a21525671261fd03f8050c6e25c643a6dfb27418d1b36c14c3ce4a035b22a07a70b43b2f39e4cc54ff9bcc27f36508f0a408446d47a5e520c14a809605865a074631777ba098eb61145839216fa571000c0bd67354bcfaff0ac9be6d6e60dd27ba907b73e48cd29c9d04bfb1648047d00e6e8357101d30b79946c6072c6967909b9aae7f069033cafaea578a6b2e0e6b2bfabd528e90c2d3424af26a7d26bf95dd06296c89ddd8a662c52756656304118dce1cb5ca358fa9726344e8c37eede52e11786758be88dab87d896216dc0291c8f250322ca0aff90cea90f5ac30a250a65e187464f11b76f15fe8fd5ae1a71fd02131af2f1585807ba1729693d7481ec47d7731eefba89272466472f6482d109004188bba4fc60efa79ea39994af0bf56accd370b06fdce321aa7c0d00d4bf8cfac3ef3408822145f58963bcddd84f1711752f24db6810bcfc10b9f2d1ee7601703e2da6f8c42ce2e771e85dc81f0f71d3ec1537848e1d29220136e4193ae98a17005732f4779ecd296857e4217453314ebc5b733d289cae7d2b4109ac8df4d7cf4b368c6942006c79503155fde7a4dfbd1840f3f8f8599dcfded2050bfc1c1f41d9a0931b52bd5ea22053e7913104eba04a68e4aa9991c74949ca80871c14744f0c247b6df3c6bd961430f1aa53b855967a91432ad5645876e6b67ccf29f0cb6b2197bdc3fae24a8f5c5215aab931e62b193e64b49c48f4e916a73a2752542b78c53b7b96ab8a819fd45c37c6bc5ef76fe5b7a1d8f74df6a776b413bb7bd3852803000500000000000000f85d7a9418cef172bfb155561966fb1978f4a27569f2187964d93828824ea453ef131a512eaa4292eec5fb9069183110ccb6d2300144d5a89c3d85b7eb481272442678354671e88e46df7b0b1b1aeab06187c12abe7bda331952fafddee51a1300ca993bfef2e7a6680b3d9f35e073b3a9951f6d191f44436db4edd1f413fbb54eba5713d395987ccc937a0b646b21c802236b88d3658fa4dbf7cfdde6abd0c56f936753deb02ca4ca4ea6f57ba53e3228c7f9b14c044934fca0b2a513fe3ea510004b710575e4e79f71970e7a88f4935a34cea42bfd0a415b78557c1664e0e53e14eb156a7dbac5c83a88264c2501f53315a462194bfaba4125e2870f4f2dd6a4f1fecb22173ec1b82db95cd6c849bcf3e6d490523c1ce9ec15ca9364d01647430600ce6ee3547b59cb629b43a2afeadce961aaf1003cba71172c96d8eb8d96d4548779a87b06fcc3e4d78c079da529e944186335d815301862bbd3eaf22e1e1e3bfa387d5e40f1e7336facd285be3cf9e02801f061748477c7071e34a759c3ef661800fd51c582d128a1f4d508e37d02caa492f5726bceed6ede47de19e4134dec5515da51b7255779ad2a27ef58883043e80fb0e22eaac9598d142438a82678a3bc918495c86e5d64575c6a732ac2bf14295a0f42faa55fdd25f23510c8e3f7125f0c00").unwrap()).unwrap();
}

fn get_mpn_contract(
    log4_token_tree_size: u8,
    log4_deposit_batch_size: u8,
    log4_withdraw_batch_size: u8,
) -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::List {
        log4_size: MPN_LOG4_TREE_SIZE,
        item_type: Box::new(zk::ZkStateModel::Struct {
            field_types: vec![
                zk::ZkStateModel::Scalar, // Nonce
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
    let mut mpn_tx_delta = get_mpn_contract(1, 1, 1);
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
    let mpn_tx_delta = get_mpn_contract(
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
            "ed02b39ed13c94e8899588363cf8be09fe6e0ecb9967631a2464d5bcca7dcd7af8"
                .parse()
                .unwrap(),
        ),
        data: TransactionData::UpdateStaker {
            vrf_pub_key: "vrfec707f8ab27ab5fe217eae8932c840393e2cfab3dfaf79d53a88e1ac4ae4c255"
                .parse()
                .unwrap(),
        },
        nonce: 1,
        fee: Money::ziesha(0),
        sig: Signature::Unsigned,
    };
    let delegate_to_staker = Transaction {
        memo: "Very first delegation!".into(),
        src: None,
        data: TransactionData::Delegate {
            to: "ed02b39ed13c94e8899588363cf8be09fe6e0ecb9967631a2464d5bcca7dcd7af8"
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
                timestamp: 0,
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
            deposit_vk: MPN_DEPOSIT_VK.clone(),
            withdraw_vk: MPN_WITHDRAW_VK.clone(),
            update_vk: MPN_UPDATE_VK.clone(),
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
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    use crate::core::RegularSendEntry;
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let mut conf = get_blockchain_config();
    conf.limited_miners = None;
    conf.mpn_config.mpn_num_update_batches = 0;
    conf.mpn_config.mpn_num_deposit_batches = 0;
    conf.mpn_config.mpn_num_withdraw_batches = 0;
    conf.mpn_config.mpn_contract_id = mpn_contract_id;
    conf.testnet_height_limit = None;
    conf.chain_start_timestamp = 0;
    conf.check_validator = false;

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
        nonce: 4,
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
