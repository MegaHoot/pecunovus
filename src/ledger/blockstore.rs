use rocksdb::{DB, Options};
use anyhow::Result;

pub struct BlockStore {
    db: DB,
}

impl BlockStore {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, format!("{}/blockstore", path)).unwrap();
        Self { db }
    }

    pub fn write_block(&self, slot: u64, data: &[u8]) -> Result<()> {
        self.db.put(slot.to_be_bytes(), data)?;
        Ok(())
    }

    pub fn read_block(&self, slot: u64) -> Result<Vec<u8>> {
        match self.db.get(slot.to_be_bytes())? {
            Some(val) => Ok(val.to_vec()),
            None => Err(anyhow::anyhow!("Block not found")),
        }
    }

    pub fn delete_block(&self, slot: u64) -> Result<()> {
        self.db.delete(slot.to_be_bytes())?;
        Ok(())
    }
}
