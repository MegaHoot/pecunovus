use crate::storage::traits::{KvStore, WriteBatch, KvIter, IterMode};
use anyhow::Result;
use std::path::{PathBuf, Path};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FsWriteBatch {
    ops: Vec<(bool, Vec<u8>, Vec<u8>)>, // (is_put, key, value)
}

impl FsWriteBatch {
    pub fn new() -> Self { Self { ops: Vec::new() } }
}

impl WriteBatch for FsWriteBatch {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.ops.push((true, key, value));
    }
    fn delete(&mut self, key: Vec<u8>) {
        self.ops.push((false, key, vec![]));
    }
}

pub struct FsKvStore {
    dir: PathBuf,
    // simple in-memory index to speed up get (persisted anyway)
    index: Mutex<HashMap<Vec<u8>, PathBuf>>,
}

impl FsKvStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let dir = path.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir, index: Mutex::new(HashMap::new()) })
    }

    fn key_path(&self, key: &[u8]) -> PathBuf {
        // derive filename from hex of key
        let name = hex::encode(key);
        self.dir.join(name)
    }
}

#[async_trait::async_trait]
impl KvStore for FsKvStore {
    fn name(&self) -> String { "fs".into() }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let p = self.key_path(key);
        let mut f = OpenOptions::new().create(true).write(true).truncate(true).open(&p)?;
        f.write_all(value)?;
        let mut idx = self.index.lock().unwrap();
        idx.insert(key.to_vec(), p);
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let p = self.key_path(key);
        if !p.exists() { return Ok(None); }
        let mut f = OpenOptions::new().read(true).open(&p)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(Some(buf))
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let p = self.key_path(key);
        if p.exists() { fs::remove_file(&p)?; }
        let mut idx = self.index.lock().unwrap();
        idx.remove(key);
        Ok(())
    }

    fn batch(&self) -> Box<dyn WriteBatch> {
        Box::new(FsWriteBatch::new())
    }

    async fn write_batch(&self, batch: Box<dyn WriteBatch>) -> Result<()> {
        // downcast to FsWriteBatch expected
        if let Some(b) = batch.downcast_ref::<FsWriteBatch>() {
            for op in &b.ops {
                if op.0 {
                    self.put(&op.1, &op.2).await?;
                } else {
                    self.delete(&op.1).await?;
                }
            }
            Ok(())
        } else {
            // fallback: try to serialize ops via Debug — but we expect correct type
            Err(anyhow::anyhow!("invalid batch type for fs store"))
        }
    }

    async fn scan_prefix(&self, prefix: &[u8]) -> Result<KvIter> {
        let mut items = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().into_string().unwrap_or_default();
            let key = match hex::decode(name) {
                Ok(k) => k,
                Err(_) => continue,
            };
            if key.starts_with(prefix) {
                let mut f = OpenOptions::new().read(true).open(entry.path())?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                items.push((key, buf));
            }
        }
        Ok(KvIter { items })
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.dir.clone())
    }
}
