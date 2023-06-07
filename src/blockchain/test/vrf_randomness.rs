use super::*;

#[test]
fn test_vrf_randomness_changes() {
    let validator = TxBuilder::new(&Vec::from("VALIDATOR"));
    let mut chain = KvStoreChain::new(
        db::RamKvStore::new(),
        blockchain::get_test_blockchain_config(),
    )
    .unwrap();

    for i in 0..100 {
        let prev_rand = chain.epoch_randomness().unwrap();
        let draft = chain
            .draft_block(1700000000 + i * 5, &[], &validator, true)
            .unwrap()
            .unwrap();
        chain.apply_block(&draft).unwrap();
        let post_rand = chain.epoch_randomness().unwrap();

        if i % 10 == 0 {
            assert!(prev_rand != post_rand);
        } else {
            assert!(prev_rand == post_rand);
        }
    }
}
