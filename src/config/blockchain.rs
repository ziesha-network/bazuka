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
use crate::wallet::TxBuilder;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 15;
const MPN_LOG4_PAYMENT_CAPACITY: u8 = 3;

const TESTNET_HEIGHT_LIMIT: u64 = 10000;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("a92c2e141c17cf90a724453340d973587b5ff1f24921ba5f46a6fee52fc214980ed4aff48049a995ffddd90902eb1c09520cdf863f02ee7c8c40632a142808086e1ab6ece528a40f2eda453410591f4ae3d1b8ff73f68826778934294a7cae1500d77104cc9518397c49b01f791e324ea9df5ee7d8c8096f0e76306d76e51fcf826633c08081a4a76a03e88eb1dda26513235233014d53b8780074c24c3ecd1ad19ecfd92df6b8dd0be9fc397001706e566586a0e6c64d9dcd201de54651d7641300295163e2773f0cdef854f6ed6ae0119fba6bed9777584c94e60cd89ed401f5af676afd1839af7bc7d0fb9e01cc70370d036b1b3bb181fd78cfe930be478304fc04e4ade2e63187ee953c3ac85bb0affeece967d7c1d70cb524937583f4a7ac00e04e6784719192b289ebed8a164e1f297675ee9f1e9a185eeb808b849be619746b9e06a1eb7fa94cd2ea54d68f44a404e5b7a15a4d5e9dcdb45793204b44b310f1d2ea06f0006e9b9d50b95063f91ecec45923d01052a416f6e25ac2f81ca7190045f300fa7fa2f023d783e98510c36d285fc9bb6ad3263503780b5a05cdd19c33c8a9884ff43f9a2817c023283cac591931a7837a7ff55e95a9cd3742bb9713ab853c66fd284481145a9fe53b1bf83a4ce7bede9dfb31e9405fc9e43f26458618843a9cd2a3c8a95552037119a1928b799b8d1142c1ceab4754d62c7bd0e2fe3b63408a783168203b2f7b3d66fcdb440bf9788f50aec336fa979ecdaf08f24217fbddc9d71fb4e971ca12735d9b620f33380f0ebc2cb16b9bdf85979d9cc4f908001c8dd0b3778f08edb7e798caf938af3f705ab3628bb6e644769723dd983fe26e3505670c2e1debcfd1f2f0d837246715f183bc7fed16c78fd5308fa95159b009784bf0b5b02cb78b74db65c1aa0499d2382997387f832b1d61b4e3e81df38804003aebd0b1209b5d4f001edef9842cf407d93f901c08efa19a5337373a918c960a9040870c90e1253419888ad61d8d8514110ca8112a95a6116576552ed12e53de43b09f8be496e428f68bd7307d2a8559bb0bb50d19e8e928a07150c0d27ffb0d65db6e546a9fed013b8179b87ffa3f2ef7b05480c1be5a0838b25aa1f7d3fdfcd26ca2f7984600a3af34710bb4a31e05994b7df506b3abc1ea988dcda632c323eb7ff54fd5e3489f27c096dfec293517d8ac61a1c81b87fa721145d5a5004c0300050000000000000010b119ffe6bf522fadbb0abd6cff2137da52f61355569cd9d6e226703b09e0fd20b14584be1a680a37c971e3d7e497098affcfe0796f3523773b5dea537ca2613af6232ed3ffc8af306b817b33c377d833c2750ef671becbc39ea25527450d0c00a603fc63600467af6ab6b0fc85d5447226472d53cda32c7591c31b68cbb0916594935f50ea14358472eb802e12a8700344a59a3829354c2d4246cc5e7922da495bc294460e2ed9224e2161440cf06b3bc5878a7b3b50c104cd4d1f626c86bc0c00de0e97ec352fdafa7b53c2df8bcf02db06e80fc13797871821c6a5b42e0a73e9b435001db1255f5d98dfdaabe68076190ffb2e1b92a222bb3c82af666a48d1a720d78c5540e52c1d08d5eed42efc8e0f6640db162398d7236918aa59f46ae51200c5ff45032bc7ad0920cf2791c9221ae4c72c800f2eff9deb7b8450313559eb2303bbc55c2f58ecb90c21bd71d10d530e251c194fed6f1877d72af4295ddf8cd943923ebb90b1a821ad998a2c773347e88077f2f638f2d0e5fdd56b132f1b7a040014b199495a2b095c85c718b12ab48534e610d6b5334f9c20b3d99c5b96cee27ac8453e1e803987fb0da4fee65b112300593d16307f0028494135b16460b25d3f58e016e458099f3cf07141934e9153bbf4f91aeb22041088f933444b0118541600").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("ab39963e55476d3707494e218157201c6840393b45b2593998d777ee35bf91ba79080da222b2a8b7c29a881904dcee05438846cc3aa7d8653a1c9536da8b5dbc068b0f0d850dcd974aaaeaa9b6dd24dded76dced45e61d86966ee48b0c80f40700895e3b719172e22a825542966249b6e7c476df5101fda4bd6bd207e1c23131f0c1aa5a17d2ffd8b28e99a6dc37d15419611acf483455103803c7796ae92708e8ae0e40ec3bac8cbe2efda03c17ea9d8795cbc00c0604db9338e205df5ca79218005264cd6555967132448284a33db96acafdd966ba049015476995b021628492a949c80544276c2265a361ba18d17bf1145b1795c4bfc8374e4c536b8aa119beaa384419539a2fbb46f51c0ccb9cb3992a20e5f1fbbf1d7069d7191efefabe8616e7655c481b910af7c7643cced7d93af87be41f57a186f32008854acff790792f792a4a5f5ea4aeab8390c6cfb668e106e80a852ad9ae9e3d5603eeae80bfa273e928a711cee4b1e332f610d1d1a2d5f80de1464a3c374fdaae6932919a990c1200b87447a1c76e95292bd88aef68f7987c1947147ae38db4b511d6a2f31186c56e8e8801dcfa6dd2701e2ff911f62fda0652efe0f629a173ecb98462887048dbe9fcc830074ac707282cdc339015c301efaac8742d22874def18400f2ba13eff0d8382ef0695141e004bdfbf47e111b3a59a4d682dfafa443e399c08f7bdd06ef181458d3698757135b50dbbde0abdd40fde711b03ce96168db5c79098dc23243f184800810bf318e678a5658ac723ca455eefbd57e250fcdab64100f13abc2d1000156203489f5337ecefd45aee6d9ede1beb230c11f0684fed6ae1a9b02c6fedc546b2dcb7d5c81f83bff9d4ed16d9dc04319920005b8b28a7acff4cf1e5101025f974db5ed2bba4bbcf7dcd1706d61bbb042af6d735a22042f8a89a038be5fa1700a8b68e8cbfb9ba08b7610848744b4c0a7ada1bf4a8e6242d079f50ea4588fadb599b22bfd2ea6f4932ce3bd5411ff2168d50ae9c7b97a2b5f518be6adc67871fe8bb590330245640ba2e6985d8e47eba5482841b64326ba805301b82263cfb17b77c1c8cdd783e3e097e2b5388259c70d9a5bfb1c98e13b45a5bb183b3ac13d4fa5cde4e85a5a0f7ed3b9f1b77dfaa045db1428cd27ce44f765e4ab9b380a9f3e17bab96c8e513595adc720e5c1cf71add79d6a59a2d4bba22c818cf06a407090005000000000000009cb781219ec04ed548087bf5d10192107a37beecbb4046d65f7c56899a112878c4e3b055d420403e8183e1564ed2b0070e375d6715d449875cf04c96e30353f263e26435bdb89ce1221fd7c92c6e352f8991ce349b72387a8330a0a618dba20900dd8d29c2763ed21c98d1b49f7ead8f4cdea4540309ea1ac3805b87df0eb84a877050c8ffb86d1a5e1707240e9adb2601b8522723acf28f95f2c9f2f091897eb82e9ab487a6e804af79a7b6624fe4eda858f3135452b7a9c89fd84668d4bf2e07002f3bc3616379296f940cb24d34356c8a4c9bbf2dd3ae2633cbd33f049a992fb031ee6123df9e053bb880d8a5f52cd515b859cdd3f489e9b25bea783ac43be5f3b638ccd83d4463fcc815af227b6fb5f8de9a425d28e923799b3061ef7e1aa70c00052c7f2ee54a18f021bfe3b996cefd5ae1541d01d739dec12e5e9e7f8aad2780c2f84a61ad489b97c2acff5b6f7a02134362ebe852d42dd93d5c72f6214dab7ed13a91af6e5eb9df711ad7ca15c0c8274ec3020acc3897761bc75a1448e6880600fb5d8b5c164fe06eb2e43ff6a7f47fc9bbffbfb79d775ba9876244391b92f024e03008aaae7b316cf77338f6c387f604e905c450a6e7dd5500b802d4bc9b34f9ea42a05f69ec4faab07169078567206ac83db6815c9ff50c90f481da88f28c0a00").unwrap()).unwrap();
    pub static ref MPN_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("932f109697dfb49abd6b2429b77c1f93155db13d020eb5472fe006b27183ff0623addc81070fccf29aa0791d0ac2ff12831ca0c40d6d61e5d729271153767eeeb119e598c82109b4c5b6b24ac5264d0e497f3e1d3f9c4190e08601a1e3ae890400475fd93aa7222116472cbb2e042d83a74eef8b1cd933390051492acf2756e4319881950acfcc52fc5bb7c0d493526005b43f1cbe4d27b835c83d354960a6ece69a4699fc4358d22ccd63a64387a98ce44652fafbbb27337bd52797051ca2500900baf3b0782828bfb1f0930bfdfb584caddea55c36f586241ce8cd5c69e96701933881c56a0741afa9143de5ba8f7d051543c0df733cdd39d0ee15a1e40270e00ac6fe83ba1509e39d374850857547dd35d9cf42ee6fc695a501353eb6918ed50549c5f1a06336f4dd137f0f54bc4dea338a8166224e2a0fb5ea8977feecef2dcae36bf8f079de175f8e4c8972b0fc0214d473cc05dc9ac8176ca5c3847f1ef19f9e9ef724ab22f812ca96b1503a92e981b69ed03ed379f1ab1822c5c8a8caab0900dd4783d023323cecce8c07511019a03896ed2d78c34b6e808262c9e5738c23fd9aa54b33196827fee89ecc522ace5a0fd0bfa084dc15c0fb7a7ba457e76afe0654dedf4b7e79bee10bfe75a1e528b102b78fc7ebd181798399d3eef1b0cf920ee814ef808cdbd0233c27a29107b13008f96f46c274135273257c073dc3ec22667ee3c570b01a54e453f7903061dcb20e2772ae59de3add173ea250f83a468947dc91c957e6a8dfffbcfca9f5e43c5beb6d7d368ec0651dfcf703a9738336cf0200bef08ce3b60f58880ccba994fb360ef07cd929805e0febcb6585168dccbd776ddd4ec9278121371e65784b6aefc17d11c32b483b472ef29b63e068784d102fc935a9472a2ab4602592b8802d0f491bb0800cdd197c2693054568856d6ae23e0b00e941bec68bba63c340fdb1fb71fcf1360ecacedfd917c3ba6c96266a7e227c349010caf61f22cfcdd8658e96d0dd7118e756ced18480b9b0a5307a0f465dc31368da3163b0efb6a110f653ff93e00c25e5279e4716b90a7965dfa11e1489d605abdd70c5e1120d3eb33636ca08950e51accc7f9c29fba99e749df8189df98ca8e663f1ee9bf3bb1d437c77a906c24513570348caca18161f3ab02e8aee1944be3c83cfee33c229d86ff4d224bfb129116e0aa243eb10a0b9cfca690e66f5ac1800050000000000000086aecc78804da22105121d090b8c0edf21db00cb7f9b08eca337fe33ab07cc1d152705e814b3e012c94c3b747945e90092315043b31fa6907f7a43a825c168a83a6d72d29e42ca689a84c21ef3b55f8fa18034367972fb8ea26bc435287c020b00406b9a0d703976064ea9cb6e02360912b610668646116326cf32580bb7552f382d85f3444b1834b0955c6c85a9ff9a102e52ea18ffb75248460f8094d8dbe61f425a8d426ce48ab61ad51c8f4a2253ba27d535dd52e30e5bfac1b7ec23e9a10200304e074e2fbe77dcdf31b5e0469c47874000353c29807098f1dfdde8ca51ce223eb073de69ec34a2174d1a6038fc3108f1577cb8285a089a547f976bd4a065dd2aba039e24d7d8f41a60d1d5bdd67b0afe33cd3a49632e031830bee67093250b00dffe84004fece5c0d75e8df2baad97d1497ec196348cd6c91e9c90440c913edd24487a686ad0308f9814fcaba771d103bf4fd1a1cd3ea8a65ec56cf12fcf2aaec8672a5b15e4408a800d887feec4fef1e448850dfcd543f96ba618d6843e1a1600ed2798b25bb7c229d7d089f142c42682cabde9f4654398f0e37ab1f3dec018c3c96ebb3c5e890ae2915cf1756042050cd830a3e4a8edd45d9dad1ce1ad7a32dc65097583ca3582cae44d725df4f8cce0c7e71b10a95639f4f1901566a0243e0700").unwrap()).unwrap();
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

pub fn get_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);
    let min_diff = Difficulty(0x032fffff);

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
        limited_miners: None,
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

    conf.genesis.block.body[0] = get_test_mpn_contract().tx;
    let abc = TxBuilder::new(&Vec::from("ABC"));
    conf.genesis.block.body.push(Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            entries: vec![RegularSendEntry {
                dst: abc.get_address(),
                amount: Money(10000),
            }],
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
