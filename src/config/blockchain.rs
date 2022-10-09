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
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 15;
const MPN_LOG4_PAYMENT_CAPACITY: u8 = 3;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("3f199befdafa0452eb41dbafe5e0a36381fe1ba7d698dc16e620591540c991969eb32ee78cd011b62eb96b503ba4b317a4c8552ff5e308fad0c0b0ef17aaaf65f53fc4b9dbdb89ba6f0c8199dd92dff453e33eede0c0845117ca822f6e6a8811007bc4cbb2b740f7ebc412a231b7183fadcb003ce07f10e8d5133c6d2588a835305ca2330692c028056f543d366575f90c066b813b3a2266b13cb19ad010c5909265ff799aec091f409cf04effcc5743645effb04d0019796e0755fcb7865b86060037e0d10bd2768642a9363085d4b323b4e0cc51e3d8da429bcd985e46f0fb32395674724e91f418e29c9020de45f6780641ad530d672dcb6f7eba18f93347221a89dba2b6914539eabaa57830668668ade353de22645e83300e82ca3c5b0f0719ff4e34ce1062331a9578aa4e392bd57baec7b6b264d37f59b6063fce47b38da04e23bc4a96b769c4c9cd58290f86a30b7ce7e0f8c0d5da07841f4369b6a4f5f376c34629dcfc05760493918d99bd99b211cb0bfdcecaa4f3ccf243d29a0c03120043a0bdc61aad71aeed05b55b5ba8cd6ba27e42d6d02f3886f8ba505977033909aedc65aa437e5ddc9827d6f81f867f1924de0b5d7557aa566a088af9f402bb760bac45c375bf77cbffd4d9ccb4ff12b2ead07f54428c79bced24573261912f15a5f16118dc0ce771342afdabe7b37cd137a0ecefe0e22a1daa171fa3e3e2c72eb746ac0b1359faa42a0151ab6940570f5c87743f4b8ab7efe9f2149d8cc94756fe35912b14ed162e571c54ba40c36ce730ee588321458a7f40f23888a9e02d1900391fe965a04a337425cba7c7237f450b94ef74f146025ff7fd9ce4ba8cbb75283e54d5a3c95598a32c234ad05b1ec703706f129ea17a5ac43aab60f009119b9c1e1de864312238df3a4ba94c502c3262582d6a2251b27c147275d31acf056218000749440a67da1be23a2c257a6b1ae68cd22981ea90f275f5f16822029053d21bb6423df3ed1767b1f42eb16c87dbb3116295970ca234fc46ea9e8e85a80c8d40d3212e4fb9929561e7f3fd229200d453bbc0165791853786fb8b1df54ca7bd10bd9d9f636ddcb8222b9509f735092a6610e9cdc90c6a71974a8a8452285a86ce29e7096243a88bdd0e8bf3bd7632d213816301f20f04bbf13060d3b4029dc218e146c55b641c5c3f8d9f145985f5fd83023c92f2b86f816deb22bce75668660e0004000000000000007a38f72f670e288976916f2e184ce1eaae4a37e9403c6ac1f2f380afe8cacdae847a68c2efa31033bb259351333b7c1180b67f5944285563825e4e8efa5609d7241a3e30371db73e42822d551ade46401b41ef3219fe07caf9e75fdb8ea00f18000bba41dcca0685de6cc4bbd05d1e3b2b7f7cac5139273bf3ff06f84af51505acc363230f63d8e283f17bbc2ac4c7cc15de6fc610be3f23b50ef441f7cfee68472586c315f84e0dd5a42d5c1ca434e419acd304e53f16fbafca3dbfa48b9253070093e2b6c99569701dac13767d047068d357afabd51313913963376d8f8699a85fd8f0efbd33ba59217aed553bce86880069679224036986cb435f6e5956b72058ad930e76eadfe6f94a39ddda623e8bfbe20e8e5fe8b81c9f381acb8226e28306000a6a7319bac9abda9b0be3257fe4dbc5c4b4e51de643f0a268dc95b7beb8fa32f52405f4c4b81296553ceeb4aee2a3163d4336b0bdcee33d13b72604078943e746e9299872cea6287e799fc21cf576619b2e953fc352937436ad23673f5b140200").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("5b13098f03bd2176afed05ef5b5e8e4af9c45dde7fcd8b5cfa35b82216e6f3c53cb9ea16cee040b4fe897a6cc85965037a59ef5080e34918d89d1fa48b017c46621965ec48d43c9a6a458344ab2d57cef3cee164d0ad963cfaeca878814e7e030067262dce13114a1a5746b33398b45cb7ce6a058496c54e3e35b0e5c499f5282e42cddb5e2a3e10042ba3109e556d3a152aed6a63251792884fb25fae55af429c558fe5c9d859e4beedf246d774bf71dbf545a6fe040015171a0cd06fbeea031300d23f696e716e31ab49a737323043e1d346f273020cc03b35dc29802793ef9fe3ead5cef00d2554786d19039954aa2603bae47eae410304d4881617bbe899fda942d0bdea0804ba112aecf41ae5c215fbcb2562e900db5c4e9bf1443ae91bf11263100c1a71e1a21f64c68cffd8367fb419ea402afd6beb391161318a1508e9b7c83d0444c9ee946f7e001f1f809b9a09f42ea8035ca3de8e6463ecf28379042354fe0ac72f1ad8e808a376b2d87a22456fb9176e7aa4a9a3e6568930d9ff621100379f0bb2838c775b508642e27a419c97dc84e62e5fced4c203fe624c3c8c802dc37da9f8cf9d15b860a213350e296b054a2bf3fa14cc3b82f0bb5ef376864292fdc2a96e81d24c71e52c1abbdb6708dadc4ea55059284293abd6a194b07a8613eb969386846265c37861b77cb09b43651b1be78410dfc9d42300dcc488c38f6985ff4bca42bc8701d94e5a8a17463e0a42fef67dec88c6e4a95f381b11643d975ea9588bc95703d07795b60dff923cd8a50217360b75147394cfd5cfab982d0c00091b61c195b1ea129ab2aa06893d2ebcf4e6f6eed1940bf59c88b8e892152a96ac852ca3ffc66a24977c2d429ad9501419d2b2c803137b7bedf09c9b6e86d1ce3b3e4c04c66f2d192913a30532e4de77e6c1c12e858bbb69ce3c928f7e166900002ac279436e4b5c593d2a142db175f64dd5b70b4fd2a10c3931b8c2b1be5f70506d6b32fb71aab99e06aab537df893403a6c2393a03513a6a077b1c42f431c27271cdabf25e2b9d8749e115b9f55a7a8fb5341b9d98f0a549cde6e47480c94c10772da0dd837611f5859785239d45bda10b1a6d07af8525d7dcb2dd7347e4c840d8aa1087a7ac6c0e6629079f308138019734578f716328296ac013aa273481971c86bd5203127459cacd5bdd32ea62a463722daa2305ea4b12e70a0585c70719000400000000000000dfb7739170e202f885129928f96224e304beb0a021f343c0e8c4287e21b1adeea3007dadd7bb8d47f0b376dbe299191769e156bb006f9fc074c20ed3568562c8d6a67189badd8d792f98d98faf10be478742f04f77d021da086661ea104bbe0b0010d7d1de5a487ba9d5781a7c5426e0a4f038d91ebd9696b7150491acad38d7b5789bedde564dc07ee2b57a89cecf7503e97e54a525eca5e0ae2b0032fe159154445265f96edc885b5d6c13115b468a4e954b48cb1b3a48beeee464e1d05dba12002e471db0eee21d0c8df4137a60cd116fbf6e712a005e633b3c0511a005be19ce2dc7e09b7f309941898bbd54554f2e028a012474c1f1ac22b24ad561f3d39cd724cb12a4f7af03218cbcc69385a30977ac564e873204cbeb62e9bf8160e5430c0027b4ba4c614a70b8ad429bcd0f944b6df4998f4b2d7df6b5fa57d205f845e726e44fb5d759d909d9f45404dacd1e0d03d67ef6d66f54bf78caf3c28372eee5ccb756bee34c0a343d49762d3f24f681ff52ea405d67a16a9ae541d0923576270900").unwrap()).unwrap();
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
        payment_functions: vec![zk::ZkPaymentVerifierKey {
            verifier_key: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
            log4_payment_capacity: MPN_LOG4_PAYMENT_CAPACITY,
        }],
        functions: vec![zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone()))],
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
            contract.payment_functions = vec![zk::ZkPaymentVerifierKey {
                verifier_key: zk::ZkVerifierKey::Dummy,
                log4_payment_capacity: 1,
            }];
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
        mpn_num_contract_payments: 1,

        minimum_pow_difficulty: min_diff,
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let min_diff = Difficulty(0x007fffff);

    let mut conf = get_blockchain_config();
    conf.mpn_num_contract_payments = 0;
    conf.mpn_num_function_calls = 0;
    conf.mpn_contract_id = mpn_contract_id;
    conf.minimum_pow_difficulty = min_diff;
    conf.genesis.block.header.proof_of_work.target = min_diff;

    conf.genesis.block.body[0] = get_test_mpn_contract().tx;
    let abc = Wallet::new(Vec::from("ABC"));
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
