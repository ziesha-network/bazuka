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
        bincode::deserialize(&hex::decode("cfcab6bcb1f6d515710e0e7d5270a137a71f0e2c0f01f45fbeeb218d8e2cf472d60ca1fd93a60de2d8bcfcc1c96e2b149d2e0021cda15e551e7978ff370c79c9e2405d8fd5bcf2e2ebc531328f923ba8f2012b11ed0f2b22bd3a35f6c51f2207007e17daa6f4bddff1241c2b0dd7a1e99212474bf927af1be13076e77b530019c10720abd2c2503004ef930a3632d074037e4cd19669a324d6ac00a24011fb47704ae4d5183993e8005449594b7bd75b3c0976a101c705a5f47b60631e146ee510008bf8fc082620d255ea015e836f285ee66b03617ac0408769aee6af084f33dbe92cc2537af6445d1b4c456bd0ac59d60d98f43fd5bcf5848407d70dc0b03b508304d82bf37e8466ccec1d4c944144f7f623b265d92af0cedcbf5ba05f9683e70c263ff55d0787b15769d49f090d22ef5042c845db297e35dab461a243001b7c5dde191607057b8a380bb092bbfb4e2700a19266e7689c2c90d0a9cfa55648e899dbf83f5009e82a216fe820a4cfd75b4fc6f1a9f2e1e0556c4d0d597d2f2db90000e748cd7ca0bc8ff80287d38107d8ce9c498906658cfcf71a9bf311a1a8394b3c56a73707013b1d25708d2c8b6cee63034b3a2b202e5d2e792cf41ec4a5ec7bfd502b82330fc626532e21af780819f70b72c2175790a84f0f9265121686bc120c4c11f9145450fbea56ad05b4e0a243356d5ba93843ff7055831fc1a20c3a9a759c9b497f2ed38307a777f3a58fc2e9160026e390180f2880c48b663400bb711306bc0048410bb1e601f06d0bd7c596a6979991365f0f48ee3aa14942b561f60d003a7aa615c344911aa99114b21ef36331c655818f614c2c92e925b91042a1ceda52f7767d8d2afcffeed6a15771441507d60f4b4c4429e9122c9fd852d03e3b8b70753e2fbce909cee71d506b7aa7195b221f4d13a63e34649fff5e8e526c331500bd7436834ea206aa3837a65b0b75f9ac9f881e21ec81562cf51a0292dab698ea7322cb95eb7ce1487b9b081bfee11b1955c385f2e5c27e36546ec2dae546d1fb13afa16f3bf93671c9da0320980899b175c9d0069f3411f564e9f3c2ac2d120bd1b5d90f8e58707d5583c9898dd8e5b31dfdf9d143ddc69987ee760ae06ea6377dde6b1379788948da6b2397044be609187c06757fddeaaf190806699654119e5b411ac53203a7627e6cb3d86a64ac584dc0b6f55c651acbec784bc47ef9ad030004000000000000000087181f4425793626a433ac393cde1f8030608fd1257bcfc2393f31a4b023d6b3e68e10195ced8c4638ec2ab9b5890e42a0a836ea9c52e0d205a37e009cd0753afcc944d943265745eaafac24add07d243915b5fe21d2e9bc93e440adad560100fa8089ecd1f826cfb7c097614ae6360d9cd7a6afb77b2da18969fe3e62fd23b90798b41e5231e5c92ab532d710805001c96f1018b02c26579906950cb6e38364f2be3cf00da20001070b0884e69cad38f55fc259422dc0e5c60174b6fd8c460d0081977fb90d7a2478c138a534ae2f9212ad31be8066b87928ba6b4fb17943e6b0a5b50c0744fa7660d8f94a61b00a6a1217620a99e77e941986e1f8f6fa3906bafd844a267b4ef49c64052cb06d3af7670c4beb3885e37b853d3813c4ab7f76030097037c238deb213c1a17737e17784c8bcf74fc53e5a3ecda80d21326869e28cfb61c7b64d7277ec5cba2094f56e7f40d03cab73d4dc30d861e08c859b5feb3f204b9fa68814060b6c465be639961909603d8d6eb52fe2cb7cab11cd00dfa940200").unwrap()).unwrap();
    pub static ref MPN_DEPOSIT_WITHDRAW_VK: zk::groth16::Groth16VerifyingKey =
        bincode::deserialize(&hex::decode("95688a7f81c45354868d6514332c9321b1e3b04218ed8c2a950701b7f3452eff82877324cea6708680dae0690e6e080d28e5c11594e21453e11c2ae8833ec631854b1bbba659198744a17bcfa29a813116ad99a054ecbc8049ef246233af1b190033269ceae94bfb3d7f90d478d31a5783a0f45f39a9f5475b26d33c74308a3793fdc6e8cff19f51a79646ea9aa324c911e13b7d02484855b0142c654445e037a21a4bdfc14ede024bfd05cc91efb6ad206a593dcd580357fdbb744de3d1f28d11007c0d9559c1ea8165d2cbe12d2af0697a4c27bd92a23437d14ecf71416616c88b84deddfb1768d6fc35afd0d5904bce1772b7654eae66d94332882ca43e7512d2f65581edaa8503885fee05bb79050ff6b5d0f98e31e62a8458071d1e857a0d184184db185d14a50e75a1d473ed4adf11e588b6df1fef6b89bc4772f44969b744c9e474a9dce4479ce32f68b135ad5d072cd2582c21016fb85a9ec815ffa8510c5ef5b5fb2d312b1903651ba2594435f60756945709220697183a4ae17d2ab3030032158a576a23130177092a47156d2d002628c5288533fae2f1f21ebb554a5c74a0f1de0efcdb46ee69d8567a000f1709e1317aa9be493e9601dd89e554a6910922acd6459410396609a320151e1d0efb922274d32acf8349de46d98203f8010dacfce6ec5b317e3729e4a867fda6c03fef38311061e9cd02ebc0c4ffd363281fd41c948139f0b1d725af181e098fd9072292194673bb644735e519b8b649ff5a79b4c4e2c3797a9675c49d61b6982d94542c7c9ee02e0c3522147ee15561730d00cb46ffc4af1074615b29f9d2e37452a8974fd4a1d29bdcf73aa32b49d8ecbb16f99e7ac3be116778e4c932953e105b0b89c80145fd65edae06422073f2a7016c707f77f34fa04b44dfa2c997f53b9d86ea7de6677243806086526cd125b66b0800f02dedf9fbf52cc609c8a1ba793a03eee39649a992099c98409fd3129b4141d5a302f0f1dd51cf08a544f7d4a29208185534d435696a174e45f09ef5b4d92c1e013ccf83650fe99c4a87b203b25cba8b179c6a37df330013016805e1e7be8f08ea1e699025a739ca2e97eb6f9059fa9e990fa1b48b89697785db0c92323755b2959b8e86a8515e650682e35e989c0806cf6d8c2a6ea0afd41b4f4b32e10a3142905ea3173741a46429a9f8639b36597d261a85c614d6ec77ff73124ee0035102000400000000000000420b460d6a35080abb54e07762a59488353a48f0f10b39a2d03c78abb8aca3edbe314a07cf6fdd21269d0404e1aa63012056e83ebdac4f047e2a9ae7fb0b512327bde5657392792c647e1a2a631fd14c049e8ae7a65733f5bedb2d48fe1f99120082b224cc7522c5c20bbc167cfe8be62a924369040a22f3dfa407008ba823a04afee57ba0f2015170df0bcc02880a3b19226917b8c460a9e206ad68907bf3f539ee3c7a566eb922292d918f605f01a0ce423210b315dbbb11ee3b0fd3f804121500a3d64a0b377082828b690aa0350df9d530b207a8eeb8e0d2c024a9797cfa06ef69cc725d459f1d101fca8e8c292489073f11456913eedcc8d4f6fe41994b7957f661f5e8986e74bf31c12eab0aa34eeeeb8ed9053e2595a7e34afca74189880c00fbae36bdabc66c24bf41ac0ea400ee085ca71ecc7cc36ea5d3ea62ff612d72046140f1f21f8c786a6aa479d68d588c0f185b56d8d02f2a9d12af5137cbacc4b4a96ef9bfbecabc2ea53f909f312168f2f7910b7f1e283a4c643e6dde1548610600").unwrap()).unwrap();
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
