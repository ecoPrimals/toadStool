// SPDX-License-Identifier: AGPL-3.0-only
//! Module cache metrics
//!
//! Provides metrics tracking for WASM module caching

/// Module cache metrics
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    pub total_modules: usize,
    pub total_size_bytes: usize,
    pub average_module_size: usize,
    pub cache_hit_rate: f64,
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
