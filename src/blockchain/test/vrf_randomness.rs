use super::*;

#[test]
fn test_vrf_randomness_changes() {
    let validator = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    let expected = [
        "0000000000000000000000000000000000000000000000000000000000000000",
        "1fb667e616e3e0778cf6f8f787761c6eb810893dc7e9906ab26c519e688e6b68",
        "68f18aa94f2abeed4507b775aa2e5b503dd5b86ddfa842a7fe31343441cc9afa",
        "0b6fac0de7b1c6a0f9f9be1775245bebbbc8474bca4cc65f750e8f31855824cf",
        "8e5710de01051390bdb7f1bb1c43b54424401fafe80b048a11792ef98c5f85e6",
        "95cbc4b5c1f35aaa99ffe166d05445472bf5a3b7643259f2927e1e3329160ee2",
        "119b969f72fd06a3d8fec4861bad921089903349ab9ef4e2ff6b0319f9e59390",
        "53d1df368d9ad9e7a025d1e6cef49b95747ba2c8ac908b102e6ed9bc29b8035a",
        "a69225ad97db281cad80595ec54076c4d752432ca0c8c689e578dfa636fc78b7",
        "5262d9d9ec7a04702ea20dcfec40034822953d966ff7654514c293d3fee4d633",
    ];
    for i in 0..100 {
        let draft = chain
            .draft_block(1700000000 + i * 5, &[], &validator, true)
            .unwrap()
            .unwrap();
        chain.apply_block(&draft).unwrap();
        assert_eq!(
            hex::encode(chain.epoch_randomness().unwrap()),
            expected[(i / 10) as usize]
        );
    }
}
