#[cfg(feature = "rocksdb")]
use crate::storage::traits::{KvStore, WriteBatch, KvIter};
#[cfg(feature = "rocksdb")]
use anyhow::Result;
#[cfg(feature = "rocksdb")]
use rocksdb::{DB, Options, WriteBatch as RocksWriteBatch};
#[cfg(feature = "rocksdb")]
use std::path::Path;
#[cfg(feature = "rocksdb")]
use std::path::PathBuf;

#[cfg(feature = "rocksdb")]
pub struct RocksWriteBatchAdapter {
    batch: RocksWriteBatch,
}

#[cfg(feature = "rocksdb")]
impl RocksWriteBatchAdapter {
    pub fn new() -> Self { Self { batch: RocksWriteBatch::default() } }
}

#[cfg(feature = "rocksdb")]
impl crate::storage::traits::WriteBatch for RocksWriteBatchAdapter {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.batch.put(key, value);
    }
    fn delete(&mut self, key: Vec<u8>) {
        self.batch.delete(key);
    }
}

#[cfg(feature = "rocksdb")]
pub struct RocksKvStore {
    db: DB,
    path: PathBuf,
}

#[cfg(feature = "rocksdb")]
impl RocksKvStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        // performance tuning: set write buffer size, compaction, compression as needed
        let db = DB::open(&opts, path.as_ref())?;
        Ok(Self { db, path: path.as_ref().to_path_buf() })
    }
}

#[cfg(feature = "rocksdb")]
#[async_trait::async_trait]
impl crate::storage::traits::KvStore for RocksKvStore {
    fn name(&self) -> String { "rocksdb".into() }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.db.get(key)? {
            Some(v) => Ok(Some(v.to_vec())),
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    fn batch(&self) -> Box<dyn WriteBatch> {
        Box::new(RocksWriteBatchAdapter::new())
    }

    async fn write_batch(&self, batch: Box<dyn WriteBatch>) -> Result<()> {
        // downcast
        if let Some(b) = batch.downcast_ref::<RocksWriteBatchAdapter>() {
            self.db.write(b.batch.clone())?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("invalid batch type for rocksdb"))
        }
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> Result<KvIter> {
        // RocksDB doesn't have native prefix scan unless configured; use iterator
        let mut items = Vec::new();
        let iter = self.db.iterator(rocksdb::IteratorMode::Start);
        for item in iter {
            let (k, v) = item?;
            if k.starts_with(prefix) {
                items.push((k.to_vec(), v.to_vec()));
            }
        }
        Ok(KvIter { items })
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.path.clone())
    }
}
