// SPDX-License-Identifier: AGPL-3.0-or-later
//! Distributed GPU Scheduling - Multi-Tower Compute Coordination
//!
//! This module provides capability-based GPU resource discovery and scheduling
//! across multiple ToadStool towers using the coordination service for discovery and routing.
//!
//! ## Architecture
//!
//! The distributed scheduler is composed of three logical components:
//!
//! - **`TowerManager`**: Discovery and health monitoring of remote towers via the coordination service
//! - **`JobTracker`**: Lifecycle and state management of distributed jobs
//! - **Scheduler**: Coordination and execution strategies
//!
//! ## Design Principles
//!
//! - **Self-Knowledge**: Only knows own capabilities, discovers others at runtime
//! - **Zero-Cost Abstractions**: Module boundaries have no runtime overhead
//! - **Testable**: Each component can be tested independently
//! - **Evolvable**: Easy to add new strategies and capabilities

mod job_tracker;
mod tower_manager;
mod types;

pub use job_tracker::JobTracker;
pub use tower_manager::TowerManager;
pub use types::{
    DistributedJobState, DistributedStats, JobStatus, PartitionStrategy, RemoteTowerEndpoint,
};

use crate::scheduler::UniversalComputeScheduler;
use crate::universal::{UniversalWorkload, WorkloadResult};
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

/// Distributed GPU scheduler for multi-tower execution
///
/// Coordinates GPU workloads across multiple ToadStool towers discovered
/// via coordination-service capability discovery.
///
/// # Example
///
/// ```no_run
/// use std::sync::Arc;
/// use toadstool::error::ToadStoolResult;
/// use toadstool_runtime_gpu::distributed::DistributedGpuScheduler;
/// use toadstool_runtime_gpu::scheduler::{SchedulingPolicy, UniversalComputeScheduler};
///
/// # async fn example() -> ToadStoolResult<()> {
/// let local = Arc::new(UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch));
/// let scheduler = DistributedGpuScheduler::new(local);
///
/// // Discover and register remote towers via the coordination service
/// // ...
///
/// // Execute workload across distributed towers
/// // let result = scheduler.execute_distributed(workload, strategy).await?;
/// # Ok(())
/// # }
/// ```
pub struct DistributedGpuScheduler {
    /// Local GPU scheduler
    local_scheduler: Arc<UniversalComputeScheduler>,

    /// Tower discovery and management
    tower_manager: TowerManager,

    /// Job state tracking
    job_tracker: JobTracker,
}

impl DistributedGpuScheduler {
    /// Create new distributed scheduler with local GPU scheduler
    pub fn new(local_scheduler: Arc<UniversalComputeScheduler>) -> Self {
        let tower_id = Uuid::new_v4().to_string();

        Self {
            local_scheduler,
            tower_manager: TowerManager::new(tower_id),
            job_tracker: JobTracker::new(),
        }
    }

    /// Register a remote tower discovered via the coordination service
    ///
    /// Towers are discovered at runtime through capability queries.
    /// No endpoints are hardcoded.
    pub async fn register_remote_tower(&self, endpoint: RemoteTowerEndpoint) {
        self.tower_manager.register_tower(endpoint).await;
    }

    /// Get all available tower IDs
    pub async fn available_towers(&self) -> Vec<String> {
        self.tower_manager.available_tower_ids().await
    }

    /// Execute workload with distribution strategy
    ///
    /// # Arguments
    ///
    /// * `workload` - The GPU workload to execute
    /// * `strategy` - How to distribute the workload across towers
    ///
    /// # Returns
    ///
    /// Aggregated result from distributed execution
    ///
    /// # Errors
    ///
    /// Returns when distributed execution or local scheduling fails.
    pub async fn execute_distributed(
        &self,
        workload: UniversalWorkload,
        strategy: PartitionStrategy,
    ) -> ToadStoolResult<WorkloadResult> {
        let job_id = Uuid::new_v4().to_string();

        // Register job
        let job = DistributedJobState {
            job_id: job_id.clone(),
            workload: workload.clone(),
            status: JobStatus::Pending,
            assigned_tower: None,
            result: None,
            created_at: std::time::Instant::now(),
            completed_at: None,
        };
        self.job_tracker.register_job(job).await;

        tracing::info!(
            "Executing distributed job {} with strategy: {:?}",
            job_id,
            strategy
        );

        // Execute based on strategy
        let result = match strategy {
            PartitionStrategy::Single => self.execute_single(&job_id, workload).await,
            PartitionStrategy::Redundant { replicas } => {
                self.execute_redundant(&job_id, workload, replicas).await
            }
            PartitionStrategy::DataParallel { chunk_size } => {
                self.execute_data_parallel(&job_id, workload, chunk_size)
                    .await
            }
            PartitionStrategy::Pipeline { stages } => {
                self.execute_pipeline(&job_id, workload, stages).await
            }
        };

        // Update job status based on result
        match &result {
            Ok(workload_result) => {
                self.job_tracker
                    .complete_job(&job_id, workload_result.clone())
                    .await;
            }
            Err(_) => {
                self.job_tracker.fail_job(&job_id).await;
            }
        }

        result
    }

    /// Get distributed scheduling statistics
    pub async fn statistics(&self) -> DistributedStats {
        let mut stats = self.job_tracker.statistics().await;

        // Add tower information
        stats.total_towers = self.tower_manager.tower_count().await;

        // Active towers = towers with recent heartbeat (last 60 seconds)
        // In production, would track health checks; for now assume all active
        stats.active_towers = stats.total_towers;

        stats
    }

    // === Private Execution Methods ===

    /// Execute on single best tower
    async fn execute_single(
        &self,
        job_id: &str,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        // Select best tower
        let tower_id = self
            .tower_manager
            .select_best_tower(&workload.requirements)
            .await?;

        // Assign to tower
        self.job_tracker
            .assign_to_tower(job_id, tower_id.clone())
            .await;

        // Execute
        if tower_id == self.tower_manager.local_tower_id() {
            self.execute_local(workload).await
        } else {
            self.execute_remote(&tower_id, workload).await
        }
    }

    /// Execute with redundancy (race multiple towers, use fastest)
    async fn execute_redundant(
        &self,
        _job_id: &str,
        workload: UniversalWorkload,
        replicas: usize,
    ) -> ToadStoolResult<WorkloadResult> {
        // Select multiple towers
        let towers = self
            .tower_manager
            .select_multiple_towers(&workload.requirements, replicas)
            .await?;

        if towers.is_empty() {
            return Err(ToadStoolError::runtime("No towers available"));
        }

        // Execute on all towers, return first successful result
        let mut handles = Vec::new();

        for tower_id in towers {
            let workload_clone = workload.clone();
            let is_local = tower_id == self.tower_manager.local_tower_id();

            let handle = if is_local {
                let scheduler = Arc::clone(&self.local_scheduler);
                tokio::spawn(async move {
                    let resource = scheduler
                        .select_resource(&workload_clone.requirements)
                        .await?;
                    let mut context = resource.create_context().await?;
                    context.execute(&workload_clone).await
                })
            } else {
                let endpoint = self.tower_manager.get_tower_endpoint(&tower_id).await;
                tokio::spawn(async move {
                    if let Some(ep) = endpoint {
                        // Remote execution via HTTP (Deep Debt: discovered endpoint)
                        Self::execute_remote_http(&ep.address, workload_clone).await
                    } else {
                        Err(ToadStoolError::runtime("Tower endpoint not found"))
                    }
                })
            };

            handles.push(handle);
        }

        // Wait for first successful result
        for handle in handles {
            if let Ok(result) = handle.await
                && result.is_ok()
            {
                return result;
            }
        }

        Err(ToadStoolError::runtime("All redundant executions failed"))
    }

    /// Execute with data parallelism (split data across towers)
    ///
    /// **Architecture**: Data-parallel execution across multiple towers
    ///
    /// **Design** (for future implementation):
    /// 1. Partition input data into chunks
    /// 2. Distribute chunks to available towers
    /// 3. Execute in parallel
    /// 4. Aggregate results
    ///
    /// **Deep Debt**: Capability-based tower selection for each chunk
    ///
    /// **Current**: Graceful fallback to local execution
    async fn execute_data_parallel(
        &self,
        _job_id: &str,
        workload: UniversalWorkload,
        chunk_size: usize,
    ) -> ToadStoolResult<WorkloadResult> {
        tracing::debug!(
            "Data-parallel execution requested (chunk_size: {}), falling back to local",
            chunk_size
        );

        // Graceful degradation: execute locally
        // Production would:
        // 1. Split workload.inputs into chunks of chunk_size
        // 2. Select towers via tower_manager.select_multiple_towers()
        // 3. Execute chunks in parallel across towers
        // 4. Aggregate results from all chunks

        self.execute_local(workload).await
    }

    /// Execute as pipeline (stages across towers)
    ///
    /// **Architecture**: Pipeline execution with stage distribution
    ///
    /// **Design** (for future implementation):
    /// 1. For each stage, select tower with matching capability
    /// 2. Execute stage on selected tower
    /// 3. Pass output to next stage's tower
    /// 4. Return final stage result
    ///
    /// **Deep Debt**: Each stage selected via capability, not tower name
    ///
    /// **Current**: Graceful fallback to local execution
    async fn execute_pipeline(
        &self,
        _job_id: &str,
        workload: UniversalWorkload,
        stages: Vec<String>,
    ) -> ToadStoolResult<WorkloadResult> {
        if stages.is_empty() {
            return self.execute_local(workload).await;
        }

        tracing::debug!(
            "Pipeline execution requested ({} stages), falling back to local",
            stages.len()
        );

        // Graceful degradation: execute locally (production would use tower_manager)

        self.execute_local(workload).await
    }

    /// Execute on local GPU
    async fn execute_local(&self, workload: UniversalWorkload) -> ToadStoolResult<WorkloadResult> {
        // Select best resource and create execution context
        let resource = self
            .local_scheduler
            .select_resource(&workload.requirements)
            .await?;
        let mut context = resource.create_context().await?;
        context.execute(&workload).await
    }

    /// Execute on remote tower
    async fn execute_remote(
        &self,
        tower_id: &str,
        workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        let endpoint = self
            .tower_manager
            .get_tower_endpoint(tower_id)
            .await
            .ok_or_else(|| ToadStoolError::runtime("Tower endpoint not found"))?;

        Self::execute_remote_http(&endpoint.address, workload).await
    }

    /// Execute on remote tower via biomeOS tower (static for spawning)
    ///
    /// ## Implementation Status
    ///
    /// Returns `Err(not_supported)` until biomeOS tower integration is complete.
    /// Remote execution requires network infrastructure and JSON-RPC protocol.
    ///
    /// ## Evolution Path (Pure Rust via biomeOS Tower)
    ///
    /// **NO reqwest/hyper** — these have C dependencies (ring, openssl).
    /// Use biomeOS tower atomic components:
    ///
    /// 1. **Coordination service**: TLS/networking (pure Rust rustls)
    /// 2. **Security service**: Cryptographic operations (pure Rust)
    /// 3. JSON-RPC 2.0 over Unix sockets for local, TCP for remote
    ///
    /// ## Example Implementation
    ///
    /// ```ignore
    /// async fn execute_remote_tower(
    ///     tower_socket: &str,
    ///     workload: UniversalWorkload,
    /// ) -> Result<WorkloadResult> {
    ///     // Use coordination-service client for remote tower connection
    ///     let client = coordination::TowerClient::connect(tower_socket).await?;
    ///     
    ///     // JSON-RPC call to tower's execute method
    ///     client.call("tower.execute", workload).await
    /// }
    /// ```
    ///
    /// ## Deep Debt Principles
    ///
    /// - No hardcoded addresses (address from coordination discovery)
    /// - No hardcoded ports (tower reports its own endpoint)
    /// - Timeout and retry configurable (not hardcoded)
    /// - **Pure Rust**: No C dependencies (no reqwest, hyper, ring)
    async fn execute_remote_http(
        address: &str,
        _workload: UniversalWorkload,
    ) -> ToadStoolResult<WorkloadResult> {
        tracing::debug!(
            "Remote execution to {} not yet implemented; returning not_supported",
            address
        );

        Err(ToadStoolError::not_supported(format!(
            "Remote GPU execution to {address} not yet implemented. \
             Use local execution or await biomeOS tower integration."
        )))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
