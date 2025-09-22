use crate::ledger::blockstore::BlockStore;
use anyhow::Result;

pub struct LedgerPruner {
    retain_slots: u64,
}

impl LedgerPruner {
    pub fn new(retain_slots: u64) -> Self {
        Self { retain_slots }
    }

    pub fn prune(&self, store: &mut BlockStore) -> Result<()> {
        // TODO: Track oldest slot, delete older ones
        // For now, placeholder log
        println!("🧹 Pruning ledger, retaining last {} slots", self.retain_slots);
        Ok(())
    }
}
