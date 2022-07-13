use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 10;

lazy_static! {
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("2092b84767f1da8f6323fa4c95f012aaa5a0858ea29124f028603472d5b84855c35e8a55e2322f03577241c256889b0be0223aa8e8de74f4476f7ba5c6a84b235e49ce0453290d9f4cf24e9ca95242561799a1b06a250c6cbd9f0e77d023c01700087915458b809f837b62dce68c779d4ae995bbc104dee78f4665f5630450e20e3fe6df820a3f3991afe4ba1a303ee0197fb8d7e5a0843ac06f44e262f2b239fd0ece8c5d5ac87ff4fb9bb590ce95dee34b69f90da1b4444059b0a3d7c4e93506001fa664d579bd99cba99ca4b16f025890cb4a01cee23f95e0758538571f73d47d31ba884357aeaf390f4549b28db8d704737000cd04df0dcfef2fd88e8de3660bfc8d2b0f1eb2e8edece3f7bd639e1be60bbe533f2a943ceaad3ebed3763e2f072fb200a9823853df041044c4742f2bf446f0eae5b80e6da6da828592bf32f189ff20561e61f24cfaacb32aec5afed302ff0e1a55f2ac7657c22487fbec3a2a00252a851fc1fa490dddc6652be1b8ebd77cf91ee5ff47600da4035b40acefbf150003f11056f323e7ec64010301c67cdcfc068b7844fd2694b9700f60fb6f10f79324c225385810b66ee4510d1398de210e91b36b344cf3b69e4d82e7afa80f1559874c6333d93b7b432bf36ecb777103913a0bb6921e459521025b96ac5b1dcf149ccc74cc7647c8ec653142f69aac4d75e9fe38116f2f32304b71badcfc1cdc66ea01783cec2208fc5424a162d38d190b36286c853404c02ecc34fa66938ce8c33535c41c8f87b31e6a08681af31e686438590e51a4fe6b63f06937e9c617370900d7e3fc45479aafa549c61dfcdf9aeebb1f79ba9723d2ecc8848ed13b40490b8b9c28a56af1d79acff3b8fda80d2f7e06cc199d9c2e557c572b66a074ce366642989341cc04cb427ee8dff34523ddb93a886050b8f2264a846ef1d8c41b4a150800d6b9a0bcabcc7172ef3323ab5f5fb9b253d8bd66782dc8c5ce149d158e31943ef73deb985e93f186a41d03b045ee4a142bcc91091be1a0d93e2b4d598d21a8378098d2c1db6385d5f36b851d968514457853e81017b83aaa3caa87cb2c861d06d45f777cb46fea2296dcf5f8b8ea0d0d13467d12493421a2aecbfe59a1beda9a7772e2dde8e4266b6c2b9b9660806e18671ac321cc3255eb5361cfb54af35d950986bcdcfb2f9a0510b597ffe760046aeb164ad54c3e15accd18da400991fe0a000300000000000000d00768f7539bc25b7d265e7fe478c4018f6290d2e0ae0cd868289ada221c5660f918cdf973be9b112f64208ea49aa30ae7af9120703e7b288da94fc0d52fd30660be59969317948585e79e2b83dcfe8db02949d723a440172c07d19ac345750d005b649f58fb1d1b4e332ae4397a8c5fa402ee9eee8edcebe6b1090379cf915bba84d98c0ffbb31544c372b7ddeb400d1174e1a11a95130bbec87df5bdf468eaceb4012d21018c88143c623ea3512cc8e667740232b9fd9a9824bdb96b80fb0b0900adb9ecee436ceae1915b2fb50187a64877a2a60e20da50de985a07cd7cf3c20b7aa14a6ab41b7d9325ce86c4bb76a412fc4f9a042e4c2281368957b2b38359ae7cfc761cabbfd58a41acea94ff4ab3f82234673d0f7acd95a59bb762a287c10900").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("2092b84767f1da8f6323fa4c95f012aaa5a0858ea29124f028603472d5b84855c35e8a55e2322f03577241c256889b0be0223aa8e8de74f4476f7ba5c6a84b235e49ce0453290d9f4cf24e9ca95242561799a1b06a250c6cbd9f0e77d023c01700087915458b809f837b62dce68c779d4ae995bbc104dee78f4665f5630450e20e3fe6df820a3f3991afe4ba1a303ee0197fb8d7e5a0843ac06f44e262f2b239fd0ece8c5d5ac87ff4fb9bb590ce95dee34b69f90da1b4444059b0a3d7c4e93506001fa664d579bd99cba99ca4b16f025890cb4a01cee23f95e0758538571f73d47d31ba884357aeaf390f4549b28db8d704737000cd04df0dcfef2fd88e8de3660bfc8d2b0f1eb2e8edece3f7bd639e1be60bbe533f2a943ceaad3ebed3763e2f072fb200a9823853df041044c4742f2bf446f0eae5b80e6da6da828592bf32f189ff20561e61f24cfaacb32aec5afed302ff0e1a55f2ac7657c22487fbec3a2a00252a851fc1fa490dddc6652be1b8ebd77cf91ee5ff47600da4035b40acefbf150003f11056f323e7ec64010301c67cdcfc068b7844fd2694b9700f60fb6f10f79324c225385810b66ee4510d1398de210e91b36b344cf3b69e4d82e7afa80f1559874c6333d93b7b432bf36ecb777103913a0bb6921e459521025b96ac5b1dcf149ccc74cc7647c8ec653142f69aac4d75e9fe38116f2f32304b71badcfc1cdc66ea01783cec2208fc5424a162d38d190b36286c853404c02ecc34fa66938ce8c33535c41c8f87b31e6a08681af31e686438590e51a4fe6b63f06937e9c617370900d7e3fc45479aafa549c61dfcdf9aeebb1f79ba9723d2ecc8848ed13b40490b8b9c28a56af1d79acff3b8fda80d2f7e06cc199d9c2e557c572b66a074ce366642989341cc04cb427ee8dff34523ddb93a886050b8f2264a846ef1d8c41b4a150800d6b9a0bcabcc7172ef3323ab5f5fb9b253d8bd66782dc8c5ce149d158e31943ef73deb985e93f186a41d03b045ee4a142bcc91091be1a0d93e2b4d598d21a8378098d2c1db6385d5f36b851d968514457853e81017b83aaa3caa87cb2c861d06d45f777cb46fea2296dcf5f8b8ea0d0d13467d12493421a2aecbfe59a1beda9a7772e2dde8e4266b6c2b9b9660806e18671ac321cc3255eb5361cfb54af35d950986bcdcfb2f9a0510b597ffe760046aeb164ad54c3e15accd18da400991fe0a000300000000000000d00768f7539bc25b7d265e7fe478c4018f6290d2e0ae0cd868289ada221c5660f918cdf973be9b112f64208ea49aa30ae7af9120703e7b288da94fc0d52fd30660be59969317948585e79e2b83dcfe8db02949d723a440172c07d19ac345750d005b649f58fb1d1b4e332ae4397a8c5fa402ee9eee8edcebe6b1090379cf915bba84d98c0ffbb31544c372b7ddeb400d1174e1a11a95130bbec87df5bdf468eaceb4012d21018c88143c623ea3512cc8e667740232b9fd9a9824bdb96b80fb0b0900adb9ecee436ceae1915b2fb50187a64877a2a60e20da50de985a07cd7cf3c20b7aa14a6ab41b7d9325ce86c4bb76a412fc4f9a042e4c2281368957b2b38359ae7cfc761cabbfd58a41acea94ff4ab3f82234673d0f7acd95a59bb762a287c10900").unwrap()).unwrap();
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
        initial_state: zk::ZkCompressedState::empty::<ZkHasher>(mpn_state_model.clone()),
        log4_deposit_withdraw_capacity: 1,
        deposit_withdraw_function: zk::ZkVerifierKey::Groth16(Box::new(MPN_UPDATE_VK.clone())),
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
            contract.deposit_withdraw_function = zk::ZkVerifierKey::Dummy;
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
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);
    println!("CONT: {}", mpn_contract_id);

    let mut conf = get_blockchain_config();
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
