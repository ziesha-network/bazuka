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
        "bf18a544e246975de5c3e430d0760b5199e671758a3f716cebe13c1f3ee04751",
        "5176b2367e20cf6f28905f6809353a23e548d091abaf0a6c6187c29d79d91294",
        "3f022f233d1631c132312ccda9eb5447673258ed9dfa773a1a13046102bbc457",
        "41ff6fff354c38e0e0e4e96a0d9679690727ee6818d557e4a1f8129fcc49d6ff",
        "f64358d619ba2a22eee64cb31594b220a308640ba164410f6124b48c707589ae",
        "7f6a98dd4a01a721746cab9156a7a6fa015d06da6d5843db87b61a1b48a066a8",
        "20e97625a54888adba17a7d47afd81a09f10ef536165e6c43147ab324345f028",
        "9e0d908ad767e7e4e300edf473a55125ebe2a6f12c1c9d7896a6a1ab5cf91e8f",
        "46d353ee118385e4ec33ffacbaf394029efe420fc17b64274e68fb01e0b60f9e",
    ];
    for i in 0..100 {
        let draft = chain
            .draft_block(1700000000 + i * 5, &[], &validator, true)
            .unwrap()
            .unwrap();
        chain.apply_block(&draft.block).unwrap();
        assert_eq!(
            hex::encode(chain.epoch_randomness().unwrap()),
            expected[(i / 10) as usize]
        );
    }
}
