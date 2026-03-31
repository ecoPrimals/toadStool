// SPDX-License-Identifier: AGPL-3.0-only
//! Intelligent caching with LRU and TTL
//!
//! This module provides intelligent caching with least-recently-used (LRU) eviction
//! and time-to-live (TTL) expiration.

use super::types::{CacheStats, CachingConfig};
use crate::ToadStoolResult;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

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
                drop(entries);
                drop(stats);
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
                drop(access_order);

                stats.hits += 1;
                let hits = stats.hits;
                let total = stats.hits + stats.misses;
                #[allow(clippy::cast_precision_loss)]
                let rate = hits as f64 / total as f64;
                stats.hit_rate = rate;

                let value = entry.value.clone();
                drop(entries);
                drop(stats);
                Some(value)
            }
        } else {
            stats.misses += 1;
            let hits = stats.hits;
            let total = stats.hits + stats.misses;
            #[allow(clippy::cast_precision_loss)]
            let rate = hits as f64 / total as f64;
            stats.hit_rate = rate;
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
        drop(entries);
        drop(access_order);
        drop(stats);

        Ok(())
    }

    /// Evict least recently used entry
    #[allow(clippy::unused_async)] // May have await in future eviction logic
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
    #[allow(clippy::unused_async)] // Spawns background task; async for API consistency
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_construction() {
        let config = CachingConfig::default();
        let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

        let stats = cache.get_stats().await;
        assert_eq!(stats.current_size, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.evictions, 0);
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let config = CachingConfig::default();
        let cache = IntelligentCache::new(config);

        let _ = cache.put("key1".to_string(), 42).await;
        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, Some(42));

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.current_size, 1);
    }

    #[tokio::test]
    async fn test_cache_miss_behavior() {
        let config = CachingConfig::default();
        let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

        let value = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(value, None);

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let config = CachingConfig {
            default_ttl: Duration::from_secs(300),
            ..Default::default()
        };
        let cache = IntelligentCache::new(config);

        let _ = cache
            .put_with_ttl("key1".to_string(), 42, Duration::from_nanos(1))
            .await;

        let value = cache.get(&"key1".to_string()).await;
        assert_eq!(value, None);

        let stats = cache.get_stats().await;
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_capacity_lru_eviction() {
        let config = CachingConfig {
            max_size: 3,
            ..Default::default()
        };
        let cache = IntelligentCache::new(config);

        let _ = cache.put("k1".to_string(), 1).await;
        let _ = cache.put("k2".to_string(), 2).await;
        let _ = cache.put("k3".to_string(), 3).await;

        assert_eq!(cache.get(&"k1".to_string()).await, Some(1));
        assert_eq!(cache.get(&"k2".to_string()).await, Some(2));
        assert_eq!(cache.get(&"k3".to_string()).await, Some(3));

        let _ = cache.put("k4".to_string(), 4).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.current_size, 3);

        assert_eq!(cache.get(&"k1".to_string()).await, None);
        assert_eq!(cache.get(&"k2".to_string()).await, Some(2));
        assert_eq!(cache.get(&"k3".to_string()).await, Some(3));
        assert_eq!(cache.get(&"k4".to_string()).await, Some(4));
    }

    #[tokio::test]
    async fn test_cache_put_with_default_ttl() {
        let config = CachingConfig {
            default_ttl: Duration::from_secs(60),
            ..Default::default()
        };
        let cache = IntelligentCache::new(config);

        let _ = cache.put("key".to_string(), 100).await;
        let value = cache.get(&"key".to_string()).await;
        assert_eq!(value, Some(100));
    }

    #[tokio::test]
    async fn test_cache_hit_rate_calculation() {
        let config = CachingConfig::default();
        let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

        let _ = cache.put("key".to_string(), 1).await;
        let _ = cache.get(&"key".to_string()).await;
        let _ = cache.get(&"key".to_string()).await;
        let _ = cache.get(&"missing".to_string()).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 1e-10);
    }

    #[tokio::test]
    async fn test_cache_multiple_evictions() {
        let config = CachingConfig {
            max_size: 2,
            ..Default::default()
        };
        let cache = IntelligentCache::new(config);

        let _ = cache.put("a".to_string(), 1).await;
        let _ = cache.put("b".to_string(), 2).await;
        let _ = cache.put("c".to_string(), 3).await;
        let _ = cache.put("d".to_string(), 4).await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.evictions, 2);
        assert_eq!(stats.current_size, 2);
    }
}
