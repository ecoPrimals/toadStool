// SPDX-License-Identifier: AGPL-3.0-only
//! WebAssembly module caching for wasmi
//!
//! **Evolution** (Jan 17, 2026):
//! - OLD: wasmtime serialization (requires unsafe)
//! - NEW: wasmi Module is Clone! (100% safe!)
//!
//! This is MUCH simpler and safer than wasmtime caching!

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::debug;
use wasmi::Module;

/// Cached WASM module with metadata
#[derive(Clone, Debug)]
pub struct CachedModule {
    /// The wasmi Module (Clone is cheap!)
    pub module: Module,

    /// Last access timestamp
    pub last_used: Instant,

    /// Number of times this module has been accessed
    pub access_count: u64,
}

impl CachedModule {
    /// Create a new cached module entry
    pub fn new(module: Module) -> Self {
        Self {
            module,
            last_used: Instant::now(),
            access_count: 1,
        }
    }

    /// Record access to this module
    pub fn record_access(&mut self) {
        self.last_used = Instant::now();
        self.access_count += 1;
    }

    /// Get age of this cache entry in seconds
    pub fn age_seconds(&self) -> u64 {
        self.last_used.elapsed().as_secs()
    }
}

/// Module cache with LRU eviction
pub struct ModuleCache {
    /// Cached modules
    cache: Arc<RwLock<HashMap<String, CachedModule>>>,

    /// Maximum cache entries
    max_entries: usize,

    /// Cache hits
    hits: Arc<RwLock<u64>>,

    /// Cache misses
    misses: Arc<RwLock<u64>>,
}

impl ModuleCache {
    /// Create a new module cache
    pub fn new(max_entries: usize) -> Self {
        debug!("Creating wasmi module cache (max entries: {})", max_entries);
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a module from cache
    pub async fn get(&self, key: &str) -> Option<Module> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // Record hit
            entry.record_access();
            *self.hits.write().await += 1;

            debug!("Cache hit for key: {}", key);
            Some(entry.module.clone())
        } else {
            // Record miss
            *self.misses.write().await += 1;
            debug!("Cache miss for key: {}", key);
            None
        }
    }

    /// Insert a module into cache
    pub async fn insert(&self, key: String, module: Module) {
        let mut cache = self.cache.write().await;

        // Evict old entries if at capacity
        if cache.len() >= self.max_entries {
            self.evict_lru(&mut cache).await;
        }

        let cached = CachedModule::new(module);
        cache.insert(key.clone(), cached);
        let len = cache.len();
        drop(cache);
        debug!("Cached module with key: {} (total: {})", key, len);
    }

    /// Evict least recently used entry
    #[expect(
        clippy::unused_async,
        reason = "kept for API consistency with async cache interface"
    )]
    async fn evict_lru(&self, cache: &mut HashMap<String, CachedModule>) {
        if cache.is_empty() {
            return;
        }

        // Find LRU entry
        let lru_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone());

        if let Some(key) = lru_key {
            cache.remove(&key);
            debug!("Evicted LRU entry: {}", key);
        }
    }

    /// Get cache hit rate
    pub async fn hit_rate(&self) -> f64 {
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Clear the cache
    pub async fn clear(&self) {
        self.cache.write().await.clear();
        *self.hits.write().await = 0;
        *self.misses.write().await = 0;
        debug!("Cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasmi::Engine;

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = ModuleCache::new(10);
        let engine = Engine::default();

        // Create a simple WASM module
        let wasm = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm[..]).unwrap();

        // Insert into cache
        cache.insert("test_key".to_string(), module.clone()).await;

        // Retrieve from cache
        let cached = cache.get("test_key").await;
        assert!(cached.is_some(), "Module should be in cache");

        // Check hit rate
        let hit_rate = cache.hit_rate().await;
        assert!(hit_rate > 0.0, "Should have cache hit");
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = ModuleCache::new(2);
        let engine = Engine::default();

        let wasm = wat::parse_str("(module)").unwrap();
        let module = Module::new(&engine, &wasm[..]).unwrap();

        // Fill cache
        cache.insert("key1".to_string(), module.clone()).await;
        cache.insert("key2".to_string(), module.clone()).await;

        assert_eq!(cache.size().await, 2);

        // This should trigger eviction
        cache.insert("key3".to_string(), module.clone()).await;

        assert_eq!(cache.size().await, 2, "Cache should maintain max size");
    }
}
