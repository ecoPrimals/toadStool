// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Scheduler for cross-platform job execution
//!
//! The scheduler routes jobs to appropriate execution backends:
//! 1. **Primal Registry**: Discovers remote primals with capabilities
//! 2. **Runtime Engines**: Local engines for direct execution (WASM, Native)
//!
//! ## Execution Flow
//!
//! ```text
//! Job → Try Primal Registry → Found? → Execute via Primal
//!                            ↓ Not Found
//!                     Try Runtime Engine → Found? → Execute Locally
//!                            ↓ Not Found
//!                     Return Fallback/Error
//! ```

mod execution;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::execution::{ExecutionResponse, RuntimeEngine, RuntimeType};
use crate::ToadStoolResult;

use super::jobs::{UniversalJob, UniversalJobType};
use super::registry::UniversalPrimalRegistry;
use super::resources::ResourceCoordinator;
use super::types::PrimalCapability;

/// Universal scheduler for any substrate
pub struct UniversalScheduler {
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry>,
    /// Resource coordinator
    resource_coordinator: Arc<ResourceCoordinator>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, UniversalJob>>>,
    /// Runtime engines for local execution (optional)
    runtime_engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
}

impl UniversalScheduler {
    /// Create new scheduler
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    #[must_use = "Scheduler creation should be checked"]
    pub async fn new(primal_registry: Arc<UniversalPrimalRegistry>) -> ToadStoolResult<Self> {
        Ok(Self {
            primal_registry,
            resource_coordinator: Arc::new(ResourceCoordinator::new().await?),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create scheduler with runtime engines for local execution
    ///
    /// # Arguments
    /// * `primal_registry` - Registry for discovering remote primals
    /// * `runtime_engines` - Map of runtime type to engine for local execution
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    pub async fn with_runtime_engines(
        primal_registry: Arc<UniversalPrimalRegistry>,
        runtime_engines: HashMap<RuntimeType, Box<dyn RuntimeEngine>>,
    ) -> ToadStoolResult<Self> {
        info!(
            "Creating scheduler with {} runtime engines: {:?}",
            runtime_engines.len(),
            runtime_engines.keys().collect::<Vec<_>>()
        );
        Ok(Self {
            primal_registry,
            resource_coordinator: Arc::new(ResourceCoordinator::new().await?),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            runtime_engines: Arc::new(RwLock::new(runtime_engines)),
        })
    }

    /// Register a runtime engine for local execution
    ///
    /// Allows adding runtime engines after scheduler creation.
    pub async fn register_runtime_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Box<dyn RuntimeEngine>,
    ) {
        info!("Registering runtime engine: {:?}", runtime_type);
        self.runtime_engines
            .write()
            .await
            .insert(runtime_type, engine);
    }

    /// Get available runtime types
    pub async fn available_runtimes(&self) -> Vec<RuntimeType> {
        self.runtime_engines.read().await.keys().cloned().collect()
    }

    /// Schedule a job
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - Resource allocation fails.
    /// - Job execution fails.
    /// - No suitable primal can be found for the job.
    #[must_use = "Job scheduling result should be checked"]
    pub async fn schedule_job(&self, job: UniversalJob) -> ToadStoolResult<ExecutionResponse> {
        let job_id = job.id;
        info!("Scheduling job: {}", job_id);

        // Add to active jobs
        self.active_jobs.write().await.insert(job_id, job.clone());

        // Allocate resources
        let _allocation = self
            .resource_coordinator
            .allocate_resources(&job.resources)
            .await?;

        // Execute based on job type
        let result = match &job.job_type {
            UniversalJobType::Native {
                executable,
                args,
                env,
            } => self.execute_native(executable, args, env).await,
            UniversalJobType::Wasm { module, args, env } => {
                self.execute_wasm(module, args, env).await
            }
            UniversalJobType::Primal {
                primal_type,
                endpoint,
                payload,
            } => self.execute_primal(primal_type, endpoint, payload).await,
            UniversalJobType::BiomeOS {
                biome_manifest,
                team_id,
            } => self.execute_biome_os(biome_manifest, team_id).await,
        };

        // Remove from active jobs
        self.active_jobs.write().await.remove(&job_id);

        result
    }

    /// Get active job count
    pub async fn get_active_job_count(&self) -> usize {
        self.active_jobs.read().await.len()
    }

    /// Find primals by capability using the registry
    pub async fn find_primals_by_capability(
        &self,
        capability: &PrimalCapability,
    ) -> Vec<Arc<dyn super::traits::UniversalPrimalProvider>> {
        self.primal_registry.find_by_capability(capability).await
    }

    /// Access primal registry (for execution submodule)
    pub(crate) fn primal_registry(&self) -> &Arc<UniversalPrimalRegistry> {
        &self.primal_registry
    }

    /// Access runtime engines (for execution submodule)
    pub(crate) fn runtime_engines(
        &self,
    ) -> &Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>> {
        &self.runtime_engines
    }
}
