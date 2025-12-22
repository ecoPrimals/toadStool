//! WebAssembly Runtime Engine
//!
//! Main engine implementation that integrates all WASM runtime components:
//! - Configuration management
//! - Module caching
//! - Execution orchestration
//! - Metrics collection
//! - Component model support

use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::info;
use wasmtime::{Config, Engine, OptLevel, Strategy, WasmBacktraceDetails};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionRequest, ExecutionResponse, RuntimeEngine};
use toadstool::resources::RuntimeMetrics;

use crate::cache::ModuleCache;
use crate::component_model::{ComponentModelSupport, ComponentRegistry};
use crate::config::{SecurityLevel, WasmRuntimeConfig};
use crate::execution::{ModuleExecutor, ModuleLoader};
use crate::metrics::MetricsCollector;

/// WebAssembly Runtime Engine
///
/// High-performance WASM runtime with comprehensive features:
/// - Wasmtime integration
/// - Module caching
/// - WASI support
/// - Security isolation
/// - Component model support
pub struct WasmRuntimeEngine {
    /// Wasmtime engine (thread-safe)
    engine: Engine,
    
    /// Runtime configuration
    config: WasmRuntimeConfig,
    
    /// Module cache
    cache: Arc<ModuleCache>,
    
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    
    /// Component model registry
    component_registry: Arc<ComponentRegistry>,
    
    /// Module loader
    loader: Arc<ModuleLoader>,
    
    /// Module executor
    executor: Arc<ModuleExecutor>,
}

impl std::fmt::Debug for WasmRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeEngine")
            .field("config", &self.config)
            .field("engine", &"<wasmtime::Engine>")
            .field("cache", &"<ModuleCache>")
            .field("metrics", &"<MetricsCollector>")
            .finish()
    }
}

impl WasmRuntimeEngine {
    /// Create a new WebAssembly runtime engine
    pub fn new(config: WasmRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Creating WebAssembly runtime engine");

        // Validate configuration
        config.validate().map_err(ToadStoolError::configuration)?;

        // Create Wasmtime engine with configuration
        let engine = Self::create_wasmtime_engine(&config)?;

        // Initialize components
        let cache = Arc::new(ModuleCache::new(config.cache.max_entries));
        let metrics = Arc::new(MetricsCollector::new());
        // ✅ ZERO-COPY: Share config via Arc instead of cloning then wrapping
        let config = Arc::new(config);
        let component_registry = Arc::new(ComponentRegistry::new(config.component_model.clone()));
        
        let loader = Arc::new(ModuleLoader::new(engine.clone(), Arc::clone(&config)));
        let executor = Arc::new(ModuleExecutor::new(engine.clone(), Arc::clone(&config)));

        Ok(Self {
            engine,
            config,
            cache,
            metrics,
            component_registry,
            loader,
            executor,
        })
    }

    /// Create Wasmtime engine with appropriate configuration
    fn create_wasmtime_engine(config: &WasmRuntimeConfig) -> ToadStoolResult<Engine> {
        let mut wasmtime_config = Config::new();
        
        // Enable debugging and async support
        wasmtime_config.wasm_backtrace_details(WasmBacktraceDetails::Enable);
        wasmtime_config.wasm_multi_memory(true);
        wasmtime_config.async_support(true);

        // Configure compilation strategy based on security level
        match config.security_level {
            SecurityLevel::None | SecurityLevel::Basic => {
                wasmtime_config.strategy(Strategy::Cranelift);
            }
            SecurityLevel::Strict | SecurityLevel::Maximum => {
                wasmtime_config.strategy(Strategy::Cranelift);
                wasmtime_config.cranelift_opt_level(OptLevel::Speed);
            }
        }

        // Enable fuel for execution limits
        if config.fuel_limit.is_some() {
            wasmtime_config.consume_fuel(true);
        }

        Engine::new(&wasmtime_config).map_err(|e| {
            ToadStoolError::configuration(format!("Failed to create Wasmtime engine: {e}"))
        })
    }

    /// Get or load module with caching
    async fn get_or_load_module(
        &self,
        module_source: &toadstool::workload::WasmModuleSource,
    ) -> ToadStoolResult<wasmtime::Module> {
        let cache_key = self.loader.generate_cache_key(module_source);

        // Try cache first
        if self.config.cache.enabled {
            if let Some(module) = self.cache.get(&cache_key, &self.engine).await {
                return Ok(module);
            }
        }

        // Load module
        let module = self.loader.load_module(module_source).await?;

        // Cache it
        if self.config.cache.enabled {
            let _ = self.cache.insert(cache_key, &module).await;
        }

        Ok(module)
    }
}

#[async_trait]
impl RuntimeEngine for WasmRuntimeEngine {
    fn initialize(
        &mut self,
        _config: toadstool::execution::RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Initializing WebAssembly runtime engine");

            // Configuration already validated in new()
            info!("WebAssembly runtime engine initialized successfully");
            Ok(())
        })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        Box::pin(async move {
            info!("Executing WebAssembly workload: {}", request.execution_id);

            // Extract WASM workload specification
            let toadstool::workload::WorkloadSpec::Wasm {
                module: module_source,
                ..
            } = &request.workload
            else {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for WASM runtime".to_string(),
                ));
            };

            // Register execution
            let cache_key = self.loader.generate_cache_key(module_source);
            let handle = crate::metrics::ExecutionHandle::new(
                request.execution_id,
                cache_key,
                request.security_context.clone(),
            );
            self.metrics.register_execution(handle).await;

            // Load or get cached module
            let module = self.get_or_load_module(module_source).await?;

            // Execute the module
            let result = self.executor.execute(&request, module).await;

            // Mark execution complete
            let success = result.is_ok();
            self.metrics.complete_execution(request.execution_id, success).await;

            result
        })
    }

    fn get_capabilities(&self) -> toadstool::execution::RuntimeCapabilities {
        toadstool::execution::RuntimeCapabilities {
            supported_workloads: vec![toadstool::workload::WorkloadType::Wasm],
            max_concurrent_executions: Some(1000),
            supported_architectures: vec!["wasm32".to_string(), "wasm64".to_string()],
            platform_features: {
                let mut features = HashMap::new();
                features.insert("wasi_support".to_string(), true);
                features.insert("module_caching".to_string(), self.config.cache.enabled);
                features.insert("memory_limits".to_string(), true);
                features.insert("fuel_tracking".to_string(), self.config.fuel_limit.is_some());
                features.insert("component_model".to_string(), true);
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &toadstool::workload::WorkloadType) -> bool {
        matches!(workload_type, toadstool::workload::WorkloadType::Wasm)
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async {
            let active_count = self.metrics.active_count().await;
            let cache_metrics = self.cache.metrics().await;

            let cpu_metrics = toadstool::resources::CpuMetrics {
                usage_percent: (active_count as f32 / 1000.0) * 100.0,
                cores_used: active_count as f32,
                system_percent: 0.0,
                user_percent: 0.0,
                idle_percent: 100.0,
            };

            let memory_metrics = toadstool::resources::MemoryMetrics {
                used_bytes: cache_metrics.memory_usage_bytes,
                available_bytes: (self.config.max_memory_mb * 1024 * 1024)
                    .saturating_sub(cache_metrics.memory_usage_bytes),
                total_bytes: self.config.max_memory_mb * 1024 * 1024,
                usage_percent: ((cache_metrics.memory_usage_bytes as f64
                    / (self.config.max_memory_mb * 1024 * 1024) as f64)
                    * 100.0) as f32,
                swap_used: 0,
                swap_total: 0,
            };

            Ok(RuntimeMetrics {
                cpu: cpu_metrics,
                memory: memory_metrics,
                storage: toadstool::resources::StorageMetrics::default(),
                network: toadstool::resources::NetworkMetrics::default(),
                gpu: None,
                timing: toadstool::resources::TimingMetrics {
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    duration: chrono::Duration::zero(),
                },
            })
        })
    }
}

impl ComponentModelSupport for WasmRuntimeEngine {
    fn register_component(
        &self,
        component_id: String,
        component_data: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            self.component_registry
                .register_component(component_id, component_data)
                .await
        })
    }

    fn instantiate_component(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<uuid::Uuid>> + Send + '_>> {
        Box::pin(async move {
            self.component_registry
                .instantiate_component(component_id)
                .await
        })
    }

    fn call_component_function(
        &self,
        instance_id: uuid::Uuid,
        function_name: &str,
        args: Vec<Vec<u8>>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>> {
        let function_name = function_name.to_string();
        Box::pin(async move {
            self.component_registry
                .call_component_function(instance_id, &function_name, args)
                .await
        })
    }

    fn get_component_exports(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<String>>> + Send + '_>> {
        Box::pin(async move {
            self.component_registry
                .get_component_exports(component_id)
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::security::IsolationLevel;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_creation() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_capabilities() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        let caps = engine.get_capabilities();
        assert_eq!(caps.supported_workloads.len(), 1);
        assert!(caps.platform_features.contains_key("wasi_support"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_supports_workload() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        assert!(engine.supports_workload(&toadstool::workload::WorkloadType::Wasm));
        assert!(!engine.supports_workload(&toadstool::workload::WorkloadType::Container));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_metrics() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        let metrics = engine.get_metrics().await;
        assert!(metrics.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_engine_initialization() {
        let config = WasmRuntimeConfig::default();
        let mut engine = WasmRuntimeEngine::new(config).unwrap();
        
        let runtime_config = toadstool::execution::RuntimeConfig::default();
        let result = engine.initialize(runtime_config).await;
        assert!(result.is_ok());
    }
}

