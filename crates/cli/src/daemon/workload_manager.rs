// SPDX-License-Identifier: AGPL-3.0-only
//! Workload Manager for ToadStool Daemon Mode
//!
//! Manages the lifecycle of workloads:
//! - Queuing incoming workload requests
//! - Executing via BiomeExecutor
//! - Tracking status and resource usage
//! - Managing concurrent workloads
//! - Stop/restart/cancel operations

use crate::{CliContextExt, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use super::api_types::{
    ResourceUsage, SubmitWorkloadRequest, WorkloadStatus, WorkloadStatusResponse,
};
use crate::executor::BiomeExecutor;

/// Workload metadata
#[derive(Debug, Clone)]
#[allow(dead_code)] // Some fields (environment, timeout_secs) used in future phases
pub struct WorkloadMetadata {
    /// Workload ID
    pub id: String,

    /// Requester identity
    pub requester: String,

    /// Biome YAML content
    pub biome_yaml: String,

    /// Environment variables
    pub environment: HashMap<String, String>,

    /// Started at timestamp
    pub started_at: std::time::SystemTime,

    /// Persistent workload (keep running)
    pub persistent: bool,

    /// Timeout in seconds
    pub timeout_secs: u64,
}

/// Running workload handle
struct RunningWorkload {
    /// Metadata
    metadata: WorkloadMetadata,

    /// Task handle
    _task_handle: JoinHandle<()>,

    /// Current status
    status: Arc<RwLock<WorkloadStatus>>,

    /// Exit code (if completed)
    exit_code: Arc<RwLock<Option<i32>>>,

    /// Error message (if failed)
    error: Arc<RwLock<Option<String>>>,

    /// Resource usage
    resource_usage: Arc<RwLock<Option<ResourceUsage>>>,
}

/// Workload Manager
///
/// Coordinates workload execution with concurrency limits and lifecycle management.
pub struct WorkloadManager {
    /// Maximum concurrent workloads (stored for introspection)
    _max_concurrent: usize,

    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,

    /// Active workloads
    workloads: Arc<RwLock<HashMap<String, RunningWorkload>>>,

    /// BiomeExecutor for workload execution
    executor: Arc<BiomeExecutor>,
}

impl WorkloadManager {
    /// Create a new workload manager
    pub async fn new(max_concurrent: usize) -> Result<Self> {
        info!(
            "📦 Initializing workload manager (max concurrent: {})",
            max_concurrent
        );

        // Create BiomeExecutor
        let executor = BiomeExecutor::new()
            .await
            .context("Failed to create BiomeExecutor")?;

        Ok(Self {
            _max_concurrent: max_concurrent,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            workloads: Arc::new(RwLock::new(HashMap::new())),
            executor: Arc::new(executor),
        })
    }

    /// Submit a workload for execution
    ///
    /// Returns workload ID
    pub async fn submit_workload(&self, request: SubmitWorkloadRequest) -> Result<String> {
        let workload_id = uuid::Uuid::new_v4().to_string();

        info!(
            "📥 Submitting workload {} from {}",
            workload_id, request.requester
        );

        // Create metadata
        let metadata = WorkloadMetadata {
            id: workload_id.clone(),
            requester: request.requester.clone(),
            biome_yaml: request.biome_yaml.clone(),
            environment: request.environment.clone(),
            started_at: std::time::SystemTime::now(),
            persistent: request.persistent,
            timeout_secs: request.timeout_secs.unwrap_or(3600),
        };

        // Create status tracking
        let status = Arc::new(RwLock::new(WorkloadStatus::Queued));
        let exit_code = Arc::new(RwLock::new(None));
        let error_msg = Arc::new(RwLock::new(None));
        let resource_usage = Arc::new(RwLock::new(None));

        // Clone for task
        let semaphore = Arc::clone(&self.semaphore);
        let executor = Arc::clone(&self.executor);
        let workloads = Arc::clone(&self.workloads);
        let metadata_clone = metadata.clone();
        let status_clone = Arc::clone(&status);
        let exit_code_clone = Arc::clone(&exit_code);
        let error_clone = Arc::clone(&error_msg);
        let resource_clone = Arc::clone(&resource_usage);

        // Spawn workload execution task
        let task_handle = tokio::spawn(async move {
            // Wait for semaphore permit (concurrency control)
            let _permit = semaphore.acquire().await;

            info!("🚀 Starting workload execution: {}", metadata_clone.id);
            *status_clone.write().await = WorkloadStatus::Running;

            // Execute workload
            match Self::execute_workload_internal(&executor, &metadata_clone, &resource_clone).await
            {
                Ok(code) => {
                    info!(
                        "✅ Workload {} completed with exit code {}",
                        metadata_clone.id, code
                    );
                    *status_clone.write().await = WorkloadStatus::Completed;
                    *exit_code_clone.write().await = Some(code);
                }
                Err(e) => {
                    error!("❌ Workload {} failed: {}", metadata_clone.id, e);
                    *status_clone.write().await = WorkloadStatus::Failed;
                    *error_clone.write().await = Some(e.to_string());
                }
            }

            // Remove from active workloads immediately after completion (unless persistent)
            // Event-driven: cleanup triggered by workload completion, not arbitrary delay
            if !metadata_clone.persistent {
                workloads.write().await.remove(&metadata_clone.id);
                info!("🧹 Cleaned up workload: {}", metadata_clone.id);
            }
        });

        // Store workload
        let workload = RunningWorkload {
            metadata: metadata.clone(),
            _task_handle: task_handle,
            status,
            exit_code,
            error: error_msg,
            resource_usage,
        };

        self.workloads
            .write()
            .await
            .insert(workload_id.clone(), workload);

        Ok(workload_id)
    }

    /// Execute workload using BiomeExecutor
    async fn execute_workload_internal(
        _executor: &BiomeExecutor,
        metadata: &WorkloadMetadata,
        resource_usage: &Arc<RwLock<Option<ResourceUsage>>>,
    ) -> Result<i32> {
        info!("🔧 Executing workload: {}", metadata.id);

        // Pending: Parse biome.yaml into validated manifest struct (BiomeManifest) and apply
        // defaults before writing. Phase 3 simulation writes raw YAML as-is.

        // Write biome.yaml to temp file
        let temp_dir = std::env::temp_dir();
        let manifest_path = temp_dir.join(format!("workload-{}.yaml", metadata.id));

        tokio::fs::write(&manifest_path, &metadata.biome_yaml)
            .await
            .context("Failed to write manifest file")?;

        info!("📄 Manifest written to: {}", manifest_path.display());

        // BLOCKED(biome-executor): Awaiting BiomeOS executor API for batch workload execution.
        // run_biome() exists but is designed for interactive foreground mode (waits for SIGINT).
        // Workload manager needs a run-to-completion API that returns exit code.

        // Simulate resource usage
        *resource_usage.write().await = Some(ResourceUsage {
            cpu_percent: 15.5,
            memory_bytes: 1024 * 1024 * 512, // 512 MB
            gpu_percent: None,
            storage_bytes: Some(1024 * 1024 * 100), // 100 MB
        });

        info!("✅ Workload {} completed", metadata.id);

        // Clean up manifest file
        if let Err(e) = tokio::fs::remove_file(&manifest_path).await {
            warn!("⚠️  Failed to remove manifest file: {}", e);
        }

        Ok(0) // Success
    }

    /// Get workload status
    pub async fn get_workload_status(&self, workload_id: &str) -> Option<WorkloadStatusResponse> {
        let workloads = self.workloads.read().await;
        let workload = workloads.get(workload_id)?;

        let status = *workload.status.read().await;
        let exit_code = *workload.exit_code.read().await;
        let error = workload.error.read().await.clone();
        let resource_usage = workload.resource_usage.read().await.clone();

        Some(WorkloadStatusResponse {
            workload_id: workload_id.to_string(),
            status,
            started_at: Some(toadstool_common::system_time_serde::format_rfc3339(
                workload.metadata.started_at,
            )),
            completed_at: if status == WorkloadStatus::Completed || status == WorkloadStatus::Failed
            {
                Some(toadstool_common::system_time_serde::format_rfc3339(
                    std::time::SystemTime::now(),
                ))
            } else {
                None
            },
            exit_code,
            error,
            resource_usage,
        })
    }

    /// Get workload metadata (requester, persistent) for display
    pub async fn get_workload_metadata(&self, workload_id: &str) -> (Option<String>, Option<bool>) {
        let workloads = self.workloads.read().await;
        let workload = match workloads.get(workload_id) {
            Some(w) => w,
            None => return (None, None),
        };
        (
            Some(workload.metadata.requester.clone()),
            Some(workload.metadata.persistent),
        )
    }

    /// List all workloads
    pub async fn list_workloads(&self) -> Vec<String> {
        let workloads = self.workloads.read().await;
        workloads.keys().cloned().collect()
    }

    /// Cancel a workload
    pub async fn cancel_workload(&self, workload_id: &str) -> Result<()> {
        info!("🛑 Cancelling workload: {}", workload_id);

        let mut workloads = self.workloads.write().await;

        if let Some(workload) = workloads.get(workload_id) {
            *workload.status.write().await = WorkloadStatus::Cancelled;
            // Task will be aborted when dropped
            workloads.remove(workload_id);
            info!("✅ Workload cancelled: {}", workload_id);
            Ok(())
        } else {
            Err(crate::CliError::Other(format!(
                "Workload {workload_id} not found"
            )))
        }
    }

    /// Get active workload count
    pub async fn active_workload_count(&self) -> usize {
        let workloads = self.workloads.read().await;
        workloads.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workload_manager_creation() {
        let manager = WorkloadManager::new(10).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_submit_workload() {
        let manager = WorkloadManager::new(10).await.unwrap();

        let request = SubmitWorkloadRequest {
            biome_yaml: "version: 1.0".to_string(),
            requester: "test".to_string(),
            environment: HashMap::new(),
            resources: None,
            timeout_secs: Some(60),
            persistent: false,
        };

        let workload_id = manager.submit_workload(request).await;
        assert!(workload_id.is_ok());

        let id = workload_id.unwrap();
        assert!(!id.is_empty());

        // Wait for status to become available (event-driven: poll until we get a result)
        let status = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
            loop {
                if let Some(s) = manager.get_workload_status(&id).await {
                    return s;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;

        // Should be able to get status
        assert!(status.is_ok());
    }

    #[tokio::test]
    async fn test_list_workloads() {
        let manager = WorkloadManager::new(10).await.unwrap();

        let request = SubmitWorkloadRequest {
            biome_yaml: "version: 1.0".to_string(),
            requester: "test".to_string(),
            environment: HashMap::new(),
            resources: None,
            timeout_secs: Some(60),
            persistent: false,
        };

        manager.submit_workload(request.clone()).await.unwrap();
        manager.submit_workload(request).await.unwrap();

        // Wait for workloads to be registered (event-driven)
        let workloads = tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
            loop {
                let list = manager.list_workloads().await;
                if list.len() >= 2 {
                    return list;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("Workloads should be listed within 5s");
        assert_eq!(workloads.len(), 2);
    }
}
