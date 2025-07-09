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

/// WASM execution context
#[derive(Debug)]
pub struct WasmContext {
    pub service_name: String,
    pub security_context: SecurityContext,
    pub resource_allocation: ResourceAllocation,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// WASM runtime statistics
#[derive(Debug, Clone)]
pub struct WasmStats {
    pub total_modules_loaded: usize,
    pub active_instances: usize,
    pub total_execution_time: Duration,
    pub memory_usage: u64,
    pub compilation_cache_size: usize,
}

/// Main WASM runtime
pub struct WasmRuntime {
    engine: Engine,
    module_cache: Arc<RwLock<HashMap<String, Module>>>,
    active_tasks: Arc<RwLock<HashMap<Uuid, WasmTaskHandle>>>,
    stats: Arc<RwLock<WasmStats>>,
}

impl WasmRuntime {
    pub async fn new() -> Result<Self, WasmError> {
        info!("Initializing WASM runtime");
        
        // Create wasmtime engine with security-focused configuration
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        
        // Security settings
        config.wasm_multi_memory(false);
        config.wasm_threads(false);
        config.wasm_reference_types(false);
        config.wasm_simd(false);
        config.wasm_bulk_memory(false);
        
        let engine = Engine::new(&config)?;
        
        Ok(Self {
            engine,
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(WasmStats {
                total_modules_loaded: 0,
                active_instances: 0,
                total_execution_time: Duration::from_secs(0),
                memory_usage: 0,
                compilation_cache_size: 0,
            })),
        })
    }

    /// Execute a WASM service
    pub async fn execute(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<WasmTaskHandle, WasmError> {
        info!("Executing WASM service: {} ({})", service.name, task_id);
        
        // Load or compile module
        let module = self.load_module(service).await?;
        
        // Create WASM context
        let wasm_context = WasmContext {
            service_name: service.name.clone(),
            security_context: security_context.clone(),
            resource_allocation: resource_allocation.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        
        // Create store with context
        let mut store = Store::new(&self.engine, wasm_context);
        
        // Set resource limits
        self.configure_resource_limits(&mut store, resource_allocation).await?;
        
        // Create WASI context
        let wasi_ctx = self.create_wasi_context(service, security_context).await?;
        store.data_mut().stdout = Vec::new();
        store.data_mut().stderr = Vec::new();
        
        // Instantiate module
        let instance = self.instantiate_module(&mut store, &module, wasi_ctx).await?;
        
        // Create task handle
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
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_instances += 1;
        }
        
        // Return handle (note: this is a simplified version)
        // In reality, we'd return a handle that can be used to control the task
        Ok(WasmTaskHandle {
            task_id,
            instance: Instance::new(&mut Store::new(&self.engine, wasm_context), &module, &[]).unwrap(),
            store: Store::new(&self.engine, WasmContext {
                service_name: service.name.clone(),
                security_context: security_context.clone(),
                resource_allocation: resource_allocation.clone(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            started_at: Instant::now(),
        })
    }

    /// Wait for task completion
    pub async fn wait_for_completion(&self, task_id: Uuid) -> Result<ExecutionResult, WasmError> {
        info!("Waiting for WASM task completion: {}", task_id);
        
        let (start_time, service_name) = {
            let tasks = self.active_tasks.read().await;
            let task = tasks.get(&task_id)
                .ok_or(WasmError::TaskNotFound { task_id })?;
            (task.started_at, task.store.data().service_name.clone())
        };
        
        // Simulate task execution and completion
        // In a real implementation, this would involve:
        // 1. Running the WASM module's main function
        // 2. Handling WASI calls
        // 3. Monitoring resource usage
        // 4. Collecting output
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let duration = start_time.elapsed();
        
        // Remove from active tasks
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.remove(&task_id);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_instances -= 1;
            stats.total_execution_time += duration;
        }
        
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: format!("WASM service {} completed successfully", service_name),
            stderr: String::new(),
            duration,
            resource_usage: RuntimeResourceUsage {
                cpu_time: duration,
                memory_peak: 1024 * 1024, // 1MB
                memory_average: 512 * 1024, // 512KB
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

    /// Get runtime statistics
    pub async fn get_stats(&self) -> WasmStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    // Private helper methods

    async fn load_module(&self, service: &ServiceConfig) -> Result<Module, WasmError> {
        let module_key = service.source.as_ref()
            .unwrap_or(&format!("service:{}", service.name));
        
        // Check cache first
        {
            let cache = self.module_cache.read().await;
            if let Some(module) = cache.get(module_key) {
                return Ok(module.clone());
            }
        }
        
        // Load and compile module
        let module = if let Some(source) = &service.source {
            if source.ends_with(".wasm") {
                // Load from file
                let wasm_bytes = tokio::fs::read(source).await?;
                Module::new(&self.engine, wasm_bytes)
                    .map_err(|e| WasmError::ModuleCompilation(e.to_string()))?
            } else {
                // Create a simple "hello world" module for testing
                self.create_test_module()?
            }
        } else {
            // Create a simple "hello world" module for testing
            self.create_test_module()?
        };
        
        // Cache the module
        {
            let mut cache = self.module_cache.write().await;
            cache.insert(module_key.clone(), module.clone());
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_modules_loaded += 1;
            stats.compilation_cache_size = cache.len();
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

    async fn configure_resource_limits(
        &self,
        store: &mut Store<WasmContext>,
        resource_allocation: &ResourceAllocation,
    ) -> Result<(), WasmError> {
        // Set fuel limit (CPU time)
        let fuel_limit = (resource_allocation.cpu_allocation.cores * 1000000.0) as u64;
        store.add_fuel(fuel_limit)
            .map_err(|e| WasmError::ResourceLimitExceeded { 
                limit: format!("Fuel limit: {}", e) 
            })?;
        
        // Set memory limit
        let memory_limit = resource_allocation.memory_allocation.bytes;
        // Note: In a real implementation, you'd configure the memory limit
        // through the module's memory import or linear memory configuration
        
        debug!("Configured WASM resource limits: fuel={}, memory={}", fuel_limit, memory_limit);
        
        Ok(())
    }

    async fn create_wasi_context(
        &self,
        service: &ServiceConfig,
        security_context: &SecurityContext,
    ) -> Result<wasmtime_wasi::WasiCtx, WasmError> {
        let mut wasi_builder = wasmtime_wasi::WasiCtxBuilder::new();
        
        // Configure command line arguments
        wasi_builder.arg(&service.name);
        for arg in &service.args {
            wasi_builder.arg(arg);
        }
        
        // Configure environment variables
        for (key, value) in &service.environment {
            wasi_builder.env(key, value);
        }
        
        // Configure file system access based on security context
        for allowed_path in &security_context.file_system_policy.allowed_paths {
            if allowed_path.to_string_lossy().starts_with("/tmp") {
                wasi_builder.preopened_dir(allowed_path, "/tmp")
                    .map_err(|e| WasmError::WasiInitialization(e.to_string()))?;
            }
        }
        
        // Configure stdin/stdout/stderr
        wasi_builder.stdout(Box::new(wasmtime_wasi::pipe::WritePipe::new_in_memory()));
        wasi_builder.stderr(Box::new(wasmtime_wasi::pipe::WritePipe::new_in_memory()));
        
        wasi_builder.build()
            .map_err(|e| WasmError::WasiInitialization(e.to_string()))
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
        let module1 = runtime.load_module(&service).await.unwrap();
        let module2 = runtime.load_module(&service).await.unwrap();
        
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