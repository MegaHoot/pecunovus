use std::collections::BTreeSet;

pub struct BlockIndex {
    slots: BTreeSet<u64>,
}

impl BlockIndex {
    pub fn new() -> Self {
        Self { slots: BTreeSet::new() }
    }

    pub fn add(&mut self, slot: u64) {
        self.slots.insert(slot);
    }

    pub fn contains(&self, slot: u64) -> bool {
        self.slots.contains(&slot)
    }

    pub fn latest(&self) -> Option<u64> {
        self.slots.iter().rev().next().cloned()
    }
}
