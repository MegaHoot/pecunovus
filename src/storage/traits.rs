use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Iteration mode for scans
#[derive(Clone, Copy, Debug)]
pub enum IterMode {
    Prefix,   // iterate keys that have given prefix
    Range,    // iterate inclusive range (start, end)
}

/// Simple KV iterator returned by `scan_prefix`/`scan_range`
pub struct KvIter {
    // each item: (key, value)
    pub items: Vec<(Vec<u8>, Vec<u8>)>,
}

/// A write-batch abstraction to batch multiple put/delete operations atomically.
pub trait WriteBatch: Send + Sync {
    fn put(&mut self, key: Vec<u8>, value: Vec<u8>);
    fn delete(&mut self, key: Vec<u8>);
}

/// Core key-value store trait (synchronous)
#[async_trait]
pub trait KvStore: Send + Sync + 'static {
    fn name(&self) -> String;

    /// Put a key / value
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Get a key
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Delete a key
    async fn delete(&self, key: &[u8]) -> Result<()>;

    /// Check existence
    async fn exists(&self, key: &[u8]) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    /// Create a write batch object for this engine
    fn batch(&self) -> Box<dyn WriteBatch>;

    /// Apply a write batch atomically
    async fn write_batch(&self, batch: Box<dyn WriteBatch>) -> Result<()>;

    /// Scan by prefix or range. For simplicity returns full Vec; engines may stream in future.
    async fn scan_prefix(&self, prefix: &[u8]) -> Result<KvIter>;

    /// Path where the engine stores data (useful for debugging)
    fn path(&self) -> Option<PathBuf>;
}
