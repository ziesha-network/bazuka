use crate::blockchain::{BlockAndPatch, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData,
};
use crate::zk;

#[cfg(test)]
use crate::wallet::Wallet;

pub fn get_mpn_contract() -> TransactionAndDelta {
    let mpn_state_model = zk::ZkStateModel::new(1, 10);
    let mpn_initial_state = zk::ZkState::new(
        1,
        mpn_state_model,
        [(100, zk::ZkScalar::from(200))].into_iter().collect(),
    );
    let mpn_contract = zk::ZkContract {
        state_model: mpn_state_model,
        initial_state: mpn_initial_state.compress(),
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
pub fn get_test_mpn_contract() -> TransactionAndDelta {
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

pub fn get_genesis_block() -> BlockAndPatch {
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
                    dst: "0x215d9af3a1bfa2a87929b6e8265e95c61c36f91493f3dbd702215255f68742552"
                        .parse()
                        .unwrap(),
                    amount: 123,
                },
                nonce: 1,
                fee: 0,
                sig: Signature::Unsigned,
            },
            mpn_tx_delta.tx,
        ],
    };

    BlockAndPatch {
        block: blk,
        patch: ZkBlockchainPatch {
            patches: [(
                mpn_contract_id,
                zk::ZkStatePatch::Delta(mpn_tx_delta.state_delta.unwrap()),
            )]
            .into_iter()
            .collect(),
        },
    }
}

#[cfg(test)]
pub fn get_test_genesis_block() -> BlockAndPatch {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);

    let mut genesis = get_genesis_block();
    genesis.block.header.proof_of_work.target = 0x007fffff;
    genesis.block.body[1] = get_test_mpn_contract().tx;
    let abc = Wallet::new(Vec::from("ABC"));
    genesis.block.body.push(Transaction {
        src: Address::Treasury,
        data: TransactionData::RegularSend {
            dst: abc.get_address(),
            amount: 10000,
        },
        nonce: 3,
        fee: 0,
        sig: Signature::Unsigned,
    });
    genesis.patch = ZkBlockchainPatch {
        patches: [(
            mpn_contract_id,
            zk::ZkStatePatch::Delta(mpn_tx_delta.state_delta.unwrap()),
        )]
        .into_iter()
        .collect(),
    };
    genesis
}
