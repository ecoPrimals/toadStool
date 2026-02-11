//! Configuration selector for optimal workgroup size selection

use crate::cache::OptimizationCache;
use crate::types::OpType;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fallback strategy when no cached config available
#[derive(Debug, Clone, Copy)]
pub enum FallbackStrategy {
    /// Conservative defaults (safe but potentially slower)
    Conservative,
    /// Aggressive defaults (higher performance, higher risk)
    Aggressive,
    /// Vendor-based hints (use vendor-specific defaults)
    VendorHint,
}

impl FallbackStrategy {
    /// Get fallback workgroup size
    #[must_use]
    pub const fn fallback_workgroup(self, op_type: OpType, size: usize) -> usize {
        match self {
            Self::Conservative => {
                // Safe defaults work everywhere
                if size < 1000 {
                    64
                } else if size < 100_000 {
                    128
                } else {
                    256
                }
            }
            Self::Aggressive => {
                // Higher performance defaults
                if size < 1000 {
                    128
                } else if size < 100_000 {
                    256
                } else {
                    512
                }
            }
            Self::VendorHint => {
                // Vendor-agnostic conservative defaults
                // Real vendor hints would be learned, not hardcoded!
                match op_type {
                    OpType::MatMul | OpType::Conv2D => 256,
                    _ => 128,
                }
            }
        }
    }
}

/// Configuration selector
///
/// Chooses optimal workgroup size for operation based on:
/// 1. Cached learned configurations (preferred)
/// 2. Fallback strategy if no cache
///
/// Deep Debt compliant:
/// - No hardcoded vendor optimizations
/// - Graceful fallback if cache miss
/// - All decisions data-driven or conservative
pub struct ConfigSelector {
    cache: Arc<RwLock<OptimizationCache>>,
    fallback_strategy: FallbackStrategy,
}

impl ConfigSelector {
    /// Create new config selector
    #[must_use]
    pub const fn new(
        cache: Arc<RwLock<OptimizationCache>>,
        fallback_strategy: FallbackStrategy,
    ) -> Self {
        Self {
            cache,
            fallback_strategy,
        }
    }

    /// Select optimal workgroup size
    ///
    /// Returns best workgroup size from cache, or fallback if unavailable.
    #[must_use]
    pub fn select_workgroup(&self, op_type: OpType, input_size: usize) -> usize {
        // Try cache first (non-blocking read)
        if let Ok(cache_guard) = self.cache.try_read() {
            if let Some(config) = cache_guard.get_optimal(op_type, input_size) {
                tracing::trace!(
                    "Using cached workgroup {} for {:?} (size: {})",
                    config.workgroup_size,
                    op_type,
                    input_size
                );
                return config.workgroup_size;
            }
        }

        // Fallback to strategy
        let fallback = self
            .fallback_strategy
            .fallback_workgroup(op_type, input_size);
        tracing::trace!(
            "Using fallback workgroup {} for {:?} (size: {})",
            fallback,
            op_type,
            input_size
        );
        fallback
    }

    /// Get selection with metadata
    #[must_use]
    pub fn select_with_metadata(&self, op_type: OpType, input_size: usize) -> WorkgroupSelection {
        if let Ok(cache_guard) = self.cache.try_read() {
            if let Some(config) = cache_guard.get_optimal(op_type, input_size) {
                return WorkgroupSelection {
                    workgroup_size: config.workgroup_size,
                    source: SelectionSource::LocalCache,
                    confidence: config.confidence,
                };
            }
        }

        let fallback = self
            .fallback_strategy
            .fallback_workgroup(op_type, input_size);
        WorkgroupSelection {
            workgroup_size: fallback,
            source: SelectionSource::Fallback,
            confidence: 0.5, // Low confidence for fallback
        }
    }
}

/// Workgroup selection result with metadata
#[derive(Debug, Clone)]
pub struct WorkgroupSelection {
    /// Selected workgroup size
    pub workgroup_size: usize,
    /// Source of selection
    pub source: SelectionSource,
    /// Confidence in selection (0.0 - 1.0)
    pub confidence: f32,
}

/// Source of workgroup selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    /// From local optimization cache
    LocalCache,
    /// From global knowledge base (future)
    GlobalCache,
    /// From quick profiling
    QuickProfile,
    /// Conservative fallback
    Fallback,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::{GpuFingerprint, GpuVendor};

    fn mock_fingerprint() -> GpuFingerprint {
        GpuFingerprint {
            vendor: GpuVendor::NVIDIA,
            architecture: "Ampere".to_string(),
            model_class: "high_end".to_string(),
            driver_version: "1.0".to_string(),
            backend: "Vulkan".to_string(),
            memory_size_gb: 24,
        }
    }

    #[test]
    fn test_fallback_strategy() {
        let conservative = FallbackStrategy::Conservative;
        let aggressive = FallbackStrategy::Aggressive;

        // Conservative is smaller
        assert!(
            conservative.fallback_workgroup(OpType::MatMul, 1000)
                < aggressive.fallback_workgroup(OpType::MatMul, 1000)
        );
    }

    #[test]
    fn test_config_selector_fallback() {
        let gpu = mock_fingerprint();
        let cache = Arc::new(RwLock::new(OptimizationCache::new(gpu)));

        let selector = ConfigSelector::new(cache, FallbackStrategy::Conservative);

        // Empty cache should return fallback
        let workgroup = selector.select_workgroup(OpType::MatMul, 10_000);
        assert!(workgroup >= 32);
        assert!(workgroup <= 512);
    }

    #[tokio::test]
    async fn test_config_selector_cached() {
        let gpu = mock_fingerprint();
        let cache = Arc::new(RwLock::new(OptimizationCache::new(gpu)));

        // Add cached config
        {
            let mut cache_guard = cache.write().await;
            cache_guard.update_measurement(OpType::MatMul, 10_000, 256, 1000.0);
        }

        let selector = ConfigSelector::new(Arc::clone(&cache), FallbackStrategy::Conservative);

        // Should return cached value
        let workgroup = selector.select_workgroup(OpType::MatMul, 10_000);
        assert_eq!(workgroup, 256);
    }

    #[tokio::test]
    async fn test_selection_with_metadata() {
        let gpu = mock_fingerprint();
        let cache = Arc::new(RwLock::new(OptimizationCache::new(gpu)));

        // Add cached config
        {
            let mut cache_guard = cache.write().await;
            cache_guard.update_measurement(OpType::MatMul, 10_000, 256, 1000.0);
        }

        let selector = ConfigSelector::new(Arc::clone(&cache), FallbackStrategy::Conservative);

        let selection = selector.select_with_metadata(OpType::MatMul, 10_000);
        assert_eq!(selection.workgroup_size, 256);
        assert_eq!(selection.source, SelectionSource::LocalCache);
        assert!(selection.confidence > 0.7);
    }
}

// Note: OptimizationCache::new is already available via pub(crate) in cache.rs
