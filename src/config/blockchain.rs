use crate::blockchain::{BlockAndPatch, BlockchainConfig, ZkBlockchainPatch};
use crate::core::{
    Address, Block, ContractId, Header, ProofOfWork, Signature, Transaction, TransactionAndDelta,
    TransactionData, ZkHasher,
};
use crate::zk;
use std::str::FromStr;

#[cfg(test)]
use crate::wallet::Wallet;

const MPN_LOG4_ACCOUNT_CAPACITY: u8 = 10;

lazy_static! {
    pub static ref MPN_CONTRACT_ID :ContractId =ContractId::from_str(
        "3370db451654b730072f16c65187d408aa1930b6a68f27469216c848d906c8c5",
    ).unwrap();
    pub static ref MPN_UPDATE_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("27ce6cc2b5b6455776eeeff71e137bf2916359e076a2f2bf923ac54bfb5e34e3b580a63c96540107ca6d5e4dc14651158933e2fa214be62b6e5a6fb9be370c5d237ed113cc6233205be2d39c3b2aa0c8051498b0f2c2d5246a4911e22980c90600e2510c5ffc80c624f24ab198e46c9c94189a7683524d968bf3647ff438241a2051077f521d4adf15b27eff06a770f507820df99455c7e2be54d2a3816a0d5906a06f0b48e2e20264edebdae294cec0cd94a6d043991ffa54c6f60846edc2471600c91e73f87a991d05cb8f2fdf2eb9195dbff951f48e8b246be9d256e176c9934978d7f7972c951223dfcf580f6d90b80bfa4747cfe33a01a180b2656aea9af997530e5d0dc7f91be4d67254bb4143a27dfd79451d8aa0c1b5ee2af4c65e03aa0784f7978fcae8c9a0cdfc3e9927b319bd6b4ccaa490e95c562ef5888c26ac670337a711dc9742f72e8316d1944194cb157c2cf55c447ddb51285185b6885ece6059cd41523f2783381895aae8901a5f425b8e2f88301d2996bb4d9f46a2a3420700682db3d56d379c0cebc687160488c78b38536ee8dd9e07c467316688f02d6acd27d5b6847a97a63ae88f8319ad63321211e05d79351bae1f817fa59754815ffc43695be920118fe409e4615feaa7004ad53458c7cd0a05e786498c588501d10c3f83fd03f880d5ea2e1e08e3707920fce85cec7809fe5c340b0d5306836ad776ab81b8d8578daa539e99c6aa3eeeb510e6156bf255c500ac0c994b17a23f0dc11801c818ac71ac28e406aa31d03e9b0af52b89e51ce775ecb0dd44b7771f08160083db874b0d41622be9fa0eb415c1775b5d332e6bd1cb76c6c7999c242cff92fcca8a76af9b68fbaa95a094ba6d001a1047c98105e5057b7ec30e64560988ed1c3d6a9ac307546832fbdc87fd7fe86c8d91a3476b7468e751fcc3cbd35258450f00077c2e3675d0d8000f9ee601b9d7792e03914f7e5168509a6a400b8267596d73c7ce38e9daeed9113331603d54503911ac0b648f21202c4259c70272c00aea80b79bdf2b4d837530e8ab86c37a5acd3a68ce4e5a02eea9e143692b93a1761312176297ffabf08e7298cbd7790f6ea58b2b9fcf6cddc65262d85f05e724b30e2a75732fc3461b2b104f23f77aa68bc40879aae1cde841462fad65f016d37db26b88d256d3a177f8c86c7f3ef57416e3106846b608f1f1b37880826630511a6c02000400000000000000ad45bff680ee35d19eefb8b4cf9eb9d6ed5db5b4040f09bf7203cab379c5f6ee45ae86689f4f17ef7eb266cfb348ac0305786e59b83a6c309b949ee72cc103eed7f9f2cbbc449ae1c384098293951f812495f8f5c59f483fe63abb398fca831200fa86dc5a629f9cfe554360a2b13c402bf5e998034071a3f4d5968b3e91fd373699d38309a3a648f24b92e25555acbb11bf603de3820d08d142ebfc06a815f6a378e01b57812bcbe3bc644c0e71ca576d25155185e3bee7bfd7056d8c8a355c140045a96c123b81c0f721e32d4ea3f391f05ea50464146245526734b7f55b652504b31f5f64f91ab7aadcd8a6d144bbb11977fcfa77eda54e0b34ec0ea656b903e4024f70a407303d13ab56712ce89570c2564d6cfe5fd7e50c9d5aec4f1741000b0026dccc7f54be3c09992aad4711ddeb58048c68fc19a9cd2df560e3d89e681c77559bc744c900bee43c0e5998db34981322330d13d68e361545220d2c38c6e4b758974dfff3043566206f1536fb7f5d1a75791311308b1745465024bda176b11300").unwrap()).unwrap();
    pub static ref MPN_PAYMENT_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("6188b95b6f34dedecbe07b3d3fe6539b056e735f8f74d20ab54558ec897787a28f76645c190b6fdabec2cc72e158391579b45929b387439c6b426d5e101dc4ba7e0b3bb98bf0ba863f23db892173947e375a9803c789d0d2d6fca907d8628c1600d34ca462e1ba19a2b50ad23569f5c6be2999c4903f31c140e1dfd3ef0c5215b26e220fe22e5d5669ad39420bd9238a0a6af10963d5045e1b3d7b57032bd4653ff5a3badbdfc832452cb513aef3d937138c2ca79c76a274ec398c8c0ed5618e180010bf2d3b50533a545d46849dc7e3b5bf9ac4bbcbdfc9e346706a26a790c4f0010d98ace01fe81eb3fa723960b190f2162a45720e7b47a6713cefe0c310b8179fb922781121e6dce15c079eb7956c163ef66a3ff0eed2236b747ba71baced8e0d992eaa1036e121a6cf1623f575f66e3e8b4e4f863ad42c1d5875b9510a4f888e6fcd3fe669832535af4a3dee4b5ce012fc6a66b386575d65bf7c779003049450caf0f1e6ce5fa4709ad6e3a2ab05f24611be3c444b8a7e9113860bc0121f6701002269c310c20163fb01d94eac5a1ad0e4ca9398d020bf8bab9a379604e8e91ce6d2de9eb01407a4bf4174bba0b31c040c8a1906da2d487f2f0df199c4570170450c9ab41f6cff37543c868283076d1cfa9a5d55f34379facef3c36b8835d63710018763a0aa406f2e3368aef248a3d859cfb359c456070c831441a40cb4086e9497d6537e60a2865ba02e0dfa9cf4db174df30708b7ba170a198390a0894895757df7baa2cf9fd1934f12025fa1b9df85e714f715d8f39ac7f25434a4732a951000654735e1a386616a4d62f49daa44f34c3734f78038f0813291a9f006b8f781142f447f0aa29ba11b54beb6ed3f3f3715bb67081c131bf091fa2692c31e8bb96ab1e1062e630e181999a6d665cd6754b9a59d58e3ca8095ec9ae46ca49129a01500782525b6e60a654cb55ff288bd31ff89b818eb849c828186e52ddecc90986f5e0ee6ef33402d2778737cd1e31d1d10039fdcf9ef79ec86207072e53d6d8e42125b21afbbe252ea08a023aedf40360bff399f8f788fbfd95e870f49524869ab07bd3d09d8cae40f60bd5b84badd73595d6d7a778bfeacb9ab45fccb7793b37f26b8143c36f6c91b3c6066d4f85b1a7018934415f91ddabab97833a13fffcf78c536e9de8d1e86d39df0e4f9d1414cb3088a4acce9e995b6703dd26dd58d0cac0a0004000000000000000e5b860d556dbe05dd3870d3b21943c0ae8359d20e1b2206028476c76c72940869b6e342b5670b2581b05fd733f07216e3e54420c2b208e0118cb4e46a189f23baa6dfddc82403b8360312ea7fa11b4bcfb4da689a4ef1bc3ccd719a72f89e0c00609cf4debf34e6d084d9dd36eac56bb8a8c97f2b95b8d2517c9165e3055e405737abd8324ced2a56f7ab2338618a46143f6b807dfc32d3e5be687add733c7f4c5a376f1ec3f41f3f202c4faf98f7891efb2baf278474213b6b4a5a2387363b14004d7affe6a704f18a2fdb12aa5cf5f14f6c31254f92ce17505fa4391415955e0f47735ba1a0f0e53002adf7846085220f075fead4b5604c90bd2b553420267ba029999ef83bf73fb2623ff74f0dd1bf66bdde9aeb80abe05fd12547488fa90407001f96360e0877d7c78d5e1ac57a3500c5afc5652769dfce06ba3abd968bf52a02a65c2f575b3f6574ecd3d6901a7bde15a139864f64809909fa31c47d36e65923fc242969b855c7e37eb11c407c831648e9b967279b839d88511fb3bdc7d0d30100").unwrap()).unwrap();
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
        log4_payment_capacity: 1,
        payment_function: zk::ZkVerifierKey::Groth16(Box::new(MPN_PAYMENT_VK.clone())),
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
    println!("{}", ContractId::new(&mpn_contract_create_tx));
    assert_eq!(
        MPN_CONTRACT_ID.clone(),
        ContractId::new(&mpn_contract_create_tx)
    );
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
            contract.payment_function = zk::ZkVerifierKey::Dummy;
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

        // We expect a minimum number of MPN contract updates
        // in a block to consider it valid
        mpn_num_function_calls: 0,
        mpn_num_contract_payments: 1,
    }
}

#[cfg(test)]
pub fn get_test_blockchain_config() -> BlockchainConfig {
    let mpn_tx_delta = get_test_mpn_contract();
    let mpn_contract_id = ContractId::new(&mpn_tx_delta.tx);
    println!("CONT: {}", mpn_contract_id);

    let mut conf = get_blockchain_config();
    conf.mpn_num_contract_payments = 0;
    conf.mpn_num_function_calls = 0;
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
