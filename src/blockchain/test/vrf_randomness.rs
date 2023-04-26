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
        "aad4bc1125e44703c1255f22c30d8cf4c7cfcb6207b17945ffc0696c5a7e5269",
        "b463e23ad6582b8b08cb4fab021b2bc1329525c8ff885157d931459906730002",
        "cd260c94eebef17912915d6fe3392da9012362ace59aefab39879d8047120b76",
        "d3381bc10949e98ebc578ab046fa1f6ba037bd25363aac28a5d04691c2310d5f",
        "04dd885b1d712c2a0e1e7ce8c26e469c547aefc7b3cc5d66da36491592afbff1",
        "d3c92f96b5a3c5ce12f33e99dfe8e0dedccefeca1fba24c1e8e23d7dda5fab36",
        "57a92de42f1bb687c5fd889d308fa4bac64a3a8c007550e6667bc0010344f200",
        "a697c8d85e729ebc3f63e503a600e982ef16488328368e6fedac3146910f4a00",
        "3b1eb86ea84ac6df14f5128f1ac50e024e0f0cbe8c5b55cd896a3735e29bbcaf",
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
