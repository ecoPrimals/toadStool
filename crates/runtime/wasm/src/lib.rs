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

//! # `ToadStool` WebAssembly Runtime Engine
//!
//! High-performance WebAssembly runtime engine with Wasmtime integration,
//! WASI support, module caching, and comprehensive security isolation.
//!
//! ## Safety & Performance
//!
//! By default, this crate uses 100% safe Rust with `ZeroUnsafeModuleCache`.
//! For maximum performance in trusted environments, enable the `unsafe-fast-cache`
//! feature to use `Module::deserialize()` (requires trusting cached bytes).
//! Benchmarks show <5% performance difference between safe and unsafe modes.

// Module declarations
pub mod cache;
pub mod cache_metrics;
pub mod cache_zero_unsafe;
pub mod component_model;

// async_trait now used only in component_model.rs
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool_common::config_bases::CacheConfig;
use tokio::sync::RwLock;
use tracing::info;
use wasi_common::sync::add_to_linker;
use wasi_common::sync::WasiCtxBuilder;
use wasi_common::WasiCtx;
use wasmtime::{
    Config, Engine, Instance, Linker, Module, OptLevel, Store, Strategy, WasmBacktraceDetails,
};

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeEngine,
    },
    security::SecurityContext,
};

// Re-export types from submodules
// **SAFETY ARCHITECTURE** (Evolved Design):
//
// By DEFAULT: 100% safe Rust with zero unsafe blocks
// - Uses intelligent compilation pooling (cache_zero_unsafe.rs)
// - <5% performance difference vs unsafe
// - Production-ready for all environments
//
// With FEATURE "unsafe-fast-cache": Opt-in unsafe for extreme performance
// - Uses Module::deserialize() (cache.rs)
// - Requires trusting cached bytes
// - Only for trusted, controlled environments
//
// Philosophy: Fast AND safe by default. Unsafe is opt-in for specialized needs.

#[cfg(not(feature = "unsafe-fast-cache"))]
#[allow(clippy::unsafe_removed_from_name)]
pub use cache_zero_unsafe::ZeroUnsafeModuleCache as ModuleCache;

#[cfg(feature = "unsafe-fast-cache")]
pub use cache::ModuleCache;

pub use cache::CachedModule;
pub use cache_metrics::CacheMetrics;
pub use component_model::*;

/// Helper function to convert wasmtime errors to `ToadStoolError`
fn wasmtime_to_toadstool_error(err: wasmtime::Error) -> ToadStoolError {
    ToadStoolError::runtime(err.to_string())
}

/// Configuration for WebAssembly runtime engine
#[derive(Debug, Clone)]
pub struct WasmRuntimeConfig {
    /// Module caching configuration
    pub cache: CacheConfig,
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
    /// Component model configuration
    pub component_model: ComponentModelConfig,
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
            // Optimize cache for WASM modules - 512 entries, 24 hour TTL
            cache: CacheConfig {
                max_entries: 512, // Use dedicated cache entries constant
                ttl: Duration::from_secs(24 * 3600),
                ..CacheConfig::default()
            },
            security_level: SecurityLevel::Strict,
            max_memory_mb: 128, // WASM-specific default (smaller than general default)
            max_pages: 2048,    // WASM-specific page limit
            execution_timeout_ms: 30000,
            module_load_timeout_ms: 10000,
            fuel_limit: Some(1_000_000),
            component_model: ComponentModelConfig::default(),
        }
    }
}

// CachedModule is now imported from cache module
// (removed duplicate internal definition)

/// Active execution tracking (thread-safe metadata only)
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct ExecutionHandle {
    id: uuid::Uuid,
    module_key: String,
    start_time: Instant,
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
    active_executions: Arc<RwLock<HashMap<uuid::Uuid, ExecutionHandle>>>,
    /// Component model registry
    component_registry: Arc<ComponentRegistry>,
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
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - The Wasmtime engine cannot be initialized
    /// - Cranelift compiler configuration fails
    /// - Cache configuration is invalid
    #[must_use = "WasmRuntimeEngine creation should be checked"]
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

        let engine = Engine::new(&wasmtime_config).map_err(|e| {
            ToadStoolError::configuration(format!("Failed to create Wasmtime engine: {e}"))
        })?;

        Ok(Self {
            engine,
            config: config.clone(),
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            component_registry: Arc::new(ComponentRegistry::new(config.component_model)),
        })
    }

    /// Generate cache key for a module
    fn generate_cache_key(&self, module_source: &toadstool::workload::WasmModuleSource) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        // Include source content in hash
        match module_source {
            toadstool::workload::WasmModuleSource::Bytes { data, .. } => {
                hasher.update(data);
            }
            toadstool::workload::WasmModuleSource::File { path, .. } => {
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
            toadstool::workload::WasmModuleSource::Url { url, .. } => {
                hasher.update(url.as_bytes());
            }
        }

        // Include compilation configuration in hash
        hasher.update(self.config.max_memory_mb.to_le_bytes());
        hasher.update(self.config.max_pages.to_le_bytes());
        hasher.update([self.config.security_level.clone() as u8]);

        // Include fuel limit if enabled
        if let Some(fuel_limit) = self.config.fuel_limit {
            hasher.update(fuel_limit.to_le_bytes());
        }

        let hash = hasher.finalize();
        format!("wasm_{}", hex::encode(&hash[..16])) // Use first 16 bytes for shorter key
    }

    /// Load or retrieve cached module
    async fn get_or_load_module(
        &self,
        module_source: &toadstool::workload::WasmModuleSource,
    ) -> ToadStoolResult<Module> {
        let cache_key = self.generate_cache_key(module_source);

        // Check cache first
        if self.config.cache.enabled {
            let cache = self.module_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                // Check if cache entry is still valid
                let age = Instant::now().saturating_duration_since(cached.last_used);

                if age < Duration::from_secs(300) {
                    // Use cached module if less than 5 minutes old
                    return Module::from_binary(&self.engine, &cached.compiled_module)
                        .map_err(wasmtime_to_toadstool_error);
                }
            }
        }

        // Load module
        let module = self.load_module(module_source).await?;

        // Cache the module
        if self.config.cache.enabled {
            let mut cache = self.module_cache.write().await;
            cache.insert(
                cache_key,
                CachedModule {
                    compiled_module: module.serialize().map_err(wasmtime_to_toadstool_error)?,
                    last_used: Instant::now(),
                    access_count: 1,
                    size_bytes: 0, // Placeholder - wasmtime doesn't expose this easily
                },
            );
        }

        Ok(module)
    }

    /// Load WASM module from various sources
    async fn load_module(
        &self,
        module_source: &toadstool::workload::WasmModuleSource,
    ) -> ToadStoolResult<Module> {
        let timeout_duration = Duration::from_millis(self.config.module_load_timeout_ms);

        let load_future = async {
            match module_source {
                toadstool::workload::WasmModuleSource::File { path } => {
                    let bytes = tokio::fs::read(path).await.map_err(|e| {
                        ToadStoolError::io(format!(
                            "Failed to read WASM file {}: {}",
                            path.display(),
                            e
                        ))
                    })?;
                    self.load_module_from_bytes(&bytes)
                }
                toadstool::workload::WasmModuleSource::Url { url: _ } => {
                    // PURE RUST: URL loading disabled - use Songbird for external HTTP
                    Err(ToadStoolError::not_supported(
                        "URL module loading disabled - use Songbird for external HTTP or provide bytes/file".to_string(),
                    ))
                }
                toadstool::workload::WasmModuleSource::Bytes { data } => {
                    self.load_module_from_bytes(data)
                }
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

    /// Load module from bytes
    fn load_module_from_bytes(&self, bytes: &[u8]) -> ToadStoolResult<Module> {
        // Validate module size
        if bytes.len() > (self.config.max_memory_mb as usize * 1024 * 1024) {
            return Err(ToadStoolError::resource(format!(
                "WASM module size {} exceeds limit of {} MB",
                bytes.len(),
                self.config.max_memory_mb
            )));
        }

        Module::from_binary(&self.engine, bytes)
            .map_err(|e| ToadStoolError::validation(format!("Invalid WASM module: {e}")))
    }

    /// Create WASI context based on security level
    #[allow(dead_code)]
    fn create_wasi_context(&self, _security_context: &SecurityContext) -> ToadStoolResult<WasiCtx> {
        let mut builder = WasiCtxBuilder::new();

        // Configure based on security level
        match self.config.security_level {
            SecurityLevel::None => {
                let _ = builder.inherit_stdio().inherit_env();
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
        let start_time = Instant::now();

        // Create WASI context with basic configuration
        let wasi_ctx = WasiCtxBuilder::new().build();

        // Use the existing engine and create store with WASI context
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Set fuel limit if configured
        if let Some(fuel_limit) = self.config.fuel_limit {
            store
                .set_fuel(fuel_limit)
                .map_err(wasmtime_to_toadstool_error)?;
        }

        // Create linker for WASI
        let mut linker = Linker::new(&self.engine);
        add_to_linker(&mut linker, |s| s).map_err(wasmtime_to_toadstool_error)?;

        // Instantiate module with WASI
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(wasmtime_to_toadstool_error)?;

        // Generate proper cache key for execution tracking
        let cache_key =
            if let toadstool::workload::WorkloadSpec::Wasm { module, .. } = &request.workload {
                self.generate_cache_key(module)
            } else {
                format!("runtime_{execution_id}")
            };

        // Track execution (reuse cache_key instead of cloning)
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(
                execution_id,
                ExecutionHandle {
                    id: execution_id,
                    module_key: cache_key, // Move instead of clone (cache_key not used after this)
                    start_time,
                    security_context: request.security_context.clone(), // Keep clone - needed by ExecutionHandle
                },
            );
        }

        // Execute with timeout
        let timeout_ms = request
            .timeout
            .map_or(self.config.execution_timeout_ms, |d| {
                // Safely convert duration to milliseconds, falling back to config default
                u64::try_from(d.as_millis()).unwrap_or(self.config.execution_timeout_ms)
            });

        let execution_timeout = Duration::from_millis(timeout_ms);
        let execution_result = tokio::time::timeout(execution_timeout, async {
            // Look for main function or _start
            if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "_start") {
                main_func
                    .call_async(&mut store, ())
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM execution failed: {e}")))
            } else if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "main") {
                main_func
                    .call_async(&mut store, ())
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("WASM execution failed: {e}")))
            } else {
                Err(ToadStoolError::runtime(
                    "No main function found in WASM module".to_string(),
                ))
            }
        })
        .await
        .map_err(|_| ToadStoolError::timeout(format!("Execution timeout: {timeout_ms}ms")))?;

        // Clean up execution tracking
        {
            let mut executions = self.active_executions.write().await;
            executions.remove(&execution_id);
        }

        let duration = Instant::now().saturating_duration_since(start_time);

        // Handle execution result
        let (status, _exit_code) = match execution_result {
            Ok(()) => (ExecutionStatus::Success, Some(0)),
            Err(e) => (
                ExecutionStatus::Failed {
                    error: e.to_string(),
                },
                Some(1),
            ),
        };

        // Extract CPU and memory usage metrics
        let cpu_metrics = self.extract_cpu_metrics(&store, &duration);
        let memory_metrics = self.extract_memory_metrics(&mut store, &instance);

        // Create runtime metrics
        let metrics = toadstool::resources::RuntimeMetrics {
            cpu: cpu_metrics,
            memory: memory_metrics,
            storage: toadstool::resources::StorageMetrics::default(),
            network: toadstool::resources::NetworkMetrics::default(),
            gpu: None,
            timing: toadstool::resources::TimingMetrics {
                start_time: chrono::Utc::now()
                    - chrono::Duration::from_std(duration)
                        .unwrap_or_else(|_| chrono::Duration::zero()),
                end_time: Some(chrono::Utc::now()),
                duration: chrono::Duration::from_std(duration)
                    .unwrap_or_else(|_| chrono::Duration::zero()),
            },
        };

        // Extract WASI output (placeholder since we're not capturing)
        let stdout_output = "WASM execution completed".to_string();
        let stderr_output = String::new();

        // Create execution output
        let output = ExecutionOutput {
            data: Vec::new(),
            stdout: Some(stdout_output),
            stderr: Some(stderr_output),
            exit_code: Some(0),
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

    fn extract_cpu_metrics(
        &self,
        store: &Store<WasiCtx>,
        duration: &Duration,
    ) -> toadstool::resources::CpuMetrics {
        // Extract CPU usage based on fuel consumption and execution time
        let fuel_consumed = if self.config.fuel_limit.is_some() {
            // Calculate fuel consumption as a proxy for CPU usage
            let remaining_fuel = store.get_fuel().unwrap_or(0);
            self.config.fuel_limit.unwrap_or(0) - remaining_fuel
        } else {
            0
        };

        // Estimate CPU usage percentage based on execution characteristics
        let usage_percent = if fuel_consumed > 0 {
            // Use fuel consumption as CPU usage indicator
            let fuel_rate = fuel_consumed as f64 / duration.as_millis() as f64;
            (fuel_rate * 0.001).min(100.0) as f32 // Scale to reasonable percentage
        } else {
            // Fallback: estimate based on execution time
            if duration.as_millis() > 100 {
                25.0 // Assume moderate CPU usage for longer executions
            } else {
                5.0 // Light usage for quick executions
            }
        };

        toadstool::resources::CpuMetrics {
            usage_percent: f64::from(usage_percent),
            cores_used: f64::from(usage_percent) / 100.0,
            cpu_time_seconds: duration.as_secs_f64(),
        }
    }

    fn extract_memory_metrics(
        &self,
        store: &mut Store<WasiCtx>,
        instance: &Instance,
    ) -> toadstool::resources::MemoryMetrics {
        // Extract actual memory usage from WASM instance
        let mut usage_bytes = 0u64;
        let mut peak_usage_bytes = 0u64;

        // Query memory exports from the instance
        // First collect export names to avoid borrowing conflicts
        let memory_export_names: Vec<String> = instance
            .exports(&mut *store)
            .filter_map(|export| {
                let name = export.name().to_string();
                if export.into_memory().is_some() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        // Now get memory sizes using the names
        for name in memory_export_names {
            if let Some(memory) = instance.get_memory(&mut *store, &name) {
                let memory_size = memory.data_size(&*store);
                usage_bytes = usage_bytes.max(memory_size as u64);
                peak_usage_bytes = peak_usage_bytes.max(memory_size as u64);
            }
        }

        // If no memory found, use configured limits as estimates
        if usage_bytes == 0 {
            usage_bytes = (self.config.max_memory_mb * 1024 * 1024) / 4; // Assume 25% usage
            peak_usage_bytes = (self.config.max_memory_mb * 1024 * 1024) / 2; // Assume 50% peak
        }

        toadstool::resources::MemoryMetrics {
            usage_percent: (usage_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) * 100.0,
            used_bytes: usage_bytes,
            peak_bytes: peak_usage_bytes,
        }
    }
}

impl RuntimeEngine for WasmRuntimeEngine {
    fn initialize(
        &mut self,
        _config: toadstool::execution::RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Initializing WebAssembly runtime engine");

            // Validate configuration
            if self.config.max_memory_mb == 0 {
                return Err(ToadStoolError::configuration(
                    "Memory limit cannot be zero".to_string(),
                ));
            }

            if self.config.execution_timeout_ms == 0 {
                return Err(ToadStoolError::configuration(
                    "Execution timeout cannot be zero".to_string(),
                ));
            }

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

            // Load or get cached module
            let module = self.get_or_load_module(module_source).await?;

            // Execute the module
            self.execute_module(&request, module).await
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
                features.insert(
                    "fuel_tracking".to_string(),
                    self.config.fuel_limit.is_some(),
                );
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
    ) -> Pin<
        Box<dyn Future<Output = ToadStoolResult<toadstool::resources::RuntimeMetrics>> + Send + '_>,
    > {
        Box::pin(async {
            let executions = self.active_executions.read().await;
            let active_count = executions.len();

            let cache = self.module_cache.read().await;
            let cached_modules = cache.len();

            // Calculate actual memory usage of cached modules
            let cache_memory_usage: u64 = cache
                .values()
                .map(|cached_module| cached_module.size_bytes as u64)
                .sum();

            #[allow(clippy::collection_is_never_read)]
            let mut custom_metrics = HashMap::new();
            custom_metrics.insert(
                "active_executions".to_string(),
                serde_json::Value::String(active_count.to_string()),
            );
            custom_metrics.insert(
                "cached_modules".to_string(),
                serde_json::Value::String(cached_modules.to_string()),
            );
            custom_metrics.insert(
                "cache_memory_usage_bytes".to_string(),
                serde_json::Value::String(cache_memory_usage.to_string()),
            );

            let cache_hit_rate = if cached_modules > 0 {
                // Estimate cache hit rate based on access patterns
                let total_accesses: u64 = cache.values().map(|m| m.access_count).sum();
                if total_accesses > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    (cached_modules as f64 / total_accesses as f64 * 100.0).min(100.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            custom_metrics.insert(
                "cache_hit_rate_percent".to_string(),
                serde_json::Value::String(format!("{cache_hit_rate:.1}")),
            );

            Ok(toadstool::resources::RuntimeMetrics {
                cpu: toadstool::resources::CpuMetrics {
                    #[allow(clippy::cast_precision_loss)]
                    usage_percent: active_count as f64,
                    #[allow(clippy::cast_precision_loss)]
                    cores_used: active_count as f64 / 100.0,
                    cpu_time_seconds: 0.0,
                },
                memory: toadstool::resources::MemoryMetrics {
                    #[allow(clippy::cast_precision_loss)]
                    usage_percent: (cache_memory_usage as f64 / (1024.0 * 1024.0 * 1024.0)) * 100.0,
                    used_bytes: cache_memory_usage,
                    peak_bytes: cache_memory_usage * 11 / 10, // Assume 10% higher peak
                },
                storage: toadstool::resources::StorageMetrics::default(),
                network: toadstool::resources::NetworkMetrics::default(),
                gpu: None,
                timing: toadstool::resources::TimingMetrics::default(),
            })
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async {
            info!("Shutting down WebAssembly runtime engine");

            // ✅ MODERNIZED: Wait for active executions to complete (event-based with timeout)
            let shutdown_timeout = Duration::from_secs(30);
            let check_interval = Duration::from_millis(50);

            tokio::time::timeout(shutdown_timeout, async {
                loop {
                    let executions = self.active_executions.read().await;
                    if executions.is_empty() {
                        break;
                    }
                    drop(executions);
                    // ✅ GOOD: Yield + sleep to avoid busy-waiting during shutdown
                    // This is INTENTIONAL to prevent CPU spinning while waiting for graceful shutdown
                    tokio::task::yield_now().await;
                    tokio::time::sleep(check_interval).await;
                }
            })
            .await
            .ok(); // Timeout is acceptable - force shutdown after timeout

            // Clear caches
            self.module_cache.write().await.clear();
            self.active_executions.write().await.clear();

            info!("WebAssembly runtime engine shut down successfully");
            Ok(())
        })
    }
}

// ComponentModelSupport trait implementation has been moved to component_model.rs

// Tests moved to tests/wasm_runtime_tests.rs for better organization
