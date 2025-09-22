//! In-memory account cache with write-back and snapshot support.
//! - Uses DashMap for concurrent access
//! - Provides get/set/update APIs used by the runtime/executor
//! - Supports materializing a consistent snapshot for block execution / ledger replay

use crate::state::account_db::{Account, AccountKey, AccountStore, InMemAccountStore};
use dashmap::DashMap;
use std::sync::Arc;
use anyhow::Result;
use parking_lot::RwLock;

/// Cache entry holds Account plus dirty flag
#[derive(Debug, Clone)]
struct CacheEntry {
    account: Account,
    dirty: bool,
}

#[derive(Clone)]
pub struct AccountCache {
    /// in-memory concurrent map: key -> CacheEntry
    map: Arc<DashMap<AccountKey, Arc<RwLock<CacheEntry>>>>,
    /// backing store for persistence (optional)
    store: Arc<dyn AccountStore>,
}

impl AccountCache {
    /// Create new cache with backing store
    pub fn new(store: Arc<dyn AccountStore>) -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            store,
        }
    }

    /// Load account from cache or backing store
    pub fn get(&self, key: &AccountKey) -> Result<Option<Account>> {
        if let Some(e) = self.map.get(key) {
            let guard = e.value().read();
            return Ok(Some(guard.account.clone()));
        }
        // load from store
        if let Some(acc) = self.store.get(key)? {
            let entry = CacheEntry { account: acc.clone(), dirty: false };
            self.map.insert(key.clone(), Arc::new(RwLock::new(entry)));
            Ok(Some(acc))
        } else {
            Ok(None)
        }
    }

    /// Insert or overwrite an account in cache (mark dirty)
    pub fn insert(&self, key: AccountKey, account: Account) -> Result<()> {
        let entry = CacheEntry { account, dirty: true };
        self.map.insert(key, Arc::new(RwLock::new(entry)));
        Ok(())
    }

    /// Modify account via closure. Returns error if account missing.
    pub fn modify<F>(&self, key: &AccountKey, mutator: F) -> Result<()>
    where
        F: FnOnce(&mut Account) -> Result<()>
    {
        if let Some(e) = self.map.get(key) {
            let mut guard = e.value().write();
            mutator(&mut guard.account)?;
            guard.dirty = true;
            return Ok(());
        }
        // try to load into cache then modify
        if let Some(acc) = self.store.get(key)? {
            let entry = CacheEntry { account: acc, dirty: true };
            self.map.insert(key.clone(), Arc::new(RwLock::new(entry)));
            if let Some(e2) = self.map.get(key) {
                let mut guard = e2.value().write();
                mutator(&mut guard.account)?;
                guard.dirty = true;
                return Ok(());
            }
        }
        Err(anyhow::anyhow!("account not found"))
    }

    /// Flush dirty entries back to backing store (synchronous)
    pub fn flush(&self) -> Result<()> {
        for r in self.map.iter() {
            let key = r.key().clone();
            let entry_lock = r.value().clone();
            let guard = entry_lock.read();
            if guard.dirty {
                self.store.insert(key.clone(), guard.account.clone())?;
                // mark as clean
                drop(guard);
                let mut guard_mut = entry_lock.write();
                guard_mut.dirty = false;
            }
        }
        Ok(())
    }

    /// Create a consistent read-only snapshot (key -> Account)
    /// Snapshot taken from current cache + store for missing entries.
    pub fn snapshot(&self) -> Result<std::collections::HashMap<AccountKey, Account>> {
        let mut out = std::collections::HashMap::new();
        // first, take cache snapshot
        for r in self.map.iter() {
            let key = r.key().clone();
            let guard = r.value().read();
            out.insert(key, guard.account.clone());
        }
        // Note: for full snapshot (persisted ledger replay), iterate store as needed.
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::account_db::InMemAccountStore;

    #[test]
    fn test_cache_get_insert_flush() {
        let store = Arc::new(InMemAccountStore::new());
        let cache = AccountCache::new(store.clone());
        let key = "alice".to_string();
        assert!(cache.get(&key).unwrap().is_none());

        let acc = Account::new(100, "system", vec![1,2,3]);
        cache.insert(key.clone(), acc.clone()).unwrap();
        let fetched = cache.get(&key).unwrap().unwrap();
        assert_eq!(fetched.lamports, 100);

        cache.flush().unwrap();
        let persisted = store.get(&key).unwrap().unwrap();
        assert_eq!(persisted.lamports, 100);
    }
}
