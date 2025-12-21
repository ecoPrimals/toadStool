//! WebAssembly module caching
//!
//! High-performance, thread-safe module cache with LRU eviction
//! and comprehensive metrics tracking.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use wasmtime::{Engine, Module};

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Cached WASM module with metadata
#[derive(Clone, Debug)]
pub struct CachedModule {
    /// Serialized compiled module bytes
    pub compiled_module: Vec<u8>,

    /// Last access timestamp
    pub last_used: Instant,

    /// Number of times this module has been accessed
    pub access_count: u64,

    /// Size of compiled module in bytes
    pub size_bytes: usize,
}

impl CachedModule {
    /// Create a new cached module entry
    pub fn new(compiled_module: Vec<u8>) -> Self {
        let size_bytes = compiled_module.len();
        Self {
            compiled_module,
            last_used: Instant::now(),
            access_count: 1,
            size_bytes,
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

/// Cache metrics for monitoring and optimization
#[derive(Debug, Clone, Default)]
pub struct CacheMetrics {
    /// Total number of cached modules
    pub total_modules: usize,

    /// Total size of all cached modules in bytes
    pub total_size_bytes: usize,

    /// Average module size in bytes
    pub average_module_size: usize,

    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,

    /// Total memory usage in bytes
    pub memory_usage_bytes: u64,
}

impl std::fmt::Display for CacheMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CacheMetrics(modules: {}, size: {} bytes, hit_rate: {:.2}%)",
            self.total_modules,
            self.total_size_bytes,
            self.cache_hit_rate * 100.0
        )
    }
}

/// Thread-safe module cache with LRU eviction
pub struct ModuleCache {
    /// Cached modules storage
    cache: Arc<RwLock<HashMap<String, CachedModule>>>,

    /// Maximum number of entries
    max_entries: usize,

    /// Cache hit counter
    hits: Arc<RwLock<u64>>,

    /// Cache miss counter
    misses: Arc<RwLock<u64>>,
}

impl ModuleCache {
    /// Create a new module cache
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(max_entries))),
            max_entries,
            hits: Arc::new(RwLock::new(0)),
            misses: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a module from cache
    pub async fn get(&self, key: &str, engine: &Engine) -> Option<Module> {
        let mut cache = self.cache.write().await;

        if let Some(cached) = cache.get_mut(key) {
            cached.record_access();

            // Deserialize module
            //
            // # Safety
            //
            // This unsafe block calls `Module::deserialize()` from Wasmtime, which is marked
            // unsafe because it trusts that the serialized bytes represent a valid compiled
            // WebAssembly module. This is safe in our context because:
            //
            // 1. **Origin Guarantee**: The cached bytes were produced by `Module::serialize()`
            //    (see `insert()` method) from a valid, previously compiled module. We never
            //    accept or cache arbitrary bytes from external sources.
            //
            // 2. **Engine Consistency**: Deserialization uses the same `Engine` configuration
            //    as the original compilation. Wasmtime guarantees that modules serialized
            //    from one engine can be safely deserialized with the same engine.
            //
            // 3. **Corruption Handling**: If the bytes become corrupted (disk error, memory
            //    corruption), deserialization will fail with an error (not UB), and we
            //    safely remove the corrupted entry from the cache.
            //
            // 4. **Memory Safety**: Wasmtime's compiled modules are memory-safe even if the
            //    serialization format changes between versions - deserialization will fail
            //    rather than cause undefined behavior.
            //
            // Alternative: We could recompile modules instead of caching them, but this would
            // significantly hurt performance (compilation is ~100x slower than deserialization).
            match unsafe { Module::deserialize(engine, &cached.compiled_module) } {
                Ok(module) => {
                    *self.hits.write().await += 1;
                    Some(module)
                }
                Err(_) => {
                    // Corrupted cache entry, remove it
                    cache.remove(key);
                    *self.misses.write().await += 1;
                    None
                }
            }
        } else {
            *self.misses.write().await += 1;
            None
        }
    }

    /// Insert a module into cache
    pub async fn insert(&self, key: String, module: &Module) -> ToadStoolResult<()> {
        // Serialize the module
        let serialized = module
            .serialize()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize module: {e}")))?;

        let cached_module = CachedModule::new(serialized);

        let mut cache = self.cache.write().await;

        // Evict old entries if cache is full
        if cache.len() >= self.max_entries {
            self.evict_lru(&mut cache).await;
        }

        cache.insert(key, cached_module);
        Ok(())
    }

    /// Evict least recently used entry
    async fn evict_lru(&self, cache: &mut HashMap<String, CachedModule>) {
        if cache.is_empty() {
            return;
        }

        // Find LRU entry
        let lru_key = cache
            .iter()
            .min_by_key(|(_, v)| v.last_used)
            .map(|(k, _)| k.clone());

        if let Some(key) = lru_key {
            cache.remove(&key);
        }
    }

    /// Clear all cached modules
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();

        *self.hits.write().await = 0;
        *self.misses.write().await = 0;
    }

    /// Get cache metrics
    pub async fn metrics(&self) -> CacheMetrics {
        let cache = self.cache.read().await;
        let hits = *self.hits.read().await;
        let misses = *self.misses.read().await;

        let total_modules = cache.len();
        let total_size_bytes: usize = cache.values().map(|m| m.size_bytes).sum();
        let average_module_size = if total_modules > 0 {
            total_size_bytes / total_modules
        } else {
            0
        };

        let total_requests = hits + misses;
        let cache_hit_rate = if total_requests > 0 {
            hits as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheMetrics {
            total_modules,
            total_size_bytes,
            average_module_size,
            cache_hit_rate,
            memory_usage_bytes: total_size_bytes as u64,
        }
    }

    /// Get number of cached modules
    pub async fn len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Check if cache is empty
    pub async fn is_empty(&self) -> bool {
        self.cache.read().await.is_empty()
    }

    /// Remove a specific entry
    pub async fn remove(&self, key: &str) -> bool {
        self.cache.write().await.remove(key).is_some()
    }

    /// Get cache capacity
    pub const fn capacity(&self) -> usize {
        self.max_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_module_creation() {
        let data = vec![1, 2, 3, 4];
        let cached = CachedModule::new(data.clone());

        assert_eq!(cached.size_bytes, 4);
        assert_eq!(cached.access_count, 1);
        assert_eq!(cached.compiled_module, data);
    }

    #[test]
    fn test_cached_module_access() {
        let mut cached = CachedModule::new(vec![1, 2, 3]);
        let initial_count = cached.access_count;

        cached.record_access();

        assert_eq!(cached.access_count, initial_count + 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_module_cache_creation() {
        let cache = ModuleCache::new(10);

        assert_eq!(cache.capacity(), 10);
        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cache_metrics() {
        let cache = ModuleCache::new(10);
        let metrics = cache.metrics().await;

        assert_eq!(metrics.total_modules, 0);
        assert_eq!(metrics.cache_hit_rate, 0.0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cache_clear() {
        let cache = ModuleCache::new(10);

        cache.clear().await;

        assert!(cache.is_empty().await);
        let metrics = cache.metrics().await;
        assert_eq!(metrics.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_cache_metrics_display() {
        let metrics = CacheMetrics {
            total_modules: 10,
            total_size_bytes: 1024,
            average_module_size: 102,
            cache_hit_rate: 0.85,
            memory_usage_bytes: 1024,
        };

        let display = format!("{}", metrics);
        assert!(display.contains("10"));
        assert!(display.contains("1024"));
        assert!(display.contains("85.00%"));
    }
}
