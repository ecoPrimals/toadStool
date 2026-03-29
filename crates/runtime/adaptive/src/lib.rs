// SPDX-License-Identifier: AGPL-3.0-only
//! Adaptive Optimization System for barraCuda
//!
//! Runtime learning system that automatically optimizes GPU operation configurations
//! for any hardware without manual tuning.
//!
//! ## Architecture
//!
//! - **GPU Fingerprinting**: Uniquely identify hardware for cache lookup
//! - **Runtime Profiler**: Micro-benchmark operations to learn optimal configs
//! - **Optimization Cache**: Persist learned configurations across runs
//! - **Config Selector**: Choose optimal workgroup sizes intelligently
//!
//! ## Deep Debt Principles
//!
//! - ✅ **Pure Rust**: Zero unsafe in application layer
//! - ✅ **Vendor Agnostic**: Works on NVIDIA, AMD, Intel, Apple
//! - ✅ **No Hardcoding**: All configs learned at runtime
//! - ✅ **Self-Knowledge**: System knows only itself, discovers hardware
//! - ✅ **Graceful Fallback**: Conservative defaults if profiling fails
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_runtime_adaptive::AdaptiveExecutor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // First run: profiles GPU (10 seconds)
//! let executor = AdaptiveExecutor::new().await?;
//!
//! // Subsequent runs: uses cached optimal configs (instant!)
//! let result = executor.execute_matmul(/* ... */).await?;
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(clippy::pedantic, clippy::cargo, clippy::nursery)]
// Transitive deps (wgpu, ash, etc.) have multiple versions; we cannot control that.
#![allow(clippy::multiple_crate_versions)]

pub mod cache;
pub mod error;
pub mod fingerprint;
pub mod profiler;
pub mod selector;
pub mod types;

pub use error::AdaptiveError;

pub use cache::{OptimizationCache, WorkgroupConfig};
pub use fingerprint::{GpuFingerprint, GpuVendor};
pub use profiler::{ProfilingConfig, RuntimeProfiler};
pub use selector::{ConfigSelector, FallbackStrategy};
pub use types::{OpType, SizeClass};

use std::sync::Arc;
use tokio::sync::RwLock;

/// Adaptive GPU executor that learns optimal configurations
///
/// Wraps standard GPU executor with adaptive optimization layer.
/// On first run, profiles operations to learn optimal workgroup sizes.
/// Subsequent runs use cached configurations for instant optimization.
///
/// ## Example
///
/// ```rust,ignore
/// # use toadstool_runtime_adaptive::AdaptiveExecutor;
/// # async fn example() -> Result<(), AdaptiveError> {
/// // One-time setup (profiles GPU on first run)
/// let executor = AdaptiveExecutor::new().await?;
///
/// // Operations automatically use optimal workgroup sizes
/// let result = executor.execute_matmul(/* ... */).await?;
/// # Ok(())
/// # }
/// ```
pub struct AdaptiveExecutor {
    fingerprint: GpuFingerprint,
    cache: Arc<RwLock<OptimizationCache>>,
    selector: ConfigSelector,
    profiler: RuntimeProfiler,
}

impl AdaptiveExecutor {
    /// Create new adaptive executor
    ///
    /// On first run, will profile GPU to learn optimal configurations.
    /// Subsequent runs load cached configurations instantly.
    ///
    /// # Errors
    ///
    /// Returns error if GPU initialization fails or profiling encounters issues.
    pub async fn new() -> Result<Self, AdaptiveError> {
        // Discover GPU hardware (no hardcoding!)
        let fingerprint = GpuFingerprint::discover().await?;

        tracing::info!(
            "Discovered GPU: {} {} ({})",
            fingerprint.vendor,
            fingerprint.model_class,
            fingerprint.architecture
        );

        // Load or create cache
        let cache = OptimizationCache::load_or_create(&fingerprint)?;
        let cache = Arc::new(RwLock::new(cache));

        // Create profiler for runtime benchmarking
        let profiler = RuntimeProfiler::new(fingerprint.clone())?;

        // Create selector with cache
        let selector = ConfigSelector::new(Arc::clone(&cache), FallbackStrategy::Conservative);

        // Check if we need to profile
        let needs_profiling = {
            let cache_read = cache.read().await;
            cache_read.is_empty()
        };

        if needs_profiling {
            tracing::info!("First run detected - profiling GPU (10 seconds)...");
            Self::quick_profile(&profiler, &cache).await?;
            tracing::info!("Profiling complete! Configurations cached for future runs.");
        } else {
            tracing::info!("Using cached optimization configurations (instant!)");
        }

        Ok(Self {
            fingerprint,
            cache,
            selector,
            profiler,
        })
    }

    /// Quick profile common operations
    ///
    /// Profiles core operations (`MatMul`, `LayerNorm`, etc.) to establish baseline.
    /// Takes ~10 seconds on first run, results cached for future runs.
    async fn quick_profile(
        profiler: &RuntimeProfiler,
        cache: &Arc<RwLock<OptimizationCache>>,
    ) -> Result<(), AdaptiveError> {
        // Profile core operations with common size classes
        let operations = vec![
            OpType::MatMul,
            OpType::LayerNorm,
            OpType::GELU,
            OpType::Softmax,
            OpType::Add,
        ];

        let size_classes = vec![SizeClass::Small, SizeClass::Medium, SizeClass::Large];

        let workgroup_candidates = vec![32, 64, 128, 256];

        for op_type in operations {
            tracing::debug!("Profiling {:?}...", op_type);

            let profile =
                profiler.profile_operation(op_type, &size_classes, &workgroup_candidates)?;

            // Update cache with results
            let mut cache_write = cache.write().await;
            cache_write.add_profile(profile);
        }

        // Save cache to disk
        {
            let cache_read = cache.read().await;
            cache_read.save()?;
        }

        Ok(())
    }

    /// Get optimal workgroup size for operation
    ///
    /// Returns optimal workgroup size based on cached configurations
    /// or conservative default if not yet profiled.
    #[must_use]
    pub fn optimal_workgroup(&self, op_type: OpType, input_size: usize) -> usize {
        self.selector.select_workgroup(op_type, input_size)
    }

    /// Get GPU fingerprint
    #[must_use]
    pub const fn fingerprint(&self) -> &GpuFingerprint {
        &self.fingerprint
    }

    /// Force re-profile all operations
    ///
    /// Useful after driver updates or significant system changes.
    ///
    /// # Errors
    ///
    /// Returns error if profiling fails.
    pub async fn force_reprofile(&self) -> Result<(), AdaptiveError> {
        tracing::info!("Re-profiling GPU...");
        Self::quick_profile(&self.profiler, &self.cache).await?;
        tracing::info!("Re-profiling complete!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adaptive_executor_creation() {
        // Should not panic - graceful fallback if GPU not available
        let result = AdaptiveExecutor::new().await;

        if let Ok(executor) = result {
            // Verify fingerprint is populated
            assert!(!executor.fingerprint().architecture.is_empty());

            // Verify can get optimal workgroup
            let workgroup = executor.optimal_workgroup(OpType::MatMul, 1024);
            assert!(workgroup >= 32);
            assert!(workgroup <= 1024);
        } else {
            // OK if no GPU available in test environment
            eprintln!("Note: No GPU available for testing");
        }
    }

    #[test]
    fn test_fallback_strategy_workgroup_sizes() {
        use crate::selector::FallbackStrategy;

        // Conservative fallback returns safe defaults
        let wg_small = FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 64);
        let wg_medium = FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 5000);
        let wg_large = FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 200_000);

        assert_eq!(wg_small, 64);
        assert_eq!(wg_medium, 128);
        assert_eq!(wg_large, 256);

        // Aggressive fallback returns higher values
        let wg_aggressive = FallbackStrategy::Aggressive.fallback_workgroup(OpType::MatMul, 5000);
        assert_eq!(wg_aggressive, 256);
    }

    #[test]
    fn test_fallback_strategy_vendor_hint() {
        use crate::selector::FallbackStrategy;

        let wg_matmul = FallbackStrategy::VendorHint.fallback_workgroup(OpType::MatMul, 1000);
        let wg_conv = FallbackStrategy::VendorHint.fallback_workgroup(OpType::Conv2D, 1000);
        let wg_add = FallbackStrategy::VendorHint.fallback_workgroup(OpType::Add, 1000);

        assert_eq!(wg_matmul, 256);
        assert_eq!(wg_conv, 256);
        assert_eq!(wg_add, 128);
    }

    #[test]
    fn test_fallback_strategy_conservative_boundaries() {
        use crate::selector::FallbackStrategy;

        assert_eq!(
            FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 999),
            64
        );
        assert_eq!(
            FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 1000),
            128
        );
        assert_eq!(
            FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 99_999),
            128
        );
        assert_eq!(
            FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 100_000),
            256
        );
    }

    #[test]
    fn test_fallback_strategy_aggressive_boundaries() {
        use crate::selector::FallbackStrategy;

        assert_eq!(
            FallbackStrategy::Aggressive.fallback_workgroup(OpType::Add, 500),
            128
        );
        assert_eq!(
            FallbackStrategy::Aggressive.fallback_workgroup(OpType::Add, 50_000),
            256
        );
        assert_eq!(
            FallbackStrategy::Aggressive.fallback_workgroup(OpType::Add, 500_000),
            512
        );
    }

    #[test]
    fn test_op_type_variants() {
        let _ = OpType::MatMul;
        let _ = OpType::LayerNorm;
        let _ = OpType::GELU;
        let _ = OpType::Softmax;
        let _ = OpType::Add;
        let _ = OpType::Conv2D;
    }

    #[test]
    fn test_size_class_variants() {
        let _ = SizeClass::Tiny;
        let _ = SizeClass::Small;
        let _ = SizeClass::Medium;
        let _ = SizeClass::Large;
        let _ = SizeClass::Huge;
    }

    #[test]
    fn test_size_class_from_size() {
        assert_eq!(SizeClass::from_size(500), SizeClass::Tiny);
        assert_eq!(SizeClass::from_size(5_000), SizeClass::Small);
        assert_eq!(SizeClass::from_size(500_000), SizeClass::Medium);
    }

    #[test]
    fn test_size_class_representative_size() {
        assert_eq!(SizeClass::Small.representative_size(), 10_000);
        assert_eq!(SizeClass::Large.representative_size(), 5_000_000);
    }

    #[test]
    fn test_size_class_all_variants_representative_size() {
        let _ = SizeClass::Tiny.representative_size();
        let _ = SizeClass::Medium.representative_size();
        let _ = SizeClass::Huge.representative_size();
    }

    #[test]
    fn test_size_class_from_size_boundaries() {
        assert_eq!(SizeClass::from_size(0), SizeClass::Tiny);
        assert_eq!(SizeClass::from_size(1_000), SizeClass::Small);
        assert_eq!(SizeClass::from_size(100_000), SizeClass::Medium);
        assert_eq!(SizeClass::from_size(5_000_000), SizeClass::Large);
        assert_eq!(SizeClass::from_size(10_000_000), SizeClass::Huge);
        assert_eq!(SizeClass::from_size(100_000_000), SizeClass::Huge);
    }

    #[test]
    fn test_fallback_strategy_all_op_types() {
        use crate::selector::FallbackStrategy;

        for op in [
            OpType::MatMul,
            OpType::LayerNorm,
            OpType::GELU,
            OpType::Softmax,
            OpType::Add,
            OpType::Conv2D,
        ] {
            let _ = FallbackStrategy::Conservative.fallback_workgroup(op, 1000);
            let _ = FallbackStrategy::Aggressive.fallback_workgroup(op, 1000);
            let _ = FallbackStrategy::VendorHint.fallback_workgroup(op, 1000);
        }
    }

    #[test]
    fn test_fallback_strategy_zero_size() {
        use crate::selector::FallbackStrategy;

        let wg = FallbackStrategy::Conservative.fallback_workgroup(OpType::MatMul, 0);
        assert!(wg >= 32);
        assert!(wg <= 1024);
    }

    #[test]
    fn test_op_type_debug() {
        let s = format!("{:?}", OpType::MatMul);
        assert!(s.contains("MatMul"));
    }

    #[test]
    fn test_size_class_debug() {
        let s = format!("{:?}", SizeClass::Small);
        assert!(s.contains("Small"));
    }

    #[test]
    fn test_adaptive_error_other_display() {
        use crate::AdaptiveError;
        let err = AdaptiveError::Other("test error".to_string());
        let s = err.to_string();
        assert!(s.contains("test error"));
    }

    #[test]
    fn test_config_selector_new() {
        use crate::cache::OptimizationCache;
        use crate::fingerprint::{GpuFingerprint, GpuVendor};
        use crate::selector::{ConfigSelector, FallbackStrategy};
        use std::sync::Arc;

        let gpu = GpuFingerprint {
            vendor: GpuVendor::Unknown,
            architecture: "test".to_string(),
            model_class: "test".to_string(),
            driver_version: "0.0".to_string(),
            backend: "test".to_string(),
            memory_size_gb: 0,
        };
        let cache = Arc::new(tokio::sync::RwLock::new(OptimizationCache::new(gpu)));
        let selector = ConfigSelector::new(cache, FallbackStrategy::Conservative);
        let wg = selector.select_workgroup(OpType::MatMul, 1024);
        assert!(wg >= 32);
        assert!(wg <= 1024);
    }
}
