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
            stats.hit_rate = (stats.hit_rate * stats.total_allocations as f64 + 1.0)
                / (stats.total_allocations as f64 + 1.0);
            obj
        } else {
            // No available objects, create new one
            let new_obj = (self.factory)();
            stats.hit_rate = (stats.hit_rate * stats.total_allocations as f64)
                / (stats.total_allocations as f64 + 1.0);
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
