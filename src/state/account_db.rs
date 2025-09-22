//! Persistent Account DB abstractions.
//! - Account struct (lamports, data, owner, executable, rent_epoch)
//! - AccountStore trait (pluggable persistence engine)
//! - RocksAccountStore (rocksdb backend)
//! - InMemAccountStore (simple HashMap for tests/dev)

use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    /// lamports / balance
    pub lamports: u64,
    /// owner program id (hex string)
    pub owner: String,
    /// raw account data
    pub data: Vec<u8>,
    /// is executable account (a program)
    pub executable: bool,
    /// rent epoch (for rent collection)
    pub rent_epoch: u64,
}

impl Account {
    pub fn new(lamports: u64, owner: &str, data: Vec<u8>) -> Self {
        Self {
            lamports,
            owner: owner.to_string(),
            data,
            executable: false,
            rent_epoch: 0,
        }
    }
}

/// Account key type (hex string of public key)
pub type AccountKey = String;

/// Trait for an account persistence engine.
pub trait AccountStore: Send + Sync + 'static {
    fn get(&self, key: &AccountKey) -> Result<Option<Account>>;
    fn insert(&self, key: AccountKey, account: Account) -> Result<()>;
    fn remove(&self, key: &AccountKey) -> Result<()>;
    fn scan_prefix(&self, prefix: &str) -> Result<Vec<(AccountKey, Account)>>;
}

/// In-memory account store (good for tests/dev)
#[derive(Debug, Default, Clone)]
pub struct InMemAccountStore {
    inner: Arc<RwLock<std::collections::HashMap<AccountKey, Account>>>,
}

impl InMemAccountStore {
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(std::collections::HashMap::new())) }
    }
}

impl AccountStore for InMemAccountStore {
    fn get(&self, key: &AccountKey) -> Result<Option<Account>> {
        Ok(self.inner.read().get(key).cloned())
    }

    fn insert(&self, key: AccountKey, account: Account) -> Result<()> {
        self.inner.write().insert(key, account);
        Ok(())
    }

    fn remove(&self, key: &AccountKey) -> Result<()> {
        self.inner.write().remove(key);
        Ok(())
    }

    fn scan_prefix(&self, prefix: &str) -> Result<Vec<(AccountKey, Account)>> {
        let map = self.inner.read();
        let mut out = Vec::new();
        for (k, v) in map.iter() {
            if k.starts_with(prefix) {
                out.push((k.clone(), v.clone()));
            }
        }
        Ok(out)
    }
}

#[cfg(feature = "rocksdb")]
mod rocks {
    use super::*;
    use rocksdb::{DB, Options};
    use bincode;

    pub struct RocksAccountStore {
        db: Arc<DB>,
    }

    impl RocksAccountStore {
        pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            let db = DB::open(&opts, path.as_ref().join("accounts.db"))?;
            Ok(Self { db: Arc::new(db) })
        }
    }

    impl AccountStore for RocksAccountStore {
        fn get(&self, key: &AccountKey) -> Result<Option<Account>> {
            match self.db.get(key.as_bytes())? {
                Some(val) => {
                    let acc: Account = bincode::deserialize(&val)?;
                    Ok(Some(acc))
                }
                None => Ok(None),
            }
        }

        fn insert(&self, key: AccountKey, account: Account) -> Result<()> {
            let bin = bincode::serialize(&account)?;
            self.db.put(key.as_bytes(), bin)?;
            Ok(())
        }

        fn remove(&self, key: &AccountKey) -> Result<()> {
            self.db.delete(key.as_bytes())?;
            Ok(())
        }

        fn scan_prefix(&self, prefix: &str) -> Result<Vec<(AccountKey, Account)>> {
            let mut out = Vec::new();
            let iter = self.db.iterator(rocksdb::IteratorMode::Start);
            for item in iter {
                let (k, v) = item?;
                let kstr = String::from_utf8_lossy(&k).to_string();
                if kstr.starts_with(prefix) {
                    let acc: Account = bincode::deserialize(&v)?;
                    out.push((kstr, acc));
                }
            }
            Ok(out)
        }
    }
}

#[cfg(feature = "rocksdb")]
pub use rocks::RocksAccountStore;
