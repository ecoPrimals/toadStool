// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Memory Management
//!
//! Zero-copy memory management for GPU compute with comprehensive safety
//!
//! ## Architecture
//! - **Pool**: Reuses GPU buffers to reduce allocation overhead
//! - **Pinned**: Host memory pinned for fast GPU transfers
//! - **Zero-Copy**: Ownership transfer without data copying
//! - **Async**: Non-blocking allocation and deallocation
//!
//! ## Performance Benefits
//! - 10-100x faster than allocating on every kernel launch
//! - 2-3x faster transfers with pinned host memory
//! - Zero-copy moves eliminate unnecessary data copying
//! - Async operations prevent blocking

pub mod pinned;
pub mod pool;
pub mod tracker;

pub use pinned::PinnedMemory;
pub use pool::GpuMemoryPool;
pub use tracker::MemoryTracker;

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// Allocate exact size requested
    Exact,
    /// Round up to power of 2 for better pooling
    PowerOfTwo,
    /// Align to cache line boundaries
    CacheAligned { alignment: usize },
}

impl Default for AllocationStrategy {
    fn default() -> Self {
        Self::PowerOfTwo // Best for pooling
    }
}

impl AllocationStrategy {
    /// Calculate allocation size based on strategy
    pub fn allocation_size(&self, requested: usize) -> usize {
        match self {
            Self::Exact => requested,
            Self::PowerOfTwo => requested.next_power_of_two(),
            Self::CacheAligned { alignment } => {
                // Round up to next multiple of alignment
                (requested + alignment - 1) / alignment * alignment
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_strategy_exact() {
        let strategy = AllocationStrategy::Exact;
        assert_eq!(strategy.allocation_size(1000), 1000);
        assert_eq!(strategy.allocation_size(1024), 1024);
        assert_eq!(strategy.allocation_size(2000), 2000);
    }

    #[test]
    fn test_allocation_strategy_power_of_two() {
        let strategy = AllocationStrategy::PowerOfTwo;
        assert_eq!(strategy.allocation_size(1000), 1024);
        assert_eq!(strategy.allocation_size(1024), 1024);
        assert_eq!(strategy.allocation_size(2000), 2048);
    }

    #[test]
    fn test_allocation_strategy_cache_aligned() {
        let strategy = AllocationStrategy::CacheAligned { alignment: 64 };
        assert_eq!(strategy.allocation_size(50), 64);
        assert_eq!(strategy.allocation_size(64), 64);
        assert_eq!(strategy.allocation_size(100), 128);
    }
}

