// SPDX-License-Identifier: AGPL-3.0-or-later
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
use std::time::{Duration, SystemTime};
use tracing::{debug, info};
use wasmi::{Config, Engine};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{
    ExecutionRequest, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeEngine,
    RuntimeType,
};
use toadstool::resources::RuntimeMetrics;
use toadstool::WorkloadType;

use crate::cache_wasmi::ModuleCache;
use crate::config::WasmRuntimeConfig;
use crate::execution_wasmi::ModuleExecutor;
use crate::metrics::MetricsCollector;
use crate::module_loader::ModuleLoader;

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

    /// Module cache (reserved for future use)
    #[allow(dead_code)]
    cache: Arc<ModuleCache>,

    /// Module loader (reserved for future use)
    #[allow(dead_code)]
    loader: Arc<ModuleLoader>,

    /// Module executor
    executor: Arc<ModuleExecutor>,

    /// Component registry (when component model is enabled)
    /// EVOLVED: Complete integration for component model support
    component_registry: Option<Arc<crate::component_model::ComponentRegistry>>,

    /// Initialized flag
    initialized: bool,
}

impl std::fmt::Debug for WasmRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeEngine")
            .field("config", &self.config)
            .field("engine", &"<wasmi::Engine>")
            .field("metrics", &"<MetricsCollector>")
            .field(
                "component_registry",
                &self
                    .component_registry
                    .as_ref()
                    .map(|_| "<ComponentRegistry>"),
            )
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
        let config_arc = Arc::new(config.clone());
        let cache = Arc::new(ModuleCache::new(config_arc.cache.max_entries as usize));
        let loader = Arc::new(ModuleLoader::new(engine.clone(), config.clone()));
        let executor = Arc::new(ModuleExecutor::new(engine.clone(), config.clone()));

        // EVOLVED: Initialize component registry if component model is enabled
        let component_registry = config.component_model.as_ref().map(|cm_config| {
            info!("✅ Component model enabled - initializing registry");
            Arc::new(crate::component_model::ComponentRegistry::new(
                cm_config.clone(),
            ))
        });

        Ok(Self {
            engine,
            config: config_arc,
            metrics,
            cache,
            loader,
            executor,
            component_registry,
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

    /// Get component registry (if component model is enabled)
    /// EVOLVED: Public accessor for component model support
    pub fn component_registry(&self) -> Option<&Arc<crate::component_model::ComponentRegistry>> {
        self.component_registry.as_ref()
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
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            let start_time = std::time::Instant::now();

            // Extract WASM module source from request
            let (module_source, args) = match &request.workload {
                toadstool::WorkloadSpec::Wasm { module, args, .. } => {
                    (module, args.clone().unwrap_or_default())
                }
                _ => {
                    return Err(ToadStoolError::validation(
                        "Expected WASM workload spec".to_string(),
                    ));
                }
            };

            // Execute the module
            // Note: wasmi doesn't have explicit entry points like wasmtime,
            // it calls _start automatically or we get exports
            let output = self
                .executor
                .load_and_execute(module_source, "_start", args)
                .await?;

            let duration = start_time.elapsed();

            // Determine status from exit code
            let status = if output.exit_code.unwrap_or(0) == 0 {
                toadstool::execution::ExecutionStatus::Success
            } else {
                toadstool::execution::ExecutionStatus::Failed {
                    error: format!("Exit code: {}", output.exit_code.unwrap_or(-1)),
                }
            };

            // Record metrics
            match &status {
                toadstool::execution::ExecutionStatus::Success => {
                    self.metrics.record_success(duration.as_micros() as u64);
                }
                _ => {
                    self.metrics.record_failure();
                }
            }

            // Build response with correct structure
            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status,
                output,
                metrics: self.get_metrics().await?,
                duration,
                runtime_used: RuntimeType::Wasm,
                warnings: Vec::new(),
            })
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        use std::collections::HashMap;

        let mut features = HashMap::new();
        features.insert("wasi".to_string(), true);
        features.insert(
            "fuel_metering".to_string(),
            self.config.fuel_limit.is_some(),
        );
        features.insert("async".to_string(), true);

        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Wasm],
            max_concurrent_executions: Some(100),
            supported_architectures: vec![
                "x86_64".to_string(),
                "aarch64".to_string(),
                "arm".to_string(),
            ],
            platform_features: features,
            version: "1.0.7".to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Wasm)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async move {
            use toadstool::resources::{CpuMetrics, MemoryMetrics, TimingMetrics};

            let _total = self.metrics.total_executions();
            let _successful = self.metrics.successful_executions();
            let _failed = self.metrics.failed_executions();
            let avg_time_us = self.metrics.average_execution_time_us();

            Ok(RuntimeMetrics {
                cpu: CpuMetrics::default(),
                memory: MemoryMetrics::default(),
                storage: toadstool::resources::StorageMetrics::default(),
                network: toadstool::resources::NetworkMetrics::default(),
                gpu: None,
                timing: TimingMetrics {
                    start_time: SystemTime::now(),
                    end_time: None,
                    duration: Duration::from_millis(avg_time_us / 1000),
                },
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
        assert_eq!(caps.version, "1.0.7"); // wasmi version
        assert_eq!(caps.supported_workloads, vec![WorkloadType::Wasm]);
        // Check WASI support in platform_features
        assert_eq!(caps.platform_features.get("wasi"), Some(&true));
        // Check async support
        assert_eq!(caps.platform_features.get("async"), Some(&true));
    }
}
