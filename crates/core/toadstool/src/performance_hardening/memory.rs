// SPDX-License-Identifier: AGPL-3.0-or-later
//! Memory pool management
//!
//! This module provides memory pool management for object reuse and
//! allocation optimization.

use super::types::{MemoryPoolConfig, PoolStats};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory pool for object reuse
pub struct MemoryPool<T> {
    /// Configuration
    config: MemoryPoolConfig,
    /// Available objects
    available: Arc<RwLock<Vec<T>>>,
    /// Factory function
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    /// Usage statistics
    stats: Arc<RwLock<PoolStats>>,
}

impl<T> MemoryPool<T> {
    /// Create new memory pool
    pub fn new<F>(config: MemoryPoolConfig, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let factory = Arc::new(factory);
        let mut available = Vec::new();

        // Pre-allocate initial objects
        for _ in 0..config.initial_size {
            available.push(factory());
        }

        Self {
            config: config.clone(),
            available: Arc::new(RwLock::new(available)),
            factory,
            stats: Arc::new(RwLock::new(PoolStats {
                current_size: config.initial_size,
                in_use: 0,
                available: config.initial_size,
                total_allocations: 0,
                total_deallocations: 0,
                hit_rate: 0.0,
            })),
        }
    }

    /// Get object from pool
    pub async fn get(&self) -> PooledObject<T>
    where
        T: Send + Sync + 'static,
    {
        let mut available = self.available.write().await;
        let mut stats = self.stats.write().await;

        let object = if let Some(obj) = available.pop() {
            let total = stats.total_allocations;
            #[allow(clippy::cast_precision_loss)]
            let rate = (stats.hit_rate * total as f64 + 1.0) / (total as f64 + 1.0);
            stats.hit_rate = rate;
            obj
        } else {
            // No available objects, create new one
            let new_obj = (self.factory)();
            let total = stats.total_allocations;
            #[allow(clippy::cast_precision_loss)]
            let rate = (stats.hit_rate * total as f64) / (total as f64 + 1.0);
            stats.hit_rate = rate;
            new_obj
        };

        stats.total_allocations += 1;
        stats.in_use = stats.current_size - available.len();
        stats.available = available.len();

        PooledObject {
            object: Some(object),
            pool: Arc::clone(&self.available),
            stats: Arc::clone(&self.stats),
            config: self.config.clone(),
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }
}

/// Pooled object wrapper
pub struct PooledObject<T: Send + Sync + 'static> {
    /// The actual object
    object: Option<T>,
    /// Pool reference
    pool: Arc<RwLock<Vec<T>>>,
    /// Stats reference
    stats: Arc<RwLock<PoolStats>>,
    /// Config reference
    config: MemoryPoolConfig,
}

impl<T: Send + Sync + 'static> PooledObject<T> {
    /// Get reference to the object
    pub fn get(&self) -> Option<&T> {
        self.object.as_ref()
    }

    /// Get mutable reference to the object
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.object.as_mut()
    }
}

impl<T: Send + Sync + 'static> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            // Use try_lock for immediate return without async spawning
            // This is safe: if locks are held, the pool will grow temporarily
            // but will be reclaimed on next successful return
            if let Ok(mut available) = self.pool.try_write() {
                if let Ok(mut stats_guard) = self.stats.try_write() {
                    let max_size = self.config.max_size;

                    if available.len() < max_size {
                        available.push(object);
                        stats_guard.current_size = available.len();
                    }

                    stats_guard.total_deallocations += 1;
                    stats_guard.in_use = stats_guard.current_size.saturating_sub(available.len());
                    stats_guard.available = available.len();
                }
            }
            // If locks are contended, object is dropped and will be recreated
            // This is acceptable for pool efficiency and maintains correctness
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_pool_construction() {
        let config = MemoryPoolConfig {
            initial_size: 5,
            max_size: 100,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, || String::from("obj"));

        let stats = pool.get_stats().await;
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.available, 5);
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.total_deallocations, 0);
    }

    #[tokio::test]
    async fn test_memory_pool_get_release_cycle() {
        let config = MemoryPoolConfig {
            initial_size: 3,
            max_size: 10,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, Vec::<i32>::new);

        let obj1 = pool.get().await;
        assert!(obj1.get().is_some());
        let obj2 = pool.get().await;
        assert!(obj2.get().is_some());

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.in_use, 2);
        assert_eq!(stats.available, 1);

        drop(obj1);
        drop(obj2);

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_deallocations, 2);
    }

    #[tokio::test]
    async fn test_memory_pool_exhaustion_and_reuse() {
        let config = MemoryPoolConfig {
            initial_size: 2,
            max_size: 5,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, || String::from("allocated"));

        let mut objs = Vec::new();
        for _ in 0..5 {
            objs.push(pool.get().await);
        }
        assert_eq!(objs.len(), 5);

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_allocations, 5);

        drop(objs);

        let obj_reuse = pool.get().await;
        assert_eq!(obj_reuse.get().map(|s| s.as_str()), Some("allocated"));

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_deallocations, 5);
    }

    #[tokio::test]
    async fn test_memory_pool_reuse_after_release() {
        let config = MemoryPoolConfig {
            initial_size: 2,
            max_size: 10,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, || 42i32);

        let obj1 = pool.get().await;
        assert_eq!(obj1.get(), Some(&42));
        drop(obj1);

        let obj2 = pool.get().await;
        assert_eq!(obj2.get(), Some(&42));

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_allocations, 2);
    }

    #[tokio::test]
    async fn test_pooled_object_get_mut() {
        let config = MemoryPoolConfig {
            initial_size: 1,
            max_size: 10,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, || vec![1, 2, 3]);

        let mut obj = pool.get().await;
        if let Some(v) = obj.get_mut() {
            v.push(4);
            assert_eq!(v, &[1, 2, 3, 4]);
        }
    }

    #[tokio::test]
    async fn test_pooled_object_drop_automatic_release() {
        let config = MemoryPoolConfig {
            initial_size: 1,
            max_size: 10,
            ..Default::default()
        };
        let pool = MemoryPool::new(config, || String::from("test"));

        {
            let _obj = pool.get().await;
            let stats = pool.get_stats().await;
            assert_eq!(stats.in_use, 1);
        }

        let stats = pool.get_stats().await;
        assert_eq!(stats.total_deallocations, 1);
    }
}
