//! # Native Runtime
//!
//! Native process runtime implementation.

use uuid::Uuid;
use std::collections::HashMap;
use std::process::{Command, Child, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{info, warn, error};

use crate::resources::ResourceAllocation;
use crate::security::SecurityContext;
use crate::config::ServiceConfig;
use crate::types::{ExecutionResult, RuntimeResourceUsage};

#[derive(Debug, thiserror::Error)]
pub enum NativeError {
    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),
    
    #[error("Process execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: Uuid },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type NativeResult<T> = Result<T, NativeError>;

#[derive(Debug, Clone)]
pub struct NativeTaskHandle {
    pub task_id: Uuid,
    pub process_id: u32,
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct NativeStats {
    pub active_processes: usize,
    pub total_processes_run: usize,
    pub total_execution_time: Duration,
    pub platform: String,
}

pub struct NativeRuntime {
    stats: Arc<RwLock<NativeStats>>,
    active_processes: Arc<RwLock<HashMap<Uuid, Child>>>,
}

impl NativeRuntime {
    pub async fn new() -> Result<Self, NativeError> {
        info!("Initializing native runtime");
        
        let stats = NativeStats {
            active_processes: 0,
            total_processes_run: 0,
            total_execution_time: Duration::from_secs(0),
            platform: std::env::consts::OS.to_owned(),
        };
        
        Ok(Self {
            stats: Arc::new(RwLock::new(stats)),
            active_processes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn execute(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        _security_context: &SecurityContext,
        _resource_allocation: &ResourceAllocation,
    ) -> Result<NativeTaskHandle, NativeError> {
        info!("Executing native task: {}", task_id);
        
        // Create the command from service config
        let mut command = Command::new(&service.executable);
        
        // Add arguments if specified
        if let Some(args) = &service.args {
            command.args(args);
        }
        
        // Set working directory if specified
        if let Some(working_dir) = &service.working_dir {
            command.current_dir(working_dir);
        }
        
        // Configure stdio
        command.stdout(Stdio::piped())
               .stderr(Stdio::piped())
               .stdin(Stdio::null());
        
        // Spawn the process
        let mut child = command.spawn()
            .map_err(|e| NativeError::ProcessSpawnFailed(e.to_string()))?;
        
        let process_id = child.id();
        let started_at = Instant::now();
        
        // Store the process for management
        {
            let mut processes = self.active_processes.write().await;
            processes.insert(task_id, child);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_processes += 1;
            stats.total_processes_run += 1;
        }
        
        Ok(NativeTaskHandle {
            task_id,
            process_id,
            started_at,
        })
    }

    pub async fn wait_for_completion(&self, task_id: Uuid) -> Result<ExecutionResult, NativeError> {
        info!("Waiting for native task completion: {}", task_id);
        
        let start_time = Instant::now();
        
        // Remove and wait for the process
        let mut child = {
            let mut processes = self.active_processes.write().await;
            processes.remove(&task_id)
                .ok_or(NativeError::TaskNotFound { task_id })?
        };
        
        // Wait for the process to complete
        let output = child.wait_with_output()
            .await
            .map_err(|e| NativeError::ExecutionFailed(e.to_string()))?;
        
        let duration = start_time.elapsed();
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_processes -= 1;
            stats.total_execution_time += duration;
        }
        
        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration,
            resource_usage: RuntimeResourceUsage {
                cpu_time: duration, // Approximation
                memory_peak: 1024 * 1024, // 1MB default
                disk_read: 0,
                disk_write: 0,
                network_in: 0,
                network_out: 0,
            },
        })
    }

    pub async fn stop_task(&self, task_id: Uuid, force: bool) -> Result<(), NativeError> {
        info!("Stopping native task: {} (force: {})", task_id, force);
        
        let mut processes = self.active_processes.write().await;
        if let Some(mut child) = processes.remove(&task_id) {
            if force {
                child.kill().await.map_err(|e| NativeError::ExecutionFailed(e.to_string()))?;
            } else {
                // Try graceful shutdown first
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Already finished
                    }
                    Ok(None) => {
                        // Still running, kill it
                        child.kill().await.map_err(|e| NativeError::ExecutionFailed(e.to_string()))?;
                    }
                    Err(e) => {
                        return Err(NativeError::ExecutionFailed(e.to_string()));
                    }
                }
            }
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_processes -= 1;
        }
        
        Ok(())
    }

    pub async fn get_stats(&self) -> NativeStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_native_runtime_creation() {
        let runtime = NativeRuntime::new().await.unwrap();
        assert_eq!(runtime.get_stats().await.active_processes, 0);
    }

    #[tokio::test]
    async fn test_native_stats() {
        let runtime = NativeRuntime::new().await.unwrap();
        let stats = runtime.get_stats().await;
        assert_eq!(stats.total_processes_run, 0);
        assert_eq!(stats.platform, std::env::consts::OS);
    }
} 