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

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, debug};
use wasmi::{Config, Engine};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeEngine};
use toadstool::resources::RuntimeMetrics;
use toadstool::WorkloadType;

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
    
    /// Initialized flag
    initialized: bool,
}

impl std::fmt::Debug for WasmRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeEngine")
            .field("config", &self.config)
            .field("engine", &"<wasmi::Engine>")
            .field("metrics", &"<MetricsCollector>")
            .field("initialized", &self.initialized)
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
            initialized: false,
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

impl RuntimeEngine for WasmRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            info!("Initializing wasmi runtime engine");
            self.initialized = true;
            Ok(())
        })
    }

    fn execute(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            // TODO: Implement full execution logic
            Err(ToadStoolError::not_supported(
                "Wasmi execution implementation in progress".to_string(),
            ))
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Wasm],
            max_concurrent_executions: Some(100), // Configurable in future
            supported_architectures: vec![
                "x86_64".to_string(),
                "aarch64".to_string(),
                "arm".to_string(),
            ],
            platform_features: vec![
                "wasi".to_string(),
                "fuel_metering".to_string(),
            ],
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Wasm)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async move {
            let total = self.metrics.total_executions();
            let successful = self.metrics.successful_executions();
            let failed = self.metrics.failed_executions();
            let avg_time_us = self.metrics.average_execution_time_us();
            
            Ok(RuntimeMetrics {
                total_executions: total,
                successful_executions: successful,
                failed_executions: failed,
                average_execution_time_ms: (avg_time_us as f64) / 1000.0,
                total_execution_time_ms: 0, // TODO: track separately
                memory_used_bytes: 0, // TODO: track memory
                cpu_time_ms: 0, // TODO: track CPU time
            })
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            info!("Shutting down wasmi runtime engine");
            self.initialized = false;
            Ok(())
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

    #[test]
    fn test_get_capabilities() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        let caps = engine.get_capabilities();
        assert_eq!(caps.name, "wasmi");
        assert!(caps.supports_wasi);
    }
}
