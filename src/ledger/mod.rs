pub mod blockstore;
pub mod snapshot;
pub mod pruner;
pub mod index;

use blockstore::BlockStore;
use snapshot::SnapshotManager;
use pruner::LedgerPruner;
use index::BlockIndex;

/// Ledger service that stores finalized blocks and provides access APIs
pub struct Ledger {
    pub store: BlockStore,
    pub snapshot_mgr: SnapshotManager,
    pub pruner: LedgerPruner,
    pub index: BlockIndex,
}

impl Ledger {
    pub fn new(path: &str) -> Self {
        Self {
            store: BlockStore::new(path),
            snapshot_mgr: SnapshotManager::new(path),
            pruner: LedgerPruner::new(100_000), // keep 100k slots
            index: BlockIndex::new(),
        }
    }

    pub fn append_block(&mut self, slot: u64, data: Vec<u8>) -> anyhow::Result<()> {
        self.store.write_block(slot, &data)?;
        self.index.add(slot);
        Ok(())
    }

    pub fn get_block(&self, slot: u64) -> Option<Vec<u8>> {
        self.store.read_block(slot).ok()
    }

    pub fn prune(&mut self) -> anyhow::Result<()> {
        self.pruner.prune(&mut self.store)
    }

    pub fn take_snapshot(&self, slot: u64) -> anyhow::Result<()> {
        self.snapshot_mgr.create(slot, &self.store)
    }
}
