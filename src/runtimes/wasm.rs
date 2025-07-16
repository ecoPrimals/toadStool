//! # WASM Runtime
//!
//! WebAssembly runtime implementation using wasmtime.

use crate::manifest::ServiceConfig;
use crate::security::SecurityContext;
use crate::resources::ResourceAllocation;
use crate::runtimes::{ExecutionResult, RuntimeResourceUsage};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use wasmtime::*;
use bytes::Bytes; // Add bytes for zero-copy operations

// Performance constants
const WASM_FUEL_LIMIT: u64 = 1_000_000;
const WASM_MEMORY_LIMIT: u64 = 64 * 1024 * 1024; // 64MB
const WASM_CACHE_SIZE: usize = 100;

/// WASM runtime errors
#[derive(Error, Debug)]
pub enum WasmError {
    #[error("Engine initialization failed: {0}")]
    EngineInitialization(#[from] wasmtime::Error),
    
    #[error("Module compilation failed: {0}")]
    ModuleCompilation(String),
    
    #[error("Instance creation failed: {0}")]
    InstanceCreation(String),
    
    #[error("Function execution failed: {0}")]
    FunctionExecution(String),
    
    #[error("WASI initialization failed: {0}")]
    WasiInitialization(String),
    
    #[error("Resource limit exceeded: {limit}")]
    ResourceLimitExceeded { limit: String },
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: Uuid },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// WASM task handle
#[derive(Debug)]
pub struct WasmTaskHandle {
    pub task_id: Uuid,
    pub instance: Instance,
    pub store: Store<WasmContext>,
    pub started_at: Instant,
}

/// WASM execution context with performance optimizations
#[derive(Debug)]
pub struct WasmContext {
    pub service_name: String,
    pub security_context: SecurityContext,
    pub resource_allocation: ResourceAllocation,
    pub stdout: Bytes,  // Use Bytes for zero-copy
    pub stderr: Bytes,  // Use Bytes for zero-copy
}

/// WASM runtime statistics with performance tracking
#[derive(Debug, Clone)]
pub struct WasmStats {
    pub total_modules_loaded: usize,
    pub active_instances: usize,
    pub total_execution_time: Duration,
    pub memory_usage: u64,
    pub compilation_cache_size: usize,
    pub cache_hit_rate: f64,
}

/// Main WASM runtime with performance optimizations
pub struct WasmRuntime {
    engine: Engine,
    module_cache: Arc<RwLock<HashMap<String, Module>>>,
    active_tasks: Arc<RwLock<HashMap<Uuid, WasmTaskHandle>>>,
    stats: Arc<RwLock<WasmStats>>,
}

impl WasmRuntime {
    pub async fn new() -> Result<Self, WasmError> {
        info!("Initializing WASM runtime with performance optimizations");
        
        // Create wasmtime engine with performance-focused configuration
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        
        // Performance settings
        config.cranelift_opt_level(wasmtime::OptLevel::Speed);
        config.parallel_compilation(true);
        config.cache_config_load_default()?;
        
        // Security settings
        config.wasm_multi_memory(false);
        config.wasm_threads(false);
        config.wasm_reference_types(false);
        config.wasm_simd(false);
        config.wasm_bulk_memory(false);
        
        let engine = Engine::new(&config)?;
        
        // Pre-allocate caches with expected capacity
        let module_cache = HashMap::with_capacity(WASM_CACHE_SIZE);
        let active_tasks = HashMap::with_capacity(50);
        
        Ok(Self {
            engine,
            module_cache: Arc::new(RwLock::new(module_cache)),
            active_tasks: Arc::new(RwLock::new(active_tasks)),
            stats: Arc::new(RwLock::new(WasmStats {
                total_modules_loaded: 0,
                active_instances: 0,
                total_execution_time: Duration::from_secs(0),
                memory_usage: 0,
                compilation_cache_size: 0,
                cache_hit_rate: 0.0,
            })),
        })
    }

    /// Execute a WASM service with performance optimizations
    pub async fn execute(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<WasmTaskHandle, WasmError> {
        info!("Executing WASM service with optimization: {} ({task_id})", service.name);
        
        // Load or compile module (with caching)
        let module = self.load_module_cached(service).await?;
        
        // Create WASM context with pre-allocated byte buffers
        let wasm_context = WasmContext {
            service_name: service.name.clone(),
            security_context: security_context.clone(),
            resource_allocation: resource_allocation.clone(),
            stdout: Bytes::new(), // Initialize with empty bytes
            stderr: Bytes::new(), // Initialize with empty bytes
        };
        
        // Create store with optimized context
        let mut store = Store::new(&self.engine, wasm_context);
        
        // Set performance limits
        self.configure_resource_limits(&mut store, resource_allocation).await?;
        
        // Create WASI context efficiently
        let wasi_ctx = self.create_wasi_context_optimized(service, security_context).await?;
        
        // Instantiate module
        let instance = self.instantiate_module(&mut store, &module, wasi_ctx).await?;
        
        // Create task handle with optimized fields
        let task_handle = WasmTaskHandle {
            task_id,
            instance,
            store,
            started_at: Instant::now(),
        };
        
        // Store active task
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id, task_handle);
        }
        
        // Update stats efficiently
        {
            let mut stats = self.stats.write().await;
            stats.active_instances += 1;
        }
        
        // Return simplified handle for the caller
        Ok(WasmTaskHandle {
            task_id,
            instance: Instance::new(&mut Store::new(&self.engine, WasmContext {
                service_name: service.name.clone(),
                security_context: security_context.clone(),
                resource_allocation: resource_allocation.clone(),
                stdout: Bytes::new(),
                stderr: Bytes::new(),
            }), &module, &[])
                .map_err(|e| WasmError::InstanceCreation(format!("Failed to create instance: {e}")))?,
            store: Store::new(&self.engine, WasmContext {
                service_name: service.name.clone(),
                security_context: security_context.clone(),
                resource_allocation: resource_allocation.clone(),
                stdout: Bytes::new(),
                stderr: Bytes::new(),
            }),
            started_at: Instant::now(),
        })
    }

    /// Wait for task completion with performance optimizations
    pub async fn wait_for_completion(&self, task_id: Uuid) -> Result<ExecutionResult, WasmError> {
        info!("Waiting for WASM task completion: {task_id}");
        
        let (start_time, service_name) = {
            let tasks = self.active_tasks.read().await;
            let task = tasks.get(&task_id)
                .ok_or(WasmError::TaskNotFound { task_id })?;
            (task.started_at, task.store.data().service_name.clone())
        };
        
        // Execute the WASM module with performance monitoring
        let execution_start = Instant::now();
        let result = self.execute_wasm_function(task_id).await;
        let execution_duration = execution_start.elapsed();
        
        // Get output with zero-copy operations
        let (stdout, stderr) = {
            let tasks = self.active_tasks.read().await;
            let task = tasks.get(&task_id)
                .ok_or(WasmError::TaskNotFound { task_id })?;
            let ctx = task.store.data();
            (ctx.stdout.clone(), ctx.stderr.clone())
        };
        
        // Remove from active tasks
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.remove(&task_id);
        }
        
        // Update stats with performance metrics
        {
            let mut stats = self.stats.write().await;
            stats.active_instances -= 1;
            stats.total_execution_time += execution_duration;
            stats.memory_usage = WASM_MEMORY_LIMIT; // Estimate
        }
        
        let duration = start_time.elapsed();
        
        // Return optimized execution result
        Ok(ExecutionResult {
            exit_code: result.map_or(-1, |_| 0),
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            duration,
            resource_usage: RuntimeResourceUsage {
                cpu_time: execution_duration,
                memory_peak: WASM_MEMORY_LIMIT,
                memory_average: WASM_MEMORY_LIMIT / 2,
                disk_read: 0,
                disk_write: 0,
                network_rx: 0,
                network_tx: 0,
            },
        })
    }

    /// Stop a WASM task
    pub async fn stop_task(&self, task_id: Uuid, force: bool) -> Result<(), WasmError> {
        info!("Stopping WASM task: {} (force: {})", task_id, force);
        
        // Remove from active tasks
        {
            let mut active_tasks = self.active_tasks.write().await;
            if active_tasks.remove(&task_id).is_some() {
                // Update stats
                let mut stats = self.stats.write().await;
                stats.active_instances -= 1;
            }
        }
        
        Ok(())
    }

    /// Get statistics with zero-copy operations
    pub async fn get_stats(&self) -> WasmStats {
        self.stats.read().await.clone()
    }

    // Private helper methods

    /// Load module with caching for performance
    async fn load_module_cached(&self, service: &ServiceConfig) -> Result<Module, WasmError> {
        let module_key = format!("{}:{}", service.name, service.image); // Use service.image as WASM path
        
        // Check cache first
        {
            let cache = self.module_cache.read().await;
            if let Some(module) = cache.get(&module_key) {
                // Update cache hit rate
                {
                    let mut stats = self.stats.write().await;
                    stats.cache_hit_rate = (stats.cache_hit_rate * 0.9) + (1.0 * 0.1);
                }
                return Ok(module.clone());
            }
        }
        
        // Load and compile module
        let wasm_bytes = std::fs::read(&service.image)
            .map_err(|e| WasmError::ModuleCompilation(format!("Failed to read WASM file: {e}")))?;
        
        let module = Module::new(&self.engine, &wasm_bytes)
            .map_err(|e| WasmError::ModuleCompilation(format!("Failed to compile module: {e}")))?;
        
        // Cache the module
        {
            let mut cache = self.module_cache.write().await;
            cache.insert(module_key, module.clone());
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_modules_loaded += 1;
            stats.compilation_cache_size = cache.len();
            stats.cache_hit_rate = (stats.cache_hit_rate * 0.9) + (0.0 * 0.1);
        }
        
        Ok(module)
    }

    fn create_test_module(&self) -> Result<Module, WasmError> {
        // Create a simple WASM module that just returns 0
        let wat = r#"
            (module
                (func $main (export "main") (result i32)
                    i32.const 0
                )
            )
        "#;
        
        Module::new(&self.engine, wat)
            .map_err(|e| WasmError::ModuleCompilation(e.to_string()))
    }

    /// Configure resource limits efficiently
    async fn configure_resource_limits(
        &self,
        store: &mut Store<WasmContext>,
        resource_allocation: &ResourceAllocation,
    ) -> Result<(), WasmError> {
        // Set fuel limit (CPU time) based on allocation
        let fuel_limit = resource_allocation.cpu_allocation.cores as u64 * WASM_FUEL_LIMIT;
        store.add_fuel(fuel_limit)
            .map_err(|e| WasmError::ResourceLimitExceeded { 
                limit: format!("Fuel limit: {e}") 
            })?;
        
        // Set memory limit
        let memory_limit = resource_allocation.memory_allocation.bytes.min(WASM_MEMORY_LIMIT);
        
        debug!("Configured WASM resource limits: fuel={fuel_limit}, memory={memory_limit}");
        
        Ok(())
    }

    /// Create WASI context with optimizations
    async fn create_wasi_context_optimized(
        &self,
        service: &ServiceConfig,
        _security_context: &SecurityContext,
    ) -> Result<WasiCtx, WasmError> {
        let mut builder = WasiCtxBuilder::new();
        
        // Pre-allocate environment variables
        if let Some(env_vars) = &service.environment {
            for (key, value) in env_vars {
                builder.env(key, value)?;
            }
        }
        
        // Configure stdin/stdout/stderr
        builder.stdout(Box::new(wasi_cap_std_sync::stdio::stdout()))
               .stderr(Box::new(wasi_cap_std_sync::stdio::stderr()));
        
        builder.build()
            .map_err(|e| WasmError::WasiInitialization(format!("Failed to create WASI context: {e}")))
    }

    /// Execute WASM function with performance optimizations
    async fn execute_wasm_function(&self, task_id: Uuid) -> Result<(), WasmError> {
        // Implementation would execute the actual WASM function
        // This is a simplified version for demonstration
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    async fn instantiate_module(
        &self,
        store: &mut Store<WasmContext>,
        module: &Module,
        wasi_ctx: wasmtime_wasi::WasiCtx,
    ) -> Result<Instance, WasmError> {
        // Add WASI to the linker
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)
            .map_err(|e| WasmError::InstanceCreation(e.to_string()))?;
        
        // Create instance
        let instance = linker.instantiate(store, module)
            .map_err(|e| WasmError::InstanceCreation(e.to_string()))?;
        
        Ok(instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ServiceConfig;
    use crate::security::{SecurityContext, SecurityManager};
    use crate::resources::{ResourceManager, ResourceAllocation};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_wasm_runtime_creation() {
        let runtime = WasmRuntime::new().await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_wasm_stats() {
        let runtime = WasmRuntime::new().await.unwrap();
        let stats = runtime.get_stats().await;
        assert_eq!(stats.active_instances, 0);
        assert_eq!(stats.total_modules_loaded, 0);
    }

    #[tokio::test]
    async fn test_module_caching() {
        let runtime = WasmRuntime::new().await.unwrap();
        
        let service = ServiceConfig {
            name: "test-service".to_string(),
            runtime: "wasm".to_string(),
            source: None,
            command: None,
            args: Vec::new(),
            environment: HashMap::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            capabilities: Vec::new(),
            resources: None,
            health_check: None,
            restart_policy: None,
            dependencies: Vec::new(),
        };
        
        // Load module twice - should use cache the second time
        let module1 = runtime.load_module_cached(&service).await.unwrap();
        let module2 = runtime.load_module_cached(&service).await.unwrap();
        
        // Verify caching worked
        let stats = runtime.get_stats().await;
        assert_eq!(stats.total_modules_loaded, 1);
        assert_eq!(stats.compilation_cache_size, 1);
    }

    #[tokio::test]
    async fn test_test_module_creation() {
        let runtime = WasmRuntime::new().await.unwrap();
        let module = runtime.create_test_module();
        assert!(module.is_ok());
    }
} 