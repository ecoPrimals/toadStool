// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # ToadStool WebAssembly Runtime Engine
//!
//! High-performance WebAssembly runtime engine with comprehensive features:
//!
//! ## Features
//!
//! - **Wasmtime Integration**: Production-ready WASM execution
//! - **Module Caching**: LRU cache with configurable size and TTL
//! - **WASI Support**: Full WASI compatibility with security controls
//! - **Security Isolation**: Multiple security levels (None, Basic, Strict, Maximum)
//! - **Component Model**: WebAssembly Component Model support
//! - **Metrics & Monitoring**: Comprehensive execution metrics
//! - **Thread-Safe**: Arc<RwLock<>> for safe concurrent access
//! - **Async-First**: Built on Tokio for high performance
//!
//! ## Architecture
//!
//! This crate is organized into focused modules following modern Rust best practices:
//!
//! - **`config`**: Configuration types and builders
//! - **`cache`**: Thread-safe module caching with LRU eviction
//! - **`metrics`**: Execution tracking and performance monitoring
//! - **`execution`**: Module loading and execution orchestration
//! - **`engine`**: Main runtime engine implementation
//! - **`component_model`**: WebAssembly Component Model support
//!
//! ## Example Usage
//!
//! ```no_run
//! use toadstool_runtime_wasm::{WasmRuntimeEngine, WasmRuntimeConfig, SecurityLevel};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create configuration with builder pattern
//! let config = WasmRuntimeConfig::builder()
//!     .max_memory_mb(256)
//!     .security_level(SecurityLevel::Strict)
//!     .execution_timeout_ms(30000)
//!     .build();
//!
//! // Create runtime engine
//! let engine = WasmRuntimeEngine::new(config)?;
//!
//! // Use with RuntimeEngine trait
//! // ... execute WASM workloads
//! # Ok(())
//! # }
//! ```
//!
//! ## Safety
//!
//! This crate contains **zero unsafe code** and maintains Rust's safety guarantees.
//! All WebAssembly execution happens within Wasmtime's safe sandbox.

// Module declarations
pub mod cache;
pub mod component_model;
pub mod config;
pub mod engine;
pub mod execution;
pub mod metrics;

// Re-export primary types for convenience
pub use cache::{CacheMetrics, CachedModule, ModuleCache};
pub use component_model::{
    ComponentInterface, ComponentModelConfig, ComponentModelSupport, ComponentRegistry,
};
pub use config::{SecurityLevel, WasmRuntimeConfig, WasmRuntimeConfigBuilder};
pub use engine::WasmRuntimeEngine;
pub use execution::{ModuleExecutor, ModuleLoader, WasiContextBuilder};
pub use metrics::{ExecutionHandle, MetricsCollector, ResourceUsage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _config: WasmRuntimeConfig;
        let _security: SecurityLevel;
        let _metrics: CacheMetrics;
    }

    #[test]
    fn test_security_levels() {
        assert_eq!(SecurityLevel::default(), SecurityLevel::Strict);
        assert!(SecurityLevel::Strict.enforces_memory_limits());
        assert!(SecurityLevel::Maximum.requires_fuel_tracking());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_creation() {
        let config = WasmRuntimeConfig::default();
        let result = WasmRuntimeEngine::new(config);
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_config_builder() {
        let config = WasmRuntimeConfig::builder()
            .max_memory_mb(512)
            .security_level(SecurityLevel::Maximum)
            .execution_timeout_ms(60000)
            .build();

        assert_eq!(config.max_memory_mb, 512);
        assert_eq!(config.security_level, SecurityLevel::Maximum);
        assert_eq!(config.execution_timeout_ms, 60000);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_cache_creation() {
        let cache = ModuleCache::new(100);
        assert_eq!(cache.capacity(), 100);
        assert!(cache.is_empty().await);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.active_count().await, 0);
        assert_eq!(collector.total_count().await, 0);
        assert_eq!(collector.success_rate().await, 1.0);
    }
}

