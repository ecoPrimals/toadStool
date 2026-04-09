// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Memory Pool - Efficient buffer reuse
//!
//! Reduces allocation overhead by reusing GPU buffers.
//! OpenCL buffer pooling was removed S198; use barraCuda/coralReef for vendor pools.

use std::sync::Arc;
use tokio::sync::RwLock;

/// Memory pool for GPU buffers
///
/// Maintains pool statistics; native buffer reuse is provided by WebGPU/Vulkan paths.
pub struct MemoryPool {
    _phantom: std::marker::PhantomData<()>,

    /// Statistics
    stats: Arc<RwLock<PoolStatistics>>,
}

/// Memory pool statistics.
#[derive(Debug, Default, Clone)]
pub struct PoolStatistics {
    /// Number of allocations.
    pub allocations: u64,
    /// Number of deallocations.
    pub deallocations: u64,
    /// Cache hits (buffer reused).
    pub cache_hits: u64,
    /// Cache misses (new allocation).
    pub cache_misses: u64,
    /// Total bytes allocated.
    pub total_bytes_allocated: u64,
    /// Total bytes reused from pool.
    pub total_bytes_reused: u64,
}

impl MemoryPool {
    /// Creates a new memory pool with default configuration.
    pub fn new() -> Self {
        Self::with_capacity(16) // 16 size buckets by default
    }

    /// Create a memory pool with specific bucket capacity
    ///
    /// More buckets = finer-grained size matching, less waste
    pub fn with_capacity(_bucket_count: usize) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            stats: Arc::new(RwLock::new(PoolStatistics::default())),
        }
    }

    /// Get pool statistics
    pub async fn statistics(&self) -> PoolStatistics {
        self.stats.read().await.clone()
    }

    /// Clear all buffers from pool
    pub async fn clear(&self) {
        tracing::info!("Memory pool cleared");
    }

    /// Get hit rate as percentage
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // display percentage from counts
    pub async fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().await;
        let total = stats.cache_hits + stats.cache_misses;

        if total == 0 {
            0.0
        } else {
            (stats.cache_hits as f64 / total as f64) * 100.0
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_statistics() {
        let pool = MemoryPool::new();
        let stats = pool.statistics().await;

        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.cache_hits, 0);
    }

    #[tokio::test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // test values are exact literals
    async fn test_hit_rate() {
        let pool = MemoryPool::new();
        assert_eq!(pool.hit_rate().await, 0.0);
    }
}
