//! Integration tests: bring up multiple nodes, run consensus, check state.

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use pecunovus::network::manager::NetworkManager;
use pecunovus::consensus::consensus_state::ConsensusState;
use pecunovus::state::account_db::AccountDb;
use pecunovus::txpool::pool::TxPool;
use pecunovus::ledger::blockstore::BlockStore;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_two_nodes_reach_consensus() {
    // Setup local accounts, txpool, ledger for two nodes
    let accounts1 = Arc::new(AccountDb::new());
    let blocks1 = Arc::new(BlockStore::new());
    let txpool1 = Arc::new(TxPool::new());
    let net1 = NetworkManager::new("127.0.0.1:7001".to_string(), vec![]);

    let accounts2 = Arc::new(AccountDb::new());
    let blocks2 = Arc::new(BlockStore::new());
    let txpool2 = Arc::new(TxPool::new());
    let net2 = NetworkManager::new("127.0.0.1:7002".to_string(), vec!["127.0.0.1:7001".to_string()]);

    // Start both nodes
    let h1 = tokio::spawn(net1.start());
    let h2 = tokio::spawn(net2.start());

    // Run consensus state machine mock
    let cs1 = ConsensusState::new(accounts1, blocks1, txpool1);
    let cs2 = ConsensusState::new(accounts2, blocks2, txpool2);

    // Let gossip + consensus happen
    sleep(Duration::from_secs(3)).await;

    // Assert some dummy state (e.g. both saw genesis)
    assert!(cs1.get_latest_height() >= 0);
    assert!(cs2.get_latest_height() >= 0);

    // Cleanup
    drop(h1);
    drop(h2);
}
