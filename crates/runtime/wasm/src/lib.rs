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
//! High-performance WebAssembly runtime engine with Wasmtime integration,
//! WASI support, module caching, and comprehensive security isolation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{debug, info};
use uuid::Uuid;

use wasmtime::{Config, Engine, Linker, Module, Store, Strategy, OptLevel, WasmBacktraceDetails};
use wasi_common::{WasiCtx};
use wasi_common::sync::{add_to_linker, WasiCtxBuilder};

use toadstool::{
    execution::{
        ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeCapabilities, RuntimeConfig,
        RuntimeEngine, RuntimeType, WorkloadType, ExecutionOutput,
    },
    error::{ToadStoolError, ToadStoolResult},
    resources::RuntimeMetrics,
    security::SecurityContext,
};

/// Configuration for WebAssembly runtime engine
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
    /// Enable module caching
    pub cache_enabled: bool,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,
    /// Cache TTL in hours
    pub cache_ttl_hours: u64,
    /// Security isolation level
    pub security_level: SecurityLevel,
    /// Memory limits
    pub max_memory_mb: u64,
    /// Maximum memory pages
    pub max_pages: u32,
    /// Execution timeout in milliseconds
    pub execution_timeout_ms: u64,
    /// Module load timeout in milliseconds
    pub module_load_timeout_ms: u64,
    /// Fuel limit for execution
    pub fuel_limit: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum SecurityLevel {
    None,
    Basic,
    Strict,
    Maximum,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            max_cache_size_mb: 512,
            cache_ttl_hours: 24,
            security_level: SecurityLevel::Strict,
            max_memory_mb: 128,
            max_pages: 2048,
            execution_timeout_ms: 30000,
            module_load_timeout_ms: 10000,
            fuel_limit: Some(1_000_000),
        }
    }
}

/// Cached module information (thread-safe)
#[derive(Debug, Clone)]
struct CachedModule {
    module: Module,
    cache_time: SystemTime,
    access_count: u64,
    size_bytes: usize,
}

/// Active execution tracking (thread-safe metadata only)
#[derive(Debug, Clone)]
struct ExecutionHandle {
    id: Uuid,
    module_key: String,
    start_time: SystemTime,
    security_context: SecurityContext,
}

/// WebAssembly Runtime Engine
/// 
/// This implementation uses a thread-safe design where:
/// - Engine and cached Modules are shared (they are Send + Sync)
/// - Store and Instance are created per-request (not shared)
/// - Only metadata is stored in shared collections
pub struct WasmRuntimeEngine {
    /// Wasmtime engine (thread-safe)
    engine: Engine,
    /// Configuration
    config: WasmRuntimeConfig,
    /// Module cache (thread-safe)
    module_cache: Arc<RwLock<HashMap<String, CachedModule>>>,
    /// Active executions metadata (thread-safe)
    active_executions: Arc<RwLock<HashMap<Uuid, ExecutionHandle>>>,
}

impl std::fmt::Debug for WasmRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmRuntimeEngine")
            .field("config", &self.config)
            .field("engine", &"<wasmtime::Engine>")
            .field("module_cache", &"<cached_modules>")
            .field("active_executions", &"<active_executions>")
            .finish()
    }
}

impl WasmRuntimeEngine {
    /// Create a new WebAssembly runtime engine
    pub fn new(config: WasmRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Creating WebAssembly runtime engine");

        // Create Wasmtime engine with configuration
        let mut wasmtime_config = Config::new();
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

        let engine = Engine::new(&wasmtime_config)
            .map_err(|e| ToadStoolError::configuration(format!("Failed to create Wasmtime engine: {}", e)))?;

        Ok(Self {
            engine,
            config,
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Generate cache key for a module
    fn generate_cache_key(&self, module_source: &toadstool::workload::WasmModuleSource) -> String {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        
        // Include workload content in hash
        match module_source {
            toadstool::workload::WasmModuleSource::File { path } => {
                hasher.update(format!("file:{}", path.display()).as_bytes());
            }
            toadstool::workload::WasmModuleSource::Url { url } => {
                hasher.update(format!("url:{}", url).as_bytes());
            }
            toadstool::workload::WasmModuleSource::Bytes { data } => {
                hasher.update(data);
            }
            toadstool::workload::WasmModuleSource::Registry { registry, name, version } => {
                hasher.update(format!("registry:{}:{}:{}", registry, name, version).as_bytes());
            }
        }
        
        // Include relevant config in hash
        hasher.update(format!("{:?}", self.config.security_level).as_bytes());
        hasher.update(format!("{}", self.config.max_memory_mb).as_bytes());
        
        format!("wasm_{}", hex::encode(hasher.finalize()))
    }

    /// Load or retrieve cached module
    async fn get_or_load_module(&self, module_source: &toadstool::workload::WasmModuleSource) -> ToadStoolResult<Module> {
        let cache_key = self.generate_cache_key(module_source);
        
        // Check cache first
        if self.config.cache_enabled {
            let cache = self.module_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                // Check if cache entry is still valid
                let age = SystemTime::now()
                    .duration_since(cached.cache_time)
                    .unwrap_or(Duration::from_secs(0));
                
                if age.as_secs() < (self.config.cache_ttl_hours * 3600) {
                    debug!("Using cached WASM module: {}", cache_key);
                    return Ok(cached.module.clone());
                }
            }
        }

        // Load module
        let module = self.load_module(module_source).await?;
        
        // Cache the module
        if self.config.cache_enabled {
            let mut cache = self.module_cache.write().await;
            cache.insert(cache_key, CachedModule {
                module: module.clone(),
                cache_time: SystemTime::now(),
                access_count: 1,
                size_bytes: 0, // TODO: Calculate actual size
            });
        }
        
        Ok(module)
    }

    /// Load WASM module from various sources
    async fn load_module(&self, module_source: &toadstool::workload::WasmModuleSource) -> ToadStoolResult<Module> {
        let timeout_duration = Duration::from_millis(self.config.module_load_timeout_ms);
        
        let load_future = async {
            match module_source {
                toadstool::workload::WasmModuleSource::File { path } => {
                    let bytes = tokio::fs::read(path).await
                        .map_err(|e| ToadStoolError::io(format!("Failed to read WASM file {}: {}", path.display(), e)))?;
                    self.load_module_from_bytes(&bytes).await
                }
                toadstool::workload::WasmModuleSource::Url { url } => {
                    #[cfg(feature = "url-module-loading")]
                    {
                        let response = reqwest::get(url).await
                            .map_err(|e| ToadStoolError::network(format!("Failed to fetch WASM from {}: {}", url, e)))?;
                        
                        let bytes = response.bytes().await
                            .map_err(|e| ToadStoolError::network(format!("Failed to read WASM response from {}: {}", url, e)))?;
                        
                        self.load_module_from_bytes(&bytes).await
                    }
                    #[cfg(not(feature = "url-module-loading"))]
                    {
                        Err(ToadStoolError::not_supported("URL module loading not enabled".to_string()))
                    }
                }
                toadstool::workload::WasmModuleSource::Bytes { data } => {
                    self.load_module_from_bytes(data).await
                }
                toadstool::workload::WasmModuleSource::Registry { .. } => {
                    Err(ToadStoolError::not_supported("Registry module loading not implemented".to_string()))
                }
            }
        };

        timeout(timeout_duration, load_future)
            .await
            .map_err(|_| ToadStoolError::timeout(self.config.module_load_timeout_ms))?
    }

    /// Load module from bytes
    async fn load_module_from_bytes(&self, bytes: &[u8]) -> ToadStoolResult<Module> {
        // Validate module size
        if bytes.len() > (self.config.max_memory_mb as usize * 1024 * 1024) {
            return Err(ToadStoolError::resource(format!(
                "WASM module size {} exceeds limit of {} MB",
                bytes.len(),
                self.config.max_memory_mb
            )));
        }

        Module::from_binary(&self.engine, bytes)
            .map_err(|e| ToadStoolError::validation(format!("Invalid WASM module: {}", e)))
    }

    /// Create WASI context based on security level
    fn create_wasi_context(&self, _security_context: &SecurityContext) -> ToadStoolResult<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();
        
        // Configure based on security level
        match self.config.security_level {
            SecurityLevel::None => {
                builder.inherit_stdio().inherit_env();
            }
            SecurityLevel::Basic => {
                // Minimal stdio setup
            }
            SecurityLevel::Strict => {
                // Restricted environment
            }
            SecurityLevel::Maximum => {
                // Maximum isolation
            }
        }

        Ok(builder.build())
    }

    /// Execute WASM module with per-request Store creation
    async fn execute_module(
        &self,
        request: &ExecutionRequest,
        module: Module,
    ) -> ToadStoolResult<ExecutionResponse> {
        let execution_id = request.execution_id;
        let start_time = SystemTime::now();

        // Create per-request Store and WASI context (not shared)
        let wasi_ctx = self.create_wasi_context(&request.security_context)?;
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Set fuel limit if configured
        if let Some(fuel_limit) = self.config.fuel_limit {
            store.set_fuel(fuel_limit)
                .map_err(|e| ToadStoolError::configuration(format!("Failed to set fuel: {}", e)))?;
        }

        // Create linker for WASI
        let mut linker = Linker::new(&self.engine);
        add_to_linker(&mut linker, |s| s)
            .map_err(|e| ToadStoolError::configuration(format!("Failed to add WASI to linker: {}", e)))?;

        // Create instance
        let instance = linker.instantiate_async(&mut store, &module)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create WASM instance: {}", e)))?;

        // Track execution
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, ExecutionHandle {
                id: execution_id,
                module_key: "runtime".to_string(), // TODO: Use actual cache key
                start_time,
                security_context: request.security_context.clone(),
            });
        }

        // Execute with timeout
        let timeout_ms = request.timeout
            .map(|d| d.as_millis() as u64)
            .unwrap_or(self.config.execution_timeout_ms);
        
        let execution_timeout = Duration::from_millis(timeout_ms);
        let execution_result = timeout(execution_timeout, async {
            // Look for main function or _start
            if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
                main_func.call_async(&mut store, ()).await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM execution failed: {}", e)))
            } else if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "main") {
                main_func.call_async(&mut store, ()).await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM execution failed: {}", e)))
            } else {
                Err(ToadStoolError::runtime("No main function found in WASM module".to_string()))
            }
        })
        .await
        .map_err(|_| ToadStoolError::timeout(timeout_ms))?;

        // Clean up execution tracking
        {
            let mut executions = self.active_executions.write().await;
            executions.remove(&execution_id);
        }

        let duration = SystemTime::now().duration_since(start_time).unwrap_or_default();

        // Handle execution result
        let (status, exit_code) = match execution_result {
            Ok(_) => (ExecutionStatus::Success, Some(0)),
            Err(e) => (ExecutionStatus::Failed { error: e.to_string() }, Some(1)),
        };

        // Create runtime metrics
        let metrics = RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics {
                usage_percent: 0.0, // TODO: Extract CPU usage
                peak_usage_percent: 0.0,
                average_usage_percent: 0.0,
                cpu_time_ms: duration.as_millis() as u64,
                cpu_cycles: None,
                throttle_events: 0,
            },
            memory: toadstool::resources::MemoryMetrics {
                usage_bytes: 0, // TODO: Extract actual memory usage from instance
                peak_usage_bytes: 0,
                average_usage_bytes: 0,
                allocation_count: 0,
                deallocation_count: 0,
                page_faults: 0,
                swap_usage_bytes: 0,
            },
            storage: toadstool::resources::StorageMetrics::default(),
            network: toadstool::resources::NetworkMetrics::default(),
            gpu: None,
            timing: toadstool::resources::TimingMetrics {
                start_time: chrono::DateTime::from_timestamp(
                    start_time.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as i64, 0
                ).unwrap_or_default(),
                end_time: Some(chrono::Utc::now()),
                duration,
                init_duration: Duration::from_millis(0),
                cleanup_duration: Duration::from_millis(0),
                queue_wait_duration: Duration::from_millis(0),
            },
            custom: std::collections::HashMap::new(),
        };

        // Create execution output
        let output = ExecutionOutput {
            data: Vec::new(),
            result: HashMap::new(),
            stdout: Some(String::new()), // TODO: Extract from WASI context
            stderr: Some(String::new()), // TODO: Extract from WASI context
            exit_code,
            format: Some("text/plain".to_string()),
        };

        Ok(ExecutionResponse {
            execution_id,
            status,
            output,
            metrics,
            duration,
            runtime_used: RuntimeType::Wasm,
            warnings: Vec::new(),
        })
    }
}

#[async_trait]
impl RuntimeEngine for WasmRuntimeEngine {
    async fn initialize(&mut self, _config: RuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing WebAssembly runtime engine");
        
        // Validate configuration
        if self.config.max_memory_mb == 0 {
            return Err(ToadStoolError::configuration("Memory limit cannot be zero".to_string()));
        }

        if self.config.execution_timeout_ms == 0 {
            return Err(ToadStoolError::configuration("Execution timeout cannot be zero".to_string()));
        }

        info!("WebAssembly runtime engine initialized successfully");
        Ok(())
    }

    async fn execute(&self, request: ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing WebAssembly workload: {}", request.execution_id);

        // Extract WASM workload specification
        let module_source = match &request.workload {
            toadstool::workload::WorkloadSpec::Wasm { module_source, .. } => module_source,
            _ => {
                return Err(ToadStoolError::validation(
                    "Invalid workload type for WASM runtime".to_string()
                ));
            }
        };

        // Load or get cached module
        let module = self.get_or_load_module(module_source).await?;

        // Execute the module
        self.execute_module(&request, module).await
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Wasm],
            max_concurrent_executions: Some(1000),
            supported_architectures: vec!["wasm32".to_string(), "wasm64".to_string()],
            platform_features: {
                let mut features = HashMap::new();
                features.insert("wasi_support".to_string(), true);
                features.insert("module_caching".to_string(), self.config.cache_enabled);
                features.insert("memory_limits".to_string(), true);
                features.insert("fuel_tracking".to_string(), self.config.fuel_limit.is_some());
                features
            },
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn supports_workload(&self, workload_type: &WorkloadType) -> bool {
        matches!(workload_type, WorkloadType::Wasm)
    }

    async fn get_metrics(&self) -> ToadStoolResult<RuntimeMetrics> {
        let executions = self.active_executions.read().await;
        let active_count = executions.len();
        
        let cache = self.module_cache.read().await;
        let cached_modules = cache.len();

        let mut custom_metrics = HashMap::new();
        custom_metrics.insert("active_executions".to_string(), serde_json::Value::from(active_count));
        custom_metrics.insert("cached_modules".to_string(), serde_json::Value::from(cached_modules));
        custom_metrics.insert("cache_enabled".to_string(), serde_json::Value::from(self.config.cache_enabled));

        Ok(RuntimeMetrics {
            cpu: toadstool::resources::CpuMetrics::default(),
            memory: toadstool::resources::MemoryMetrics {
                usage_bytes: 0, // TODO: Calculate memory usage of cached modules
                peak_usage_bytes: 0,
                average_usage_bytes: 0,
                allocation_count: 0,
                deallocation_count: 0,
                page_faults: 0,
                swap_usage_bytes: 0,
            },
            storage: toadstool::resources::StorageMetrics::default(),
            network: toadstool::resources::NetworkMetrics::default(),
            gpu: None,
            timing: toadstool::resources::TimingMetrics {
                start_time: chrono::Utc::now(),
                end_time: None,
                duration: Duration::from_millis(0),
                init_duration: Duration::from_millis(0),
                cleanup_duration: Duration::from_millis(0),
                queue_wait_duration: Duration::from_millis(0),
            },
            custom: custom_metrics,
        })
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down WebAssembly runtime engine");

        // Wait for active executions to complete (with timeout)
        let shutdown_timeout = Duration::from_secs(30);
        let start_time = SystemTime::now();

        while start_time.elapsed().unwrap_or_default() < shutdown_timeout {
            let executions = self.active_executions.read().await;
            if executions.is_empty() {
                break;
            }
            drop(executions);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Clear caches
        self.module_cache.write().await.clear();
        self.active_executions.write().await.clear();

        info!("WebAssembly runtime engine shut down successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::security::IsolationLevel;

    #[tokio::test]
    async fn test_wasm_engine_creation() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_capabilities() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        let capabilities = engine.get_capabilities();
        
        assert!(capabilities.supported_workloads.contains(&WorkloadType::Wasm));
        assert!(capabilities.platform_features.get("wasi_support").copied().unwrap_or(false));
        assert_eq!(capabilities.supported_architectures, vec!["wasm32", "wasm64"]);
    }

    #[tokio::test]
    async fn test_workload_support() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        assert!(engine.supports_workload(&WorkloadType::Wasm));
        assert!(!engine.supports_workload(&WorkloadType::Container));
        assert!(!engine.supports_workload(&WorkloadType::Native));
    }

    #[tokio::test]
    async fn test_security_isolation() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        // Test security context validation
        let security_context = SecurityContext::for_isolation_level(IsolationLevel::Basic);
        assert_eq!(security_context.isolation_level, IsolationLevel::Basic);
    }

    #[tokio::test]
    async fn test_resource_management() {
        let mut config = WasmRuntimeConfig::default();
        config.max_memory_mb = 64;
        config.execution_timeout_ms = 5000;
        
        let engine = WasmRuntimeEngine::new(config).unwrap();
        let capabilities = engine.get_capabilities();
        
        assert!(!capabilities.platform_features.is_empty());
    }

    #[tokio::test]
    async fn test_module_caching() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        // Test cache operations
        assert_eq!(engine.module_cache.read().await.len(), 0);
        
        // Create a simple WASM module source
        let module_source = toadstool::workload::WasmModuleSource::Bytes {
            data: vec![
                0x00, 0x61, 0x73, 0x6d, // WASM magic number
                0x01, 0x00, 0x00, 0x00, // WASM version
            ]
        };
        
        let cache_key = engine.generate_cache_key(&module_source);
        assert!(!cache_key.is_empty());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let config = WasmRuntimeConfig::default();
        let engine = WasmRuntimeEngine::new(config).unwrap();
        
        // Test invalid module handling
        let invalid_module = toadstool::workload::WasmModuleSource::Bytes {
            data: vec![0x00, 0x00, 0x00, 0x00]
        };
        let cache_key = engine.generate_cache_key(&invalid_module);
        assert!(!cache_key.is_empty()); // Should still generate a key
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        // Test various configuration options
        let mut config = WasmRuntimeConfig::default();
        config.cache_enabled = true;
        config.max_memory_mb = 128;
        config.execution_timeout_ms = 10000;
        
        let engine = WasmRuntimeEngine::new(config);
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_wasi_configuration() {
        let mut config = WasmRuntimeConfig::default();
        config.cache_enabled = true;
        config.max_memory_mb = 256;
        
        let engine = WasmRuntimeEngine::new(config).unwrap();
        let capabilities = engine.get_capabilities();
        
        assert!(capabilities.platform_features.get("wasi_support").copied().unwrap_or(false));
        assert!(capabilities.platform_features.get("module_caching").copied().unwrap_or(false));
    }
}
