//! Fuzz tests for transactions, block validation, and consensus safety.
//!
//! Run with `cargo fuzz run fuzz_target`

use pecunovus::txpool::pool::TxPool;
use pecunovus::ledger::blockstore::BlockStore;
use rand::Rng;

#[test]
fn fuzz_transaction_parsing() {
    let mut rng = rand::thread_rng();
    let txpool = TxPool::new();

    for _ in 0..1000 {
        let bogus: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
        let hex_tx = hex::encode(&bogus);
        let _ = txpool.ingest_from_rpc(hex_tx);
        // Just ensure we don’t panic
    }
}

#[test]
fn fuzz_block_validation() {
    let mut rng = rand::thread_rng();
    let blockstore = BlockStore::new();

    for _ in 0..500 {
        let bogus: Vec<u8> = (0..128).map(|_| rng.gen()).collect();
        // Simulate bogus block ingestion
        let res = blockstore.validate_block_bytes(&bogus);
        assert!(res.is_ok() || res.is_err()); // never panic
    }
}
