// SPDX-License-Identifier: AGPL-3.0-only
//! GPU Memory Pool - Efficient buffer reuse
//!
//! Reduces allocation overhead by reusing GPU buffers

use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "opencl")]
use std::collections::VecDeque;
#[cfg(feature = "opencl")]
use toadstool::error::{ToadStoolError, ToadStoolResult};

#[cfg(feature = "opencl")]
use ocl::{Buffer, Queue};

/// Memory pool for GPU buffers
///
/// Maintains a pool of pre-allocated buffers to reduce allocation overhead
pub struct MemoryPool {
    /// Available buffers by size bucket
    #[cfg(feature = "opencl")]
    buffers: Arc<RwLock<Vec<BufferBucket>>>,
    #[cfg(not(feature = "opencl"))]
    _phantom: std::marker::PhantomData<()>,

    /// Statistics
    stats: Arc<RwLock<PoolStatistics>>,
}

#[cfg(feature = "opencl")]
struct BufferBucket {
    size: usize,
    buffers: VecDeque<Buffer<u8>>,
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
    pub fn with_capacity(bucket_count: usize) -> Self {
        #[cfg(feature = "opencl")]
        let buffers = {
            let mut buckets = Vec::with_capacity(bucket_count);
            for i in 0..bucket_count {
                let size = 1024 * (1 << i); // 1KB, 2KB, 4KB, 8KB, ...
                buckets.push(BufferBucket {
                    size,
                    buffers: VecDeque::with_capacity(4), // 4 buffers per bucket
                });
            }
            Arc::new(RwLock::new(buckets))
        };

        #[cfg(not(feature = "opencl"))]
        let _ = bucket_count;

        Self {
            #[cfg(feature = "opencl")]
            buffers,
            #[cfg(not(feature = "opencl"))]
            _phantom: std::marker::PhantomData,
            stats: Arc::new(RwLock::new(PoolStatistics::default())),
        }
    }

    /// Get buffer from pool or allocate new
    #[cfg(feature = "opencl")]
    pub async fn acquire_buffer(&self, size: usize, queue: &Queue) -> ToadStoolResult<Buffer<u8>> {
        // Try to find buffer in pool
        {
            let mut buffers = self.buffers.write().await;

            // Find bucket with this size
            if let Some(bucket) = buffers
                .iter_mut()
                .find(|b| b.size >= size && b.size < size * 2)
                && let Some(buffer) = bucket.buffers.pop_front()
            {
                // Cache hit!
                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                stats.total_bytes_reused += size as u64;
                drop(stats);

                tracing::debug!(
                    "Memory pool cache hit: {} bytes (pool size: {})",
                    size,
                    bucket.buffers.len()
                );

                return Ok(buffer);
            }
        }

        // Cache miss - allocate new buffer
        let mut stats = self.stats.write().await;
        stats.cache_misses += 1;
        stats.allocations += 1;
        stats.total_bytes_allocated += size as u64;
        drop(stats);

        tracing::debug!("Memory pool cache miss: allocating {} bytes", size);

        Buffer::<u8>::builder()
            .queue(queue.clone())
            .len(size)
            .build()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to allocate buffer: {}", e)))
    }

    /// Return buffer to pool for reuse
    #[cfg(feature = "opencl")]
    pub async fn release_buffer(&self, buffer: Buffer<u8>) {
        let size = buffer.len();
        let mut buffers = self.buffers.write().await;

        // Find or create bucket
        let bucket = buffers
            .iter_mut()
            .find(|b| b.size >= size && b.size < size * 2);

        if let Some(bucket) = bucket {
            // Add to existing bucket (limit pool size per bucket)
            if bucket.buffers.len() < 16 {
                bucket.buffers.push_back(buffer);

                let mut stats = self.stats.write().await;
                stats.deallocations += 1;
                drop(stats);

                tracing::debug!(
                    "Buffer returned to pool: {} bytes (pool size: {})",
                    size,
                    bucket.buffers.len()
                );
            } else {
                // Pool full, drop buffer (will be freed)
                tracing::debug!("Buffer pool full for size {}, dropping buffer", size);
            }
        } else {
            // Create new bucket
            buffers.push(BufferBucket {
                size,
                buffers: {
                    let mut queue = VecDeque::new();
                    queue.push_back(buffer);
                    queue
                },
            });

            let mut stats = self.stats.write().await;
            stats.deallocations += 1;
            drop(stats);

            tracing::debug!("New buffer bucket created: {} bytes", size);
        }

        drop(buffers);
    }

    /// Get pool statistics
    pub async fn statistics(&self) -> PoolStatistics {
        self.stats.read().await.clone()
    }

    /// Clear all buffers from pool
    pub async fn clear(&self) {
        #[cfg(feature = "opencl")]
        {
            let mut buffers = self.buffers.write().await;
            buffers.clear();
        }

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
