#[cfg(feature = "sled")]
use crate::storage::traits::{KvStore, WriteBatch, KvIter};
#[cfg(feature = "sled")]
use anyhow::Result;
#[cfg(feature = "sled")]
use sled;
#[cfg(feature = "sled")]
use std::path::PathBuf;

#[cfg(feature = "sled")]
pub struct SledWriteBatchAdapter {
    batch: sled::Batch,
}

#[cfg(feature = "sled")]
impl SledWriteBatchAdapter {
    pub fn new() -> Self { Self { batch: sled::Batch::default() } }
}

#[cfg(feature = "sled")]
impl crate::storage::traits::WriteBatch for SledWriteBatchAdapter {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.batch.insert(key, value);
    }
    fn delete(&mut self, key: Vec<u8>) {
        self.batch.remove(key);
    }
}

#[cfg(feature = "sled")]
pub struct SledKvStore {
    db: sled::Db,
    path: PathBuf,
}

#[cfg(feature = "sled")]
impl SledKvStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let db = sled::open(path.as_ref())?;
        Ok(Self { db, path: path.as_ref().to_path_buf() })
    }
}

#[cfg(feature = "sled")]
#[async_trait::async_trait]
impl crate::storage::traits::KvStore for SledKvStore {
    fn name(&self) -> String { "sled".into() }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.insert(key, value)?;
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.db.get(key)? {
            Some(v) => Ok(Some(v.to_vec())),
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    fn batch(&self) -> Box<dyn WriteBatch> {
        Box::new(SledWriteBatchAdapter::new())
    }

    async fn write_batch(&self, batch: Box<dyn WriteBatch>) -> Result<()> {
        if let Some(b) = batch.downcast_ref::<SledWriteBatchAdapter>() {
            self.db.apply_batch(b.batch.clone())?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("invalid batch type for sled"))
        }
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> Result<KvIter> {
        let mut items = Vec::new();
        let iter = self.db.iter();
        for res in iter {
            let (k, v) = res?;
            if k.as_ref().starts_with(prefix) {
                items.push((k.to_vec(), v.to_vec()));
            }
        }
        Ok(KvIter { items })
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.path.clone())
    }
}
