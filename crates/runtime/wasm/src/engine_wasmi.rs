//! WebAssembly Runtime Engine - Wasmi Implementation
//!
//! 100% Pure Rust WASM runtime using wasmi interpreter.
//! 
//! **Evolution** (Jan 17, 2026):
//! - OLD: wasmtime (JIT with C dependencies)
//! - NEW: wasmi 1.0 (100% Pure Rust interpreter)
//!
//! **Benefits**:
//! - ✅ ZERO C dependencies
//! - ✅ Trivial ARM cross-compilation
//! - ✅ Instant startup (no JIT)
//! - ✅ Lower memory usage
//! - ✅ Better security
//!
//! **Perfect for ToadStool's short-lived WASM workloads!**

use std::sync::Arc;
use tracing::{info, debug};
use wasmi::{Config, Engine, Linker, Module, Store};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionRequest, ExecutionResponse, RuntimeEngine};
use toadstool::resources::RuntimeMetrics;

use crate::config::WasmRuntimeConfig;
use crate::metrics::MetricsCollector;

/// WebAssembly Runtime Engine (wasmi-based)
///
/// Pure Rust WASM interpreter for ToadStool's short-lived workloads.
pub struct WasmRuntimeEngine {
    /// Wasmi engine (thread-safe)
    engine: Engine,
    
    /// Runtime configuration
    config: Arc<WasmRuntimeConfig>,
    
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
}

impl std::fmt::Debug for WasmRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeEngine")
            .field("config", &self.config)
            .field("engine", &"<wasmi::Engine>")
            .field("metrics", &"<MetricsCollector>")
            .finish()
    }
}

impl WasmRuntimeEngine {
    /// Create a new WebAssembly runtime engine
    pub fn new(config: WasmRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Creating WebAssembly runtime engine (wasmi 1.0 - Pure Rust!)");

        // Validate configuration
        config.validate().map_err(ToadStoolError::configuration)?;

        // Create wasmi engine with configuration
        let engine = Self::create_wasmi_engine(&config)?;

        // Initialize components
        let metrics = Arc::new(MetricsCollector::new());
        let config = Arc::new(config);

        Ok(Self {
            engine,
            config,
            metrics,
        })
    }

    /// Create wasmi engine with appropriate configuration
    fn create_wasmi_engine(config: &WasmRuntimeConfig) -> ToadStoolResult<Engine> {
        debug!("Creating wasmi engine with Pure Rust interpreter");
        
        // wasmi 1.0 Config is much simpler than wasmtime!
        let mut wasmi_config = Config::default();
        
        // Enable fuel metering if configured
        if config.fuel_limit.is_some() {
            wasmi_config.consume_fuel(true);
        }
        
        // wasmi 1.0 supports multi-memory by default!
        // No need for explicit configuration like wasmtime
        
        let engine = Engine::new(&wasmi_config);
        
        info!("✅ Wasmi engine created - 100% Pure Rust interpreter ready!");
        
        Ok(engine)
    }
    
    /// Get engine reference
    pub fn engine(&self) -> &Engine {
        &self.engine
    }
    
    /// Get configuration
    pub fn config(&self) -> &WasmRuntimeConfig {
        &self.config
    }
}

#[async_trait::async_trait]
impl RuntimeEngine for WasmRuntimeEngine {
    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        // TODO: Implement full execution logic
        // For now, return not implemented
        Err(ToadStoolError::not_supported(
            "Wasmi execution implementation in progress".to_string(),
        ))
    }

    async fn health_check(&self) -> ToadStoolResult<bool> {
        // Engine is always healthy if created successfully
        Ok(true)
    }

    async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        // Return basic metrics for now
        Ok(RuntimeMetrics {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time_ms: 0.0,
            total_execution_time_ms: 0,
            memory_used_bytes: 0,
            cpu_time_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config);
        assert!(engine.is_ok(), "Should create wasmi engine successfully");
    }
}
