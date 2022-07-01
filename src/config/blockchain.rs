use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::Wallet;

fn get_mpn_contract() -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::List {
        item_type: Box::new(zk::ZkStateModel::Scalar),
        log4_size: 5,
    };
    let mpn_initial_state = zk::ZkDataPairs(
        [(zk::ZkDataLocator(vec![100]), zk::ZkScalar::from(200))]
            .into_iter()
            .collect(),
    );
    let mpn_contract = zk::ZkContract {
        state_model: mpn_state_model.clone(),
        initial_state: zk::compress_state::<ZkHasher>(
            mpn_state_model.clone(),
            mpn_initial_state.clone(),
        )
        .unwrap(),
        log4_deposit_withdraw_capacity: 1,
        deposit_withdraw_function: zk::ZkVerifierKey::Plonk(0),
        functions: vec![zk::ZkVerifierKey::Plonk(0)],
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
        state_delta: Some(mpn_initial_state.as_delta()),
    }
}

#[cfg(test)]
fn get_test_mpn_contract() -> TransactionAndDelta {
    let mut mpn_tx_delta = get_mpn_contract();
    match &mut mpn_tx_delta.tx.data {
        TransactionData::CreateContract { contract } => {
            contract.deposit_withdraw_function = zk::ZkVerifierKey::Dummy;
            contract.functions = vec![zk::ZkVerifierKey::Dummy];
        }
        _ => panic!(),
    }
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

        state_manager_config: zk::StateManagerConfig {},
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
