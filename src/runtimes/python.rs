//! # Python Runtime
//!
//! Python runtime implementation.

use uuid::Uuid;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::resources::ResourceAllocation;
use crate::security::SecurityContext;
use crate::config::ServiceConfig;
use crate::types::{ExecutionResult, RuntimeResourceUsage};

#[derive(Debug, thiserror::Error)]
pub enum PythonError {
    #[error("Python interpreter not available: {0}")]
    InterpreterNotAvailable(String),
    
    #[error("Python execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: Uuid },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PythonResult<T> = Result<T, PythonError>;

#[derive(Debug, Clone)]
pub struct PythonTaskHandle {
    pub task_id: Uuid,
    pub interpreter_id: String,
    pub started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct PythonStats {
    pub active_interpreters: usize,
    pub total_scripts_run: usize,
    pub total_execution_time: Duration,
    pub python_version: String,
}

pub struct PythonRuntime {
    stats: Arc<RwLock<PythonStats>>,
    active_processes: Arc<RwLock<HashMap<Uuid, tokio::process::Child>>>,
}

impl PythonRuntime {
    pub async fn new() -> Result<Self, PythonError> {
        info!("Initializing Python runtime");
        
        // Check if Python is available and get version
        let python_version = Self::get_python_version().await?;
        
        let stats = PythonStats {
            active_interpreters: 0,
            total_scripts_run: 0,
            total_execution_time: Duration::from_secs(0),
            python_version,
        };
        
        Ok(Self {
            stats: Arc::new(RwLock::new(stats)),
            active_processes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn get_python_version() -> Result<String, PythonError> {
        let output = Command::new("python3")
            .args(&["--version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| PythonError::InterpreterNotAvailable(format!("Python3 not available: {e}")))?;
        
        if !output.status.success() {
            return Err(PythonError::InterpreterNotAvailable("Python3 not available".to_owned()));
        }
        
        let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(version)
    }
    
    pub async fn execute(
        &self,
        task_id: Uuid,
        service: &ServiceConfig,
        _security_context: &SecurityContext,
        _resource_allocation: &ResourceAllocation,
    ) -> Result<PythonTaskHandle, PythonError> {
        info!("Executing Python task: {}", task_id);
        
        // Create Python command
        let mut command = tokio::process::Command::new("python3");
        
        // Add the script file or execute directly
        if let Some(script_path) = &service.script_path {
            command.arg(script_path);
        } else if let Some(script_content) = &service.script_content {
            command.arg("-c").arg(script_content);
        } else {
            return Err(PythonError::ExecutionFailed("No script provided".to_owned()));
        }
        
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
        let child = command.spawn()
            .map_err(|e| PythonError::ExecutionFailed(e.to_string()))?;
        
        let interpreter_id = format!("python-{task_id}");
        let started_at = Instant::now();
        
        // Store the process for management
        {
            let mut processes = self.active_processes.write().await;
            processes.insert(task_id, child);
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_interpreters += 1;
            stats.total_scripts_run += 1;
        }
        
        Ok(PythonTaskHandle {
            task_id,
            interpreter_id,
            started_at,
        })
    }

    pub async fn wait_for_completion(&self, task_id: Uuid) -> Result<ExecutionResult, PythonError> {
        info!("Waiting for Python task completion: {}", task_id);
        
        let start_time = Instant::now();
        
        // Remove and wait for the process
        let mut child = {
            let mut processes = self.active_processes.write().await;
            processes.remove(&task_id)
                .ok_or(PythonError::TaskNotFound { task_id })?
        };
        
        // Wait for the process to complete
        let output = child.wait_with_output()
            .await
            .map_err(|e| PythonError::ExecutionFailed(e.to_string()))?;
        
        let duration = start_time.elapsed();
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_interpreters -= 1;
            stats.total_execution_time += duration;
        }
        
        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration,
            resource_usage: RuntimeResourceUsage {
                cpu_time: duration, // Approximation
                memory_peak: 32 * 1024 * 1024, // 32MB default
                disk_read: 0,
                disk_write: 0,
                network_in: 0,
                network_out: 0,
            },
        })
    }

    pub async fn stop_task(&self, task_id: Uuid, force: bool) -> Result<(), PythonError> {
        info!("Stopping Python task: {} (force: {})", task_id, force);
        
        let mut processes = self.active_processes.write().await;
        if let Some(mut child) = processes.remove(&task_id) {
            if force {
                child.kill().await.map_err(|e| PythonError::ExecutionFailed(e.to_string()))?;
            } else {
                // Try graceful shutdown first
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Already finished
                    }
                    Ok(None) => {
                        // Still running, kill it
                        child.kill().await.map_err(|e| PythonError::ExecutionFailed(e.to_string()))?;
                    }
                    Err(e) => {
                        return Err(PythonError::ExecutionFailed(e.to_string()));
                    }
                }
            }
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_interpreters -= 1;
        }
        
        Ok(())
    }

    pub async fn get_stats(&self) -> PythonStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_python_runtime_creation() {
        let runtime = PythonRuntime::new().await.unwrap();
        assert_eq!(runtime.get_stats().await.active_interpreters, 0);
    }

    #[tokio::test]
    async fn test_python_stats() {
        let runtime = PythonRuntime::new().await.unwrap();
        let stats = runtime.get_stats().await;
        assert_eq!(stats.total_scripts_run, 0);
        assert!(stats.python_version.contains("Python"));
    }
} 