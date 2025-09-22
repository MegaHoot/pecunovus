//! Storage module: abstraction over persistent key-value backends.
//!
//! Engines: RocksDB (feature "rocksdb"), sled (feature "sled"), simple FS fallback.
//! Use `storage::open(path, Engine)` to create an Arc<dyn KvStore> to pass to other modules.

pub mod traits;
pub mod fs_store;

#[cfg(feature = "rocksdb")]
pub mod rocksdb_store;

#[cfg(feature = "sled")]
pub mod sled_store;

pub use traits::{KvStore, WriteBatch, IterMode, KvIter};
pub use fs_store::FsKvStore;

#[cfg(feature = "rocksdb")]
pub use rocksdb_store::RocksKvStore;

#[cfg(feature = "sled")]
pub use sled_store::SledKvStore;

use std::sync::Arc;
use anyhow::Result;
use std::path::Path;

/// Engine selection enum
pub enum StorageEngine {
    Fs,
    #[cfg(feature = "rocksdb")]
    RocksDb,
    #[cfg(feature = "sled")]
    Sled,
}

/// Open a KvStore from path using preferred engine (falls back to Fs if unavailable)
pub fn open(path: impl AsRef<Path>, engine: StorageEngine) -> Result<Arc<dyn KvStore>> {
    match engine {
        StorageEngine::Fs => {
            let s = FsKvStore::open(path)?;
            Ok(Arc::new(s))
        }
        #[cfg(feature = "rocksdb")]
        StorageEngine::RocksDb => {
            let s = RocksKvStore::open(path)?;
            Ok(Arc::new(s))
        }
        #[cfg(feature = "sled")]
        StorageEngine::Sled => {
            let s = SledKvStore::open(path)?;
            Ok(Arc::new(s))
        }
    }
}
