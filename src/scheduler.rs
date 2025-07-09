//! # Workload Scheduler
//!
//! Core scheduling and execution engine for biomes.

use crate::manifest::{BiomeRuntime, BiomeStatus, ServiceConfig};
use crate::runtimes::{RuntimeManager, RuntimeError};
use crate::security::{SecurityManager, SecurityError};
use crate::resources::{ResourceManager, ResourceError};

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{Duration, timeout};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Scheduler-specific errors
#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),
    
    #[error("Biome not found: {id}")]
    BiomeNotFound { id: Uuid },
    
    #[error("Service not found: {name}")]
    ServiceNotFound { name: String },
    
    #[error("Scheduling failed: {reason}")]
    SchedulingFailed { reason: String },
    
    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },
    
    #[error("Timeout: {operation}")]
    Timeout { operation: String },
    
    #[error("Dependency error: {dependency}")]
    DependencyError { dependency: String },
}

/// Log entry for biome output
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub service: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// Scheduled task information
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub biome_id: Uuid,
    pub service_name: String,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub exit_code: Option<i32>,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub network_rx: u64,
    pub network_tx: u64,
}

/// Main workload scheduler
pub struct WorkloadScheduler {
    runtime_manager: Arc<RuntimeManager>,
    security_manager: Arc<SecurityManager>,
    resource_manager: Arc<ResourceManager>,
    
    // Active biomes and tasks
    biomes: Arc<RwLock<HashMap<Uuid, BiomeExecution>>>,
    tasks: Arc<RwLock<HashMap<Uuid, ScheduledTask>>>,
    
    // Communication channels
    task_events: broadcast::Sender<TaskEvent>,
    log_channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<String>>>>,
    
    // Configuration
    max_concurrent_tasks: usize,
    default_timeout: Duration,
}

/// Biome execution state
#[derive(Debug)]
struct BiomeExecution {
    runtime: BiomeRuntime,
    tasks: Vec<Uuid>,
    status: BiomeStatus,
    completion_tx: Option<tokio::sync::oneshot::Sender<i32>>,
}

/// Task events for monitoring
#[derive(Debug, Clone)]
pub enum TaskEvent {
    TaskStarted { task_id: Uuid, service: String },
    TaskCompleted { task_id: Uuid, exit_code: i32 },
    TaskFailed { task_id: Uuid, error: String },
    LogMessage { task_id: Uuid, message: String },
    ResourceUpdate { task_id: Uuid, usage: ResourceUsage },
}

impl WorkloadScheduler {
    pub async fn new() -> Result<Self, SchedulerError> {
        let runtime_manager = Arc::new(RuntimeManager::new().await?);
        let security_manager = Arc::new(SecurityManager::new().await?);
        let resource_manager = Arc::new(ResourceManager::new().await?);
        
        let (task_events, _) = broadcast::channel(1000);
        
        Ok(Self {
            runtime_manager,
            security_manager,
            resource_manager,
            biomes: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            task_events,
            log_channels: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_tasks: 100,
            default_timeout: Duration::from_secs(300),
        })
    }

    /// Schedule a biome for execution
    pub async fn schedule_biome(&self, biome: &BiomeRuntime) -> Result<(), SchedulerError> {
        info!("Scheduling biome: {} ({})", biome.name, biome.id);
        
        // Validate resources
        self.resource_manager.validate_biome_resources(&biome.manifest).await?;
        
        // Create log channel for this biome
        let (log_tx, _) = broadcast::channel(1000);
        {
            let mut log_channels = self.log_channels.write().await;
            log_channels.insert(biome.id, log_tx);
        }
        
        // Create completion channel
        let (completion_tx, completion_rx) = tokio::sync::oneshot::channel();
        
        // Create biome execution
        let biome_execution = BiomeExecution {
            runtime: biome.clone(),
            tasks: Vec::new(),
            status: BiomeStatus::Starting,
            completion_tx: Some(completion_tx),
        };
        
        // Store biome execution
        {
            let mut biomes = self.biomes.write().await;
            biomes.insert(biome.id, biome_execution);
        }
        
        // Schedule services
        let service_tasks = self.schedule_services(biome).await?;
        
        // Update biome with task IDs
        {
            let mut biomes = self.biomes.write().await;
            if let Some(biome_exec) = biomes.get_mut(&biome.id) {
                biome_exec.tasks = service_tasks;
                biome_exec.status = BiomeStatus::Running;
            }
        }
        
        // Start monitoring task
        self.start_biome_monitor(biome.id, completion_rx).await;
        
        Ok(())
    }

    /// Schedule individual services within a biome
    async fn schedule_services(&self, biome: &BiomeRuntime) -> Result<Vec<Uuid>, SchedulerError> {
        let mut task_ids = Vec::new();
        
        // Sort services by dependencies
        let ordered_services = self.resolve_service_dependencies(&biome.manifest.services)?;
        
        for service in ordered_services {
            let task_id = self.schedule_service(biome, &service).await?;
            task_ids.push(task_id);
        }
        
        Ok(task_ids)
    }

    /// Schedule a single service
    async fn schedule_service(
        &self,
        biome: &BiomeRuntime,
        service: &ServiceConfig,
    ) -> Result<Uuid, SchedulerError> {
        let task_id = Uuid::new_v4();
        
        info!("Scheduling service: {} ({})", service.name, task_id);
        
        // Create scheduled task
        let task = ScheduledTask {
            id: task_id,
            biome_id: biome.id,
            service_name: service.name.clone(),
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            exit_code: None,
            resource_usage: None,
        };
        
        // Store task
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id, task);
        }
        
        // Execute service
        let scheduler = self.clone();
        tokio::spawn(async move {
            if let Err(e) = scheduler.execute_service(task_id, biome, service).await {
                error!("Service execution failed: {} - {}", service.name, e);
                scheduler.handle_task_failure(task_id, e.to_string()).await;
            }
        });
        
        Ok(task_id)
    }

    /// Execute a service
    async fn execute_service(
        &self,
        task_id: Uuid,
        biome: &BiomeRuntime,
        service: &ServiceConfig,
    ) -> Result<(), SchedulerError> {
        info!("Executing service: {} ({})", service.name, task_id);
        
        // Update task status
        self.update_task_status(task_id, TaskStatus::Running).await?;
        
        // Send task started event
        let _ = self.task_events.send(TaskEvent::TaskStarted {
            task_id,
            service: service.name.clone(),
        });
        
        // Create security context
        let security_context = self.security_manager
            .create_service_context(service, &biome.manifest.security).await?;
        
        // Allocate resources
        let resource_allocation = self.resource_manager
            .allocate_service_resources(service).await?;
        
        // Execute based on runtime
        let exit_code = match service.runtime.as_str() {
            "wasm" => {
                self.runtime_manager.execute_wasm(
                    task_id,
                    service,
                    &security_context,
                    &resource_allocation,
                ).await?
            }
            "container" => {
                self.runtime_manager.execute_container(
                    task_id,
                    service,
                    &security_context,
                    &resource_allocation,
                ).await?
            }
            "native" => {
                self.runtime_manager.execute_native(
                    task_id,
                    service,
                    &security_context,
                    &resource_allocation,
                ).await?
            }
            "python" => {
                self.runtime_manager.execute_python(
                    task_id,
                    service,
                    &security_context,
                    &resource_allocation,
                ).await?
            }
            _ => {
                return Err(SchedulerError::ExecutionFailed {
                    reason: format!("Unsupported runtime: {}", service.runtime),
                });
            }
        };
        
        // Clean up resources
        self.resource_manager.deallocate_service_resources(&resource_allocation).await?;
        
        // Update task completion
        self.update_task_completion(task_id, exit_code).await?;
        
        // Send task completed event
        let _ = self.task_events.send(TaskEvent::TaskCompleted {
            task_id,
            exit_code,
        });
        
        info!("Service completed: {} (exit code: {})", service.name, exit_code);
        
        Ok(())
    }

    /// Wait for a biome to complete
    pub async fn wait_for_biome(&self, biome_id: Uuid) -> Result<i32, SchedulerError> {
        let completion_rx = {
            let mut biomes = self.biomes.write().await;
            biomes.get_mut(&biome_id)
                .ok_or(SchedulerError::BiomeNotFound { id: biome_id })?
                .completion_tx
                .take()
                .ok_or(SchedulerError::ExecutionFailed {
                    reason: "Biome already completed".to_string(),
                })?
        };
        
        match completion_rx.await {
            Ok(exit_code) => Ok(exit_code),
            Err(_) => Err(SchedulerError::ExecutionFailed {
                reason: "Biome execution was cancelled".to_string(),
            }),
        }
    }

    /// Stop a biome gracefully
    pub async fn stop_biome(&self, biome_id: Uuid, timeout_secs: u64) -> Result<i32, SchedulerError> {
        info!("Stopping biome: {}", biome_id);
        
        let task_ids = {
            let biomes = self.biomes.read().await;
            biomes.get(&biome_id)
                .ok_or(SchedulerError::BiomeNotFound { id: biome_id })?
                .tasks
                .clone()
        };
        
        // Stop all tasks
        for task_id in task_ids {
            self.stop_task(task_id, false).await?;
        }
        
        // Wait for completion with timeout
        let result = timeout(
            Duration::from_secs(timeout_secs),
            self.wait_for_biome(biome_id),
        ).await;
        
        match result {
            Ok(Ok(exit_code)) => Ok(exit_code),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout - force stop
                self.force_stop_biome(biome_id).await
            }
        }
    }

    /// Force stop a biome
    pub async fn force_stop_biome(&self, biome_id: Uuid) -> Result<i32, SchedulerError> {
        info!("Force stopping biome: {}", biome_id);
        
        let task_ids = {
            let biomes = self.biomes.read().await;
            biomes.get(&biome_id)
                .ok_or(SchedulerError::BiomeNotFound { id: biome_id })?
                .tasks
                .clone()
        };
        
        // Force stop all tasks
        for task_id in task_ids {
            self.stop_task(task_id, true).await?;
        }
        
        // Mark biome as stopped
        {
            let mut biomes = self.biomes.write().await;
            if let Some(biome_exec) = biomes.get_mut(&biome_id) {
                biome_exec.status = BiomeStatus::Stopped;
                if let Some(completion_tx) = biome_exec.completion_tx.take() {
                    let _ = completion_tx.send(130); // SIGINT exit code
                }
            }
        }
        
        Ok(130)
    }

    /// Stop a specific task
    async fn stop_task(&self, task_id: Uuid, force: bool) -> Result<(), SchedulerError> {
        info!("Stopping task: {} (force: {})", task_id, force);
        
        // Update task status
        self.update_task_status(task_id, TaskStatus::Cancelled).await?;
        
        // Stop the runtime execution
        self.runtime_manager.stop_task(task_id, force).await?;
        
        Ok(())
    }

    /// Get log stream for a biome
    pub async fn get_log_stream(&self, biome_id: Uuid) -> Result<broadcast::Receiver<String>, SchedulerError> {
        let log_channels = self.log_channels.read().await;
        
        log_channels.get(&biome_id)
            .ok_or(SchedulerError::BiomeNotFound { id: biome_id })?
            .subscribe()
            .map_err(|_| SchedulerError::ExecutionFailed {
                reason: "Failed to subscribe to log stream".to_string(),
            })
    }

    /// Get logs for a biome
    pub async fn get_logs(&self, biome_id: Uuid, tail: u32) -> Result<Vec<LogEntry>, SchedulerError> {
        // This would typically read from persistent log storage
        // For now, return empty logs
        Ok(Vec::new())
    }

    /// Start monitoring a biome
    async fn start_biome_monitor(
        &self,
        biome_id: Uuid,
        completion_rx: tokio::sync::oneshot::Receiver<i32>,
    ) {
        let scheduler = self.clone();
        
        tokio::spawn(async move {
            // Monitor biome execution
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check biome health
                        if let Err(e) = scheduler.check_biome_health(biome_id).await {
                            error!("Biome health check failed: {}", e);
                        }
                    }
                    result = &mut completion_rx => {
                        match result {
                            Ok(exit_code) => {
                                info!("Biome {} completed with exit code: {}", biome_id, exit_code);
                            }
                            Err(_) => {
                                warn!("Biome {} completion channel closed", biome_id);
                            }
                        }
                        break;
                    }
                }
            }
        });
    }

    /// Check biome health
    async fn check_biome_health(&self, biome_id: Uuid) -> Result<(), SchedulerError> {
        let biomes = self.biomes.read().await;
        
        if let Some(biome_exec) = biomes.get(&biome_id) {
            // Check if all tasks are still running
            let tasks = self.tasks.read().await;
            
            for task_id in &biome_exec.tasks {
                if let Some(task) = tasks.get(task_id) {
                    if matches!(task.status, TaskStatus::Failed) {
                        warn!("Task {} failed in biome {}", task_id, biome_id);
                        // Handle task failure based on restart policy
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Resolve service dependencies
    fn resolve_service_dependencies(&self, services: &[ServiceConfig]) -> Result<Vec<ServiceConfig>, SchedulerError> {
        // Simple topological sort based on dependencies
        let mut resolved = Vec::new();
        let mut remaining: Vec<_> = services.iter().cloned().collect();
        
        while !remaining.is_empty() {
            let mut made_progress = false;
            
            let mut i = 0;
            while i < remaining.len() {
                let service = &remaining[i];
                
                // Check if all dependencies are resolved
                let deps_resolved = service.dependencies.iter().all(|dep| {
                    resolved.iter().any(|s: &ServiceConfig| s.name == *dep)
                });
                
                if deps_resolved {
                    resolved.push(remaining.remove(i));
                    made_progress = true;
                } else {
                    i += 1;
                }
            }
            
            if !made_progress {
                return Err(SchedulerError::DependencyError {
                    dependency: "Circular dependency detected".to_string(),
                });
            }
        }
        
        Ok(resolved)
    }

    /// Update task status
    async fn update_task_status(&self, task_id: Uuid, status: TaskStatus) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = status.clone();
            
            match status {
                TaskStatus::Running => {
                    task.started_at = Some(chrono::Utc::now());
                }
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    task.completed_at = Some(chrono::Utc::now());
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    /// Update task completion
    async fn update_task_completion(&self, task_id: Uuid, exit_code: i32) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = if exit_code == 0 { TaskStatus::Completed } else { TaskStatus::Failed };
            task.exit_code = Some(exit_code);
            task.completed_at = Some(chrono::Utc::now());
        }
        
        Ok(())
    }

    /// Handle task failure
    async fn handle_task_failure(&self, task_id: Uuid, error: String) {
        error!("Task {} failed: {}", task_id, error);
        
        let _ = self.update_task_status(task_id, TaskStatus::Failed).await;
        
        let _ = self.task_events.send(TaskEvent::TaskFailed {
            task_id,
            error,
        });
    }
}

// Clone implementation for Arc sharing
impl Clone for WorkloadScheduler {
    fn clone(&self) -> Self {
        Self {
            runtime_manager: Arc::clone(&self.runtime_manager),
            security_manager: Arc::clone(&self.security_manager),
            resource_manager: Arc::clone(&self.resource_manager),
            biomes: Arc::clone(&self.biomes),
            tasks: Arc::clone(&self.tasks),
            task_events: self.task_events.clone(),
            log_channels: Arc::clone(&self.log_channels),
            max_concurrent_tasks: self.max_concurrent_tasks,
            default_timeout: self.default_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{BiomeManifest, BiomeMetadata, ServiceConfig};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = WorkloadScheduler::new().await;
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_service_dependency_resolution() {
        let scheduler = WorkloadScheduler::new().await.unwrap();
        
        let services = vec![
            ServiceConfig {
                name: "service-b".to_string(),
                runtime: "wasm".to_string(),
                dependencies: vec!["service-a".to_string()],
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
            },
            ServiceConfig {
                name: "service-a".to_string(),
                runtime: "wasm".to_string(),
                dependencies: Vec::new(),
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
            },
        ];
        
        let resolved = scheduler.resolve_service_dependencies(&services).unwrap();
        assert_eq!(resolved[0].name, "service-a");
        assert_eq!(resolved[1].name, "service-b");
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let scheduler = WorkloadScheduler::new().await.unwrap();
        
        let services = vec![
            ServiceConfig {
                name: "service-a".to_string(),
                runtime: "wasm".to_string(),
                dependencies: vec!["service-b".to_string()],
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
            },
            ServiceConfig {
                name: "service-b".to_string(),
                runtime: "wasm".to_string(),
                dependencies: vec!["service-a".to_string()],
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
            },
        ];
        
        let result = scheduler.resolve_service_dependencies(&services);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SchedulerError::DependencyError { .. }));
    }
} 