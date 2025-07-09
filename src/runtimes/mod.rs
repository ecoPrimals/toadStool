//! # Runtime Manager
//!
//! Manages different execution runtimes for services.

use crate::manifest::ServiceConfig;
use crate::security::SecurityContext;
use crate::resources::ResourceAllocation;

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub mod wasm;
pub mod container;
pub mod native;
pub mod python;

use wasm::WasmRuntime;
use container::ContainerRuntime;
use native::NativeRuntime;
use python::PythonRuntime;

/// Runtime-specific errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("WASM runtime error: {0}")]
    Wasm(#[from] wasm::WasmError),
    
    #[error("Container runtime error: {0}")]
    Container(#[from] container::ContainerError),
    
    #[error("Native runtime error: {0}")]
    Native(#[from] native::NativeError),
    
    #[error("Python runtime error: {0}")]
    Python(#[from] python::PythonError),
    
    #[error("Unsupported runtime: {runtime}")]
    UnsupportedRuntime { runtime: String },
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: Uuid },
    
    #[error("Runtime initialization failed: {reason}")]
    InitializationFailed { reason: String },
    
    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },
}

/// Runtime execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: std::time::Duration,
    pub resource_usage: RuntimeResourceUsage,
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceUsage {
    pub cpu_time: std::time::Duration,
    pub memory_peak: u64,
    pub memory_average: u64,
    pub disk_read: u64,
    pub disk_write: u64,
    pub network_rx: u64,
    pub network_tx: u64,
}

/// Active task information
#[derive(Debug)]
struct ActiveTask {
    pub task_id: Uuid,
    pub service_name: String,
    pub runtime_type: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub handle: TaskHandle,
}

/// Task handle for different runtime types
#[derive(Debug)]
enum TaskHandle {
    Wasm(wasm::WasmTaskHandle),
    Container(container::ContainerTaskHandle),
    Native(native::NativeTaskHandle),
    Python(python::PythonTaskHandle),
}

/// Main runtime manager
pub struct RuntimeManager {
    wasm_runtime: Arc<WasmRuntime>,
    container_runtime: Arc<ContainerRuntime>,
    native_runtime: Arc<NativeRuntime>,
    python_runtime: Arc<PythonRuntime>,
    
    // Active tasks
    active_tasks: Arc<RwLock<HashMap<Uuid, ActiveTask>>>,
    
    // Configuration
    max_concurrent_tasks: usize,
    default_timeout: std::time::Duration,
}

impl RuntimeManager {
    pub async fn new() -> Result<Self, RuntimeError> {
        info!("Initializing runtime manager");
        
        // Initialize all runtimes
        let wasm_runtime = Arc::new(WasmRuntime::new().await?);
        let container_runtime = Arc::new(ContainerRuntime::new().await?);
        let native_runtime = Arc::new(NativeRuntime::new().await?);
        let python_runtime = Arc::new(PythonRuntime::new().await?);
        
        Ok(Self {
            wasm_runtime,
            container_runtime,
            native_runtime,
            python_runtime,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_tasks: 50,
            default_timeout: std::time::Duration::from_secs(300),
        })
    }

    /// Execute a service using WASM runtime
    pub async fn execute_wasm(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<i32, RuntimeError> {
        info!("Executing WASM service: {} ({})", service.name, task_id);
        
        // Check concurrent task limit
        self.check_task_limit().await?;
        
        // Execute the service
        let handle = self.wasm_runtime.execute(
            task_id,
            service,
            security_context,
            resource_allocation,
        ).await?;
        
        // Store active task
        let active_task = ActiveTask {
            task_id,
            service_name: service.name.clone(),
            runtime_type: "wasm".to_string(),
            started_at: chrono::Utc::now(),
            handle: TaskHandle::Wasm(handle),
        };
        
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.insert(task_id, active_task);
        }
        
        // Wait for completion
        let result = self.wasm_runtime.wait_for_completion(task_id).await?;
        
        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.remove(&task_id);
        }
        
        info!("WASM service completed: {} (exit code: {})", service.name, result.exit_code);
        
        Ok(result.exit_code)
    }

    /// Execute a service using container runtime
    pub async fn execute_container(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<i32, RuntimeError> {
        info!("Executing container service: {} ({})", service.name, task_id);
        
        // Check concurrent task limit
        self.check_task_limit().await?;
        
        // Execute the service
        let handle = self.container_runtime.execute(
            task_id,
            service,
            security_context,
            resource_allocation,
        ).await?;
        
        // Store active task
        let active_task = ActiveTask {
            task_id,
            service_name: service.name.clone(),
            runtime_type: "container".to_string(),
            started_at: chrono::Utc::now(),
            handle: TaskHandle::Container(handle),
        };
        
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.insert(task_id, active_task);
        }
        
        // Wait for completion
        let result = self.container_runtime.wait_for_completion(task_id).await?;
        
        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.remove(&task_id);
        }
        
        info!("Container service completed: {} (exit code: {})", service.name, result.exit_code);
        
        Ok(result.exit_code)
    }

    /// Execute a service using native runtime
    pub async fn execute_native(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<i32, RuntimeError> {
        info!("Executing native service: {} ({})", service.name, task_id);
        
        // Check concurrent task limit
        self.check_task_limit().await?;
        
        // Execute the service
        let handle = self.native_runtime.execute(
            task_id,
            service,
            security_context,
            resource_allocation,
        ).await?;
        
        // Store active task
        let active_task = ActiveTask {
            task_id,
            service_name: service.name.clone(),
            runtime_type: "native".to_string(),
            started_at: chrono::Utc::now(),
            handle: TaskHandle::Native(handle),
        };
        
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.insert(task_id, active_task);
        }
        
        // Wait for completion
        let result = self.native_runtime.wait_for_completion(task_id).await?;
        
        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.remove(&task_id);
        }
        
        info!("Native service completed: {} (exit code: {})", service.name, result.exit_code);
        
        Ok(result.exit_code)
    }

    /// Execute a service using Python runtime
    pub async fn execute_python(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<i32, RuntimeError> {
        info!("Executing Python service: {} ({})", service.name, task_id);
        
        // Check concurrent task limit
        self.check_task_limit().await?;
        
        // Execute the service
        let handle = self.python_runtime.execute(
            task_id,
            service,
            security_context,
            resource_allocation,
        ).await?;
        
        // Store active task
        let active_task = ActiveTask {
            task_id,
            service_name: service.name.clone(),
            runtime_type: "python".to_string(),
            started_at: chrono::Utc::now(),
            handle: TaskHandle::Python(handle),
        };
        
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.insert(task_id, active_task);
        }
        
        // Wait for completion
        let result = self.python_runtime.wait_for_completion(task_id).await?;
        
        // Remove from active tasks
        {
            let mut tasks = self.active_tasks.write().await;
            tasks.remove(&task_id);
        }
        
        info!("Python service completed: {} (exit code: {})", service.name, result.exit_code);
        
        Ok(result.exit_code)
    }

    /// Stop a task
    pub async fn stop_task(&self, task_id: Uuid, force: bool) -> Result<(), RuntimeError> {
        info!("Stopping task: {} (force: {})", task_id, force);
        
        let task = {
            let tasks = self.active_tasks.read().await;
            tasks.get(&task_id).cloned()
        };
        
        match task {
            Some(active_task) => {
                match &active_task.handle {
                    TaskHandle::Wasm(_) => {
                        self.wasm_runtime.stop_task(task_id, force).await?;
                    }
                    TaskHandle::Container(_) => {
                        self.container_runtime.stop_task(task_id, force).await?;
                    }
                    TaskHandle::Native(_) => {
                        self.native_runtime.stop_task(task_id, force).await?;
                    }
                    TaskHandle::Python(_) => {
                        self.python_runtime.stop_task(task_id, force).await?;
                    }
                }
                
                // Remove from active tasks
                {
                    let mut tasks = self.active_tasks.write().await;
                    tasks.remove(&task_id);
                }
                
                Ok(())
            }
            None => Err(RuntimeError::TaskNotFound { task_id }),
        }
    }

    /// Get active tasks
    pub async fn get_active_tasks(&self) -> Vec<(Uuid, String, String)> {
        let tasks = self.active_tasks.read().await;
        tasks.values()
            .map(|task| (task.task_id, task.service_name.clone(), task.runtime_type.clone()))
            .collect()
    }

    /// Get runtime statistics
    pub async fn get_runtime_stats(&self) -> RuntimeStats {
        let tasks = self.active_tasks.read().await;
        let active_count = tasks.len();
        
        let mut stats_by_runtime = HashMap::new();
        for task in tasks.values() {
            *stats_by_runtime.entry(task.runtime_type.clone()).or_insert(0) += 1;
        }
        
        RuntimeStats {
            total_active_tasks: active_count,
            tasks_by_runtime: stats_by_runtime,
            wasm_stats: self.wasm_runtime.get_stats().await,
            container_stats: self.container_runtime.get_stats().await,
            native_stats: self.native_runtime.get_stats().await,
            python_stats: self.python_runtime.get_stats().await,
        }
    }

    /// Check if we can accept more tasks
    async fn check_task_limit(&self) -> Result<(), RuntimeError> {
        let tasks = self.active_tasks.read().await;
        
        if tasks.len() >= self.max_concurrent_tasks {
            return Err(RuntimeError::ExecutionFailed {
                reason: format!("Maximum concurrent tasks reached: {}", self.max_concurrent_tasks),
            });
        }
        
        Ok(())
    }
}

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub total_active_tasks: usize,
    pub tasks_by_runtime: HashMap<String, usize>,
    pub wasm_stats: wasm::WasmStats,
    pub container_stats: container::ContainerStats,
    pub native_stats: native::NativeStats,
    pub python_stats: python::PythonStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ServiceConfig;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_runtime_manager_creation() {
        let manager = RuntimeManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_get_active_tasks() {
        let manager = RuntimeManager::new().await.unwrap();
        let tasks = manager.get_active_tasks().await;
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_stats() {
        let manager = RuntimeManager::new().await.unwrap();
        let stats = manager.get_runtime_stats().await;
        assert_eq!(stats.total_active_tasks, 0);
    }

    #[tokio::test]
    async fn test_task_limit_check() {
        let manager = RuntimeManager::new().await.unwrap();
        let result = manager.check_task_limit().await;
        assert!(result.is_ok());
    }
} 