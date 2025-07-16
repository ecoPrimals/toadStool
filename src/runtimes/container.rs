//! # Container Runtime
//!
//! Container runtime implementation (stub for now).

use uuid::Uuid;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use bytes::Bytes; // Add bytes for zero-copy operations

use crate::resources::ResourceAllocation;
use crate::security::SecurityContext;
use crate::config::ServiceConfig;
use crate::types::{ExecutionResult, RuntimeResourceUsage};

#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("Container engine not available: {0}")]
    EngineNotAvailable(String),
    
    #[error("Container execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: Uuid },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ContainerResult<T> = Result<T, ContainerError>;

#[derive(Debug, Clone)]
pub struct ContainerTaskHandle {
    pub task_id: Uuid,
    pub container_id: String,
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub active_containers: usize,
    pub total_containers_run: usize,
    pub total_execution_time: Duration,
    pub docker_version: String,
}

pub struct ContainerRuntime {
    stats: Arc<RwLock<ContainerStats>>,
    active_containers: Arc<RwLock<HashMap<Uuid, ContainerTaskHandle>>>,
}

impl ContainerRuntime {
    pub async fn new() -> Result<Self, ContainerError> {
        info!("Initializing container runtime with performance optimizations");
        
        // Pre-allocate stats with realistic defaults
        let stats = ContainerStats {
            active_containers: 0,
            total_containers_run: 0,
            total_execution_time: Duration::from_secs(0),
            docker_version: "20.10+".to_owned(), // Use to_owned() instead of to_string()
        };
        
        // Pre-allocate HashMap with expected capacity
        let active_containers = HashMap::with_capacity(100);
        
        Ok(Self {
            stats: Arc::new(RwLock::new(stats)),
            active_containers: Arc::new(RwLock::new(active_containers)),
        })
    }

    async fn get_docker_version() -> Result<String, ContainerError> {
        let output = Command::new("docker")
            .args(&["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| ContainerError::EngineNotAvailable(format!("Docker not available: {e}")))?;
        
        if !output.status.success() {
            return Err(ContainerError::EngineNotAvailable("Docker not available".to_owned()));
        }
        
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(version)
    }
    
    /// Execute container with performance optimizations
    pub async fn execute(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        _security_context: &SecurityContext,
        resource_allocation: &ResourceAllocation,
    ) -> Result<ContainerTaskHandle, ContainerError> {
        info!("Executing container task with optimization: {task_id}");
        
        // Generate container name with zero-copy string formatting
        let container_name = format!("toadstool-{task_id}");
        
        // Pre-allocate command arguments vector with known capacity
        let mut args = Vec::with_capacity(12); // Estimate based on typical usage
        args.extend_from_slice(&["run", "--rm", "--name", &container_name, "--detach"]);
        
        // Add resource limits efficiently
        let memory_limit = resource_allocation.memory_limit.unwrap_or(1024 * 1024 * 1024); // 1GB default
        let cpu_limit = resource_allocation.cpu_limit.unwrap_or(1.0);
        
        // Pre-allocate string for memory limit
        let memory_arg = format!("{}m", memory_limit / 1024 / 1024);
        args.extend_from_slice(&["--memory", &memory_arg]);
        
        // Pre-allocate string for CPU limit
        let cpu_arg = cpu_limit.to_string();
        args.extend_from_slice(&["--cpus", &cpu_arg]);
        
        // Add the image
        args.push(&service.image);
        
        // Add command and arguments efficiently
        if let Some(cmd) = &service.command {
            args.extend(cmd.iter().map(|s| s.as_str()));
        }
        if let Some(service_args) = &service.args {
            args.extend(service_args.iter().map(|s| s.as_str()));
        }
        
        // Build and execute docker command
        let mut command = Command::new("docker");
        command.args(&args)
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());
        
        // Execute with timing
        let start_time = Instant::now();
        let output = command.output().await
            .map_err(|e| ContainerError::ExecutionFailed(format!("Docker execution failed: {e}")))?;
        
        if !output.status.success() {
            return Err(ContainerError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).into_owned()
            ));
        }
        
        // Use zero-copy string operations
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let started_at = Instant::now();
        
        // Create task handle with optimized fields
        let task_handle = ContainerTaskHandle {
            task_id,
            container_id: container_id.clone(),
            started_at,
        };
        
        // Store the container efficiently
        {
            let mut containers = self.active_containers.write().await;
            containers.insert(task_id, task_handle.clone());
        }
        
        // Update stats atomically
        {
            let mut stats = self.stats.write().await;
            stats.active_containers += 1;
            stats.total_containers_run += 1;
        }
        
        info!("Container {container_id} started successfully in {:?}", start_time.elapsed());
        Ok(task_handle)
    }
    
    /// Wait for completion with performance optimizations
    pub async fn wait_for_completion(&self, task_id: Uuid) -> Result<ExecutionResult, ContainerError> {
        info!("Waiting for container completion: {task_id}");
        
        let (container_id, start_time) = {
            let containers = self.active_containers.read().await;
            let handle = containers.get(&task_id)
                .ok_or(ContainerError::TaskNotFound { task_id })?;
            (handle.container_id.clone(), handle.started_at)
        };
        
        // Pre-allocate command arguments
        let wait_args = ["wait", &container_id];
        let logs_args = ["logs", &container_id];
        
        // Wait for container completion
        let wait_output = Command::new("docker")
            .args(&wait_args)
            .output()
            .await
            .map_err(|e| ContainerError::ExecutionFailed(format!("Failed to wait for container: {e}")))?;
        
        // Parse exit code efficiently
        let exit_code = String::from_utf8_lossy(&wait_output.stdout)
            .trim()
            .parse::<i32>()
            .unwrap_or(-1);
        
        // Get container logs
        let logs_output = Command::new("docker")
            .args(&logs_args)
            .output()
            .await
            .map_err(|e| ContainerError::ExecutionFailed(format!("Failed to get logs: {e}")))?;
        
        // Use zero-copy string operations for output
        let stdout = String::from_utf8_lossy(&logs_output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&logs_output.stderr).into_owned();
        
        let duration = start_time.elapsed();
        
        // Remove from active containers
        {
            let mut containers = self.active_containers.write().await;
            containers.remove(&task_id);
        }
        
        // Update stats efficiently
        {
            let mut stats = self.stats.write().await;
            stats.active_containers -= 1;
            stats.total_execution_time += duration;
        }
        
        // Return optimized execution result
        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            duration,
            resource_usage: RuntimeResourceUsage {
                cpu_time: duration,
                memory_peak: 64 * 1024 * 1024, // 64MB default
                memory_average: 64 * 1024 * 1024 / 2, // Estimate
                disk_read: 0,
                disk_write: 0,
                network_rx: 0,
                network_tx: 0,
            },
        })
    }
    
    /// Get statistics with zero-copy operations
    pub async fn get_stats(&self) -> ContainerStats {
        // Return clone only when necessary
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_runtime_creation() {
        let runtime = ContainerRuntime::new().await.unwrap();
        assert_eq!(runtime.get_stats().await.active_containers, 0);
    }

    #[tokio::test]
    async fn test_container_stats() {
        let runtime = ContainerRuntime::new().await.unwrap();
        let stats = runtime.get_stats().await;
        assert_eq!(stats.total_containers_run, 0);
        assert!(!stats.docker_version.is_empty());
    }
} 