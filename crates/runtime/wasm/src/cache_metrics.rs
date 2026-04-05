// SPDX-License-Identifier: AGPL-3.0-or-later
//! Module cache metrics
//!
//! Provides metrics tracking for WASM module caching

/// Module cache metrics for WASM module caching
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    /// Total number of cached modules
    pub total_modules: usize,
    /// Total byte size of all cached modules
    pub total_size_bytes: usize,
    /// Average module size in bytes
    pub average_module_size: usize,
    /// Cache hit rate (0.0–1.0)
    pub cache_hit_rate: f64,
    /// Memory usage of cache in bytes
    pub memory_usage_bytes: u64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            total_modules: 0,
            total_size_bytes: 0,
            average_module_size: 0,
            cache_hit_rate: 0.0,
            memory_usage_bytes: 0,
        }
    }
}

impl std::fmt::Display for CacheMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CacheMetrics(modules: {}, size: {} bytes, hit_rate: {:.2}%)",
            self.total_modules,
            self.total_size_bytes,
            self.cache_hit_rate * 100.0
        )
    }
}
