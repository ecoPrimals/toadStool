//! Intelligent caching with LRU and TTL
//!
//! This module provides intelligent caching with least-recently-used (LRU) eviction
//! and time-to-live (TTL) expiration.

use super::types::{CacheStats, CachingConfig};
use crate::ToadStoolResult;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Intelligent cache
pub struct IntelligentCache<K, V> {
    /// Configuration
    config: CachingConfig,
    /// Cache entries
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    /// Access order for LRU
    access_order: Arc<RwLock<VecDeque<K>>>,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    /// Value
    value: V,
    /// Expiry time
    expires_at: Instant,
    /// Access count
    access_count: u64,
    /// Last accessed
    last_accessed: Instant,
}

impl<K, V> IntelligentCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create new intelligent cache
    #[must_use]
    pub fn new(config: CachingConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(CacheStats {
                current_size: 0,
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
                evictions: 0,
            })),
        }
    }

    /// Get value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            // Check if expired
            if now > entry.expires_at {
                entries.remove(key);
                stats.misses += 1;
                None
            } else {
                // Update access info
                entry.access_count += 1;
                entry.last_accessed = now;

                // Update access order
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == key) {
                    access_order.remove(pos);
                }
                access_order.push_back(key.clone());

                stats.hits += 1;
                stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;

                Some(entry.value.clone())
            }
        } else {
            stats.misses += 1;
            stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;
            None
        }
    }

    /// Put value in cache
    pub async fn put(&self, key: K, value: V) -> ToadStoolResult<()> {
        self.put_with_ttl(key, value, self.config.default_ttl).await
    }

    /// Put value in cache with custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) -> ToadStoolResult<()> {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;
        let mut stats = self.stats.write().await;
        let now = Instant::now();

        // Check if we need to evict
        if entries.len() >= self.config.max_size {
            self.evict_lru(&mut entries, &mut access_order, &mut stats)
                .await;
        }

        // Add new entry
        entries.insert(
            key.clone(),
            CacheEntry {
                value,
                expires_at: now + ttl,
                access_count: 1,
                last_accessed: now,
            },
        );

        access_order.push_back(key);
        stats.current_size = entries.len();

        Ok(())
    }

    /// Evict least recently used entry
    async fn evict_lru(
        &self,
        entries: &mut HashMap<K, CacheEntry<V>>,
        access_order: &mut VecDeque<K>,
        stats: &mut CacheStats,
    ) {
        while let Some(key) = access_order.pop_front() {
            if entries.remove(&key).is_some() {
                stats.evictions += 1;
                break;
            }
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Start cleanup task
    pub async fn start_cleanup_task(&self) {
        let entries = Arc::clone(&self.entries);
        let access_order = Arc::clone(&self.access_order);
        let stats = Arc::clone(&self.stats);
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let mut entries = entries.write().await;
                let mut access_order = access_order.write().await;
                let mut stats = stats.write().await;
                let now = Instant::now();

                // Remove expired entries
                entries.retain(|key, entry| {
                    let expired = now > entry.expires_at;
                    if expired {
                        // Remove from access order
                        if let Some(pos) = access_order.iter().position(|k| k == key) {
                            access_order.remove(pos);
                        }
                    }
                    !expired
                });

                stats.current_size = entries.len();
            }
        });
    }
}
