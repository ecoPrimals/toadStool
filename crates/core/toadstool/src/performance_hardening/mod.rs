// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Performance Hardening Module
//!
//! This module provides performance optimization features for `ToadStool`:
//! - Optimized resource monitoring with configurable sampling
//! - Memory pool management and allocation optimization
//! - Intelligent caching and memoization
//! - Async operation optimization and batching
//! - Connection pooling and resource reuse
//! - Performance metrics and profiling
//!
//! ## Organization
//!
//! The module is organized by logical resource domains:
//! - `types`: All configuration and statistics types
//! - `monitoring`: Resource monitoring and metrics collection
//! - `memory`: Memory pool management
//! - `caching`: Intelligent caching with LRU and TTL
//! - `async_ops`: Async operation batching and optimization
//!
//! This organization follows Deep Debt principles of smart refactoring by domain
//! rather than arbitrary line count limits.

// Module declarations
pub mod async_ops;
pub mod caching;
pub mod memory;
pub mod monitoring;
pub mod types;

// Re-exports for public API
pub use async_ops::AsyncBatcher;
pub use caching::IntelligentCache;
pub use memory::{MemoryPool, PooledObject};
pub use monitoring::OptimizedResourceMonitor;
pub use types::*;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{ToadStoolError, ToadStoolResult};

/// Performance hardening manager
///
/// Central manager for all performance hardening features, providing
/// unified configuration and lifecycle management.
pub struct PerformanceHardeningManager {
    /// Configuration
    config: PerformanceHardeningConfig,
    /// Optimized resource monitor
    resource_monitor: Arc<OptimizedResourceMonitor>,
    /// Memory pools (type-erased for storage)
    memory_pools: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
    /// Intelligent caches (type-erased for storage)
    caches: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
}

impl PerformanceHardeningManager {
    /// Create new performance hardening manager
    #[must_use]
    pub fn new(config: PerformanceHardeningConfig) -> Self {
        let resource_monitor = Arc::new(OptimizedResourceMonitor::new(
            config.monitoring_config.clone(),
        ));

        Self {
            config,
            resource_monitor,
            memory_pools: Arc::new(RwLock::new(HashMap::new())),
            caches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize performance hardening
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        info!("Initializing performance hardening");

        // Start monitoring
        if self.config.enable_optimized_monitoring {
            self.start_monitoring_task().await;
        }

        info!("Performance hardening initialized");
        Ok(())
    }

    /// Start monitoring task
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Spawns background task; async for API consistency
    async fn start_monitoring_task(&self) {
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let base_interval = self.config.monitoring_config.base_sampling_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(base_interval);

            loop {
                interval.tick().await;

                // Get current sampling interval
                let current_interval = resource_monitor.get_sampling_interval().await;

                // Adjust interval if needed
                if current_interval != base_interval {
                    interval = tokio::time::interval(current_interval);
                }

                // Future enhancement: Collect actual metrics and add samples
                // This would integrate with the real resource monitoring system
                // Current implementation provides basic performance monitoring
            }
        });
    }

    /// Get resource monitor
    #[must_use]
    pub fn get_resource_monitor(&self) -> Arc<OptimizedResourceMonitor> {
        Arc::clone(&self.resource_monitor)
    }

    /// Create memory pool
    ///
    /// # Errors
    ///
    /// Returns error if memory pools are disabled in configuration.
    pub async fn create_memory_pool<T, F>(
        &self,
        name: &str,
        factory: F,
    ) -> ToadStoolResult<Arc<MemoryPool<T>>>
    where
        T: Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        if !self.config.enable_memory_pools {
            return Err(ToadStoolError::runtime(
                "Memory pools are disabled".to_string(),
            ));
        }

        let pool = Arc::new(MemoryPool::new(
            self.config.memory_pool_config.clone(),
            factory,
        ));

        self.memory_pools
            .write()
            .await
            .insert(name.to_string(), pool.clone());

        Ok(pool)
    }

    /// Create intelligent cache
    ///
    /// # Errors
    ///
    /// Returns error if caching is disabled in configuration.
    pub async fn create_cache<K, V>(
        &self,
        name: &str,
    ) -> ToadStoolResult<Arc<IntelligentCache<K, V>>>
    where
        K: Hash + Eq + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        if !self.config.enable_caching {
            return Err(ToadStoolError::runtime("Caching is disabled".to_string()));
        }

        let cache = Arc::new(IntelligentCache::new(self.config.caching_config.clone()));
        cache.start_cleanup_task().await;

        self.caches
            .write()
            .await
            .insert(name.to_string(), cache.clone());

        Ok(cache)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
