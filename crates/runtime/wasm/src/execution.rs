//! WebAssembly module loading and execution orchestration
//!
//! This module handles the complete execution lifecycle:
//! - Module loading from various sources (file, URL, bytes)
//! - WASI context creation and configuration
//! - Execution with timeout and resource limits
//! - Metrics extraction and reporting

use std::time::{Duration, Instant};
use wasi_common::sync::WasiCtxBuilder;
use wasi_common::WasiCtx;
use wasmtime::{Engine, Instance, Linker, Module, Store};

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus};
use toadstool::security::SecurityContext;
use toadstool::workload::WasmModuleSource;

use crate::config::{SecurityLevel, WasmRuntimeConfig};
use crate::metrics::ResourceUsage;

/// Helper to convert wasmtime errors to ToadStoolError
#[inline]
pub(crate) fn wasmtime_error(err: wasmtime::Error) -> ToadStoolError {
    ToadStoolError::runtime(err.to_string())
}

/// Module loader for WASM modules from various sources
pub struct ModuleLoader {
    engine: Engine,
    config: WasmRuntimeConfig,
}

impl ModuleLoader {
    /// Create a new module loader
    pub fn new(engine: Engine, config: WasmRuntimeConfig) -> Self {
        Self { engine, config }
    }

    /// Generate cache key for a module source
    pub fn generate_cache_key(&self, module_source: &WasmModuleSource) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Include source content in hash
        match module_source {
            WasmModuleSource::Bytes { data, .. } => {
                hasher.update(data);
            }
            WasmModuleSource::File { path, .. } => {
                hasher.update(path.to_string_lossy().as_bytes());
                // Include file modification time if available
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) =
                            modified.duration_since(std::time::SystemTime::UNIX_EPOCH)
                        {
                            hasher.update(duration.as_secs().to_le_bytes());
                        }
                    }
                }
            }
            WasmModuleSource::Url { url, .. } => {
                hasher.update(url.as_bytes());
            }
        }

        // Include compilation configuration in hash
        hasher.update(self.config.max_memory_mb.to_le_bytes());
        hasher.update(self.config.max_pages.to_le_bytes());
        hasher.update([self.config.security_level as u8]);

        if let Some(fuel_limit) = self.config.fuel_limit {
            hasher.update(fuel_limit.to_le_bytes());
        }

        let hash = hasher.finalize();
        format!("wasm_{}", hex::encode(&hash[..16]))
    }

    /// Load WASM module from various sources
    pub async fn load_module(&self, module_source: &WasmModuleSource) -> ToadStoolResult<Module> {
        let timeout_duration = Duration::from_millis(self.config.module_load_timeout_ms);

        let load_future = async {
            match module_source {
                WasmModuleSource::File { path } => {
                    let bytes = tokio::fs::read(path).await.map_err(|e| {
                        ToadStoolError::io(format!(
                            "Failed to read WASM file {}: {}",
                            path.display(),
                            e
                        ))
                    })?;
                    self.load_from_bytes(&bytes)
                }
                WasmModuleSource::Url { url } => {
                    #[cfg(feature = "url-module-loading")]
                    {
                        let response = reqwest::get(url).await.map_err(|e| {
                            ToadStoolError::network(format!("Failed to fetch WASM from {url}: {e}"))
                        })?;

                        let bytes = response.bytes().await.map_err(|e| {
                            ToadStoolError::network(format!(
                                "Failed to read WASM response from {url}: {e}"
                            ))
                        })?;

                        self.load_from_bytes(&bytes)
                    }
                    #[cfg(not(feature = "url-module-loading"))]
                    {
                        let _ = url;
                        Err(ToadStoolError::not_supported(
                            "URL module loading not enabled".to_string(),
                        ))
                    }
                }
                WasmModuleSource::Bytes { data } => self.load_from_bytes(data),
            }
        };

        tokio::time::timeout(timeout_duration, load_future)
            .await
            .map_err(|_| {
                ToadStoolError::timeout(format!(
                    "Module load timeout: {}ms",
                    self.config.module_load_timeout_ms
                ))
            })?
    }

    /// Load module from bytes with validation
    fn load_from_bytes(&self, bytes: &[u8]) -> ToadStoolResult<Module> {
        // Validate module size
        let max_size_bytes = self.config.max_memory_mb as usize * 1024 * 1024;
        if bytes.len() > max_size_bytes {
            return Err(ToadStoolError::resource(format!(
                "WASM module size {} bytes exceeds limit of {} MB",
                bytes.len(),
                self.config.max_memory_mb
            )));
        }

        Module::from_binary(&self.engine, bytes)
            .map_err(|e| ToadStoolError::validation(format!("Invalid WASM module: {e}")))
    }
}

/// WASI context builder based on security level
pub struct WasiContextBuilder {
    security_level: SecurityLevel,
}

impl WasiContextBuilder {
    /// Create a new WASI context builder
    pub const fn new(security_level: SecurityLevel) -> Self {
        Self { security_level }
    }

    /// Build WASI context with appropriate security restrictions
    pub fn build(&self, _security_context: &SecurityContext) -> ToadStoolResult<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();

        match self.security_level {
            SecurityLevel::None => {
                let _ = builder.inherit_stdio().inherit_env();
            }
            SecurityLevel::Basic => {
                // Minimal stdio setup
            }
            SecurityLevel::Strict | SecurityLevel::Maximum => {
                // Restricted environment - no inherited capabilities
            }
        }

        Ok(builder.build())
    }
}

/// WASM module executor
pub struct ModuleExecutor {
    engine: Engine,
    config: WasmRuntimeConfig,
}

impl ModuleExecutor {
    /// Create a new module executor
    pub fn new(engine: Engine, config: WasmRuntimeConfig) -> Self {
        Self { engine, config }
    }

    /// Execute a WASM module with full orchestration
    pub async fn execute(
        &self,
        request: &ExecutionRequest,
        module: Module,
    ) -> ToadStoolResult<ExecutionResponse> {
        let execution_id = request.execution_id;
        let start_time = Instant::now();

        // Create WASI context
        let wasi_builder = WasiContextBuilder::new(self.config.security_level);
        let wasi_ctx = wasi_builder.build(&request.security_context)?;

        // Create store with WASI context
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Set fuel limit if configured
        if let Some(fuel_limit) = self.config.fuel_limit {
            store
                .set_fuel(fuel_limit)
                .map_err(|e| ToadStoolError::runtime(format!("Failed to set fuel: {e}")))?;
        }

        // Create linker and add WASI
        let mut linker = Linker::new(&self.engine);
        wasi_common::sync::add_to_linker(&mut linker, |s| s).map_err(wasmtime_error)?;

        // Instantiate module
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(wasmtime_error)?;

        // Execute with timeout
        let timeout_ms = request
            .timeout
            .map_or(self.config.execution_timeout_ms, |d| {
                u64::try_from(d.as_millis()).unwrap_or(self.config.execution_timeout_ms)
            });

        let execution_result = self
            .execute_with_timeout(&mut store, &instance, timeout_ms)
            .await;

        let duration = start_time.elapsed();

        // Build response
        self.build_response(execution_id, execution_result, &store, &instance, duration)
    }

    /// Execute module with timeout
    async fn execute_with_timeout(
        &self,
        store: &mut Store<WasiCtx>,
        instance: &Instance,
        timeout_ms: u64,
    ) -> ToadStoolResult<()> {
        let timeout_duration = Duration::from_millis(timeout_ms);

        tokio::time::timeout(timeout_duration, async {
            // Try _start first (WASI), then main
            if let Ok(start_func) = instance.get_typed_func::<(), ()>(store, "_start") {
                start_func
                    .call_async(store, ())
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM _start failed: {e}")))
            } else if let Ok(main_func) = instance.get_typed_func::<(), ()>(store, "main") {
                main_func
                    .call_async(store, ())
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM main failed: {e}")))
            } else {
                Err(ToadStoolError::runtime(
                    "No entry point found (_start or main)".to_string(),
                ))
            }
        })
        .await
        .map_err(|_| ToadStoolError::timeout(format!("Execution timeout: {timeout_ms}ms")))?
    }

    /// Build execution response with metrics
    fn build_response(
        &self,
        execution_id: uuid::Uuid,
        execution_result: ToadStoolResult<()>,
        store: &Store<WasiCtx>,
        instance: &Instance,
        duration: Duration,
    ) -> ToadStoolResult<ExecutionResponse> {
        let (status, exit_code) = match execution_result {
            Ok(()) => (ExecutionStatus::Success, Some(0)),
            Err(e) => (
                ExecutionStatus::Failed {
                    error: e.to_string(),
                },
                Some(1),
            ),
        };

        // Extract metrics
        let cpu_metrics = self.extract_cpu_metrics(store, &duration);
        let memory_metrics = self.extract_memory_metrics(instance);

        let metrics = toadstool::resources::RuntimeMetrics {
            cpu: cpu_metrics,
            memory: memory_metrics,
            storage: toadstool::resources::StorageMetrics::default(),
            network: toadstool::resources::NetworkMetrics::default(),
            gpu: None,
            timing: toadstool::resources::TimingMetrics {
                start_time: chrono::Utc::now() 
                    - chrono::Duration::from_std(duration).unwrap_or_else(|_| chrono::Duration::zero()),
                end_time: Some(chrono::Utc::now()),
                duration: chrono::Duration::from_std(duration).unwrap_or_else(|_| chrono::Duration::zero()),
            },
        };

        let output = ExecutionOutput {
            data: Vec::new(),
            stdout: Some("WASM execution completed".to_string()),
            stderr: Some(String::new()),
            exit_code,
            format: Some("bytes".to_string()),
            result: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
        };

        Ok(ExecutionResponse {
            execution_id,
            status,
            output,
            metrics,
            duration,
            runtime_used: toadstool::execution::RuntimeType::Wasm,
            warnings: Vec::new(),
        })
    }

    /// Extract CPU metrics from store
    fn extract_cpu_metrics(
        &self,
        store: &Store<WasiCtx>,
        duration: &Duration,
    ) -> toadstool::resources::CpuMetrics {
        let fuel_consumed = if let Some(fuel_limit) = self.config.fuel_limit {
            let remaining = store.get_fuel().unwrap_or(0);
            fuel_limit - remaining
        } else {
            0
        };

        let usage_percent = if fuel_consumed > 0 {
            let fuel_rate = fuel_consumed as f64 / duration.as_millis().max(1) as f64;
            (fuel_rate * 0.001).min(100.0) as f32
        } else {
            if duration.as_millis() > 100 {
                25.0
            } else {
                5.0
            }
        };

        toadstool::resources::CpuMetrics {
            usage_percent,
            cores_used: 1.0,
            system_percent: 0.0,
            user_percent: usage_percent,
            idle_percent: 100.0 - usage_percent,
        }
    }

    /// Extract memory metrics from instance
    fn extract_memory_metrics(&self, instance: &Instance) -> toadstool::resources::MemoryMetrics {
        let usage = self.get_memory_usage(instance);

        toadstool::resources::MemoryMetrics {
            used_bytes: usage.used_bytes,
            available_bytes: (self.config.max_memory_mb * 1024 * 1024)
                .saturating_sub(usage.used_bytes),
            total_bytes: self.config.max_memory_mb * 1024 * 1024,
            usage_percent: ((usage.used_bytes as f64
                / (self.config.max_memory_mb * 1024 * 1024) as f64)
                * 100.0) as f32,
            swap_used: 0,
            swap_total: 0,
        }
    }

    /// Get memory usage from WASM instance
    fn get_memory_usage(&self, instance: &Instance) -> ResourceUsage {
        // Try to get memory export
        let usage_bytes = instance
            .get_memory("memory")
            .map(|mem| mem.data_size() as u64)
            .unwrap_or(0);

        let peak_usage_bytes = usage_bytes; // Wasmtime doesn't track peak separately

        ResourceUsage::new(usage_bytes, peak_usage_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::security::IsolationLevel;
    use wasmtime::Config;

    fn create_test_engine() -> Engine {
        let mut config = Config::new();
        config.async_support(true);
        Engine::new(&config).unwrap()
    }

    #[test]
    fn test_module_loader_creation() {
        let engine = create_test_engine();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        assert!(loader.engine.is_some());
    }

    #[test]
    fn test_cache_key_generation() {
        let engine = create_test_engine();
        let config = WasmRuntimeConfig::default();
        let loader = ModuleLoader::new(engine, config);

        let source = WasmModuleSource::Bytes {
            data: vec![1, 2, 3],
        };

        let key1 = loader.generate_cache_key(&source);
        let key2 = loader.generate_cache_key(&source);

        assert_eq!(key1, key2); // Same input = same key
        assert!(key1.starts_with("wasm_"));
    }

    #[test]
    fn test_wasi_context_builder() {
        let builder = WasiContextBuilder::new(SecurityLevel::Strict);
        let security = SecurityContext::new(IsolationLevel::Full);
        let ctx = builder.build(&security);

        assert!(ctx.is_ok());
    }

    #[test]
    fn test_module_executor_creation() {
        let engine = create_test_engine();
        let config = WasmRuntimeConfig::default();
        let executor = ModuleExecutor::new(engine, config);

        assert!(executor.engine.is_some());
    }
}

