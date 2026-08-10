// SPDX-License-Identifier: AGPL-3.0-or-later
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

use std::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::ToadStoolResult;
use crate::execution::{ExecutionResponse, RuntimeEngine, RuntimeType, StubRuntimeEngine};

use super::jobs::{UniversalJob, UniversalJobType};
use super::primal_provider_dispatch::UniversalPrimalProviderDispatch;
use super::registry::UniversalPrimalRegistry;
use super::resources::ResourceCoordinator;
use super::traits::UniversalPrimalProvider;
use super::types::PrimalCapability;

/// Universal scheduler for any substrate
pub struct UniversalScheduler<
    P = UniversalPrimalProviderDispatch,
    E: RuntimeEngine = StubRuntimeEngine,
> where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry<P>>,
    /// Resource coordinator
    resource_coordinator: Arc<ResourceCoordinator>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, UniversalJob>>>,
    /// Runtime engines for local execution (optional)
    runtime_engines: Arc<RwLock<HashMap<RuntimeType, Arc<E>>>>,
}

impl<P, E: RuntimeEngine> UniversalScheduler<P, E>
where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Create new scheduler for engine type `E`.
    ///
    /// When `E` is [`StubRuntimeEngine`], prefer the inherent [`UniversalScheduler::new`]
    /// constructor (no turbofish).
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    #[must_use = "Scheduler creation should be checked"]
    pub async fn create(primal_registry: Arc<UniversalPrimalRegistry<P>>) -> ToadStoolResult<Self> {
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
    pub async fn create_with_runtime_engines(
        primal_registry: Arc<UniversalPrimalRegistry<P>>,
        runtime_engines: HashMap<RuntimeType, Arc<E>>,
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
    pub fn register_runtime_engine(&self, runtime_type: RuntimeType, engine: Arc<E>) {
        info!("Registering runtime engine: {:?}", runtime_type);
        self.runtime_engines
            .write().unwrap_or_else(|e| e.into_inner())
            .insert(runtime_type, engine);
    }

    /// Get available runtime types
    pub fn available_runtimes(&self) -> Vec<RuntimeType> {
        self.runtime_engines.read().unwrap_or_else(|e| e.into_inner()).keys().cloned().collect()
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
        self.active_jobs.write().unwrap_or_else(|e| e.into_inner()).insert(job_id, job.clone());

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
            } => {
                #[cfg(feature = "runtime")]
                { self.execute_native(executable, args, env).await }
                #[cfg(not(feature = "runtime"))]
                { let _ = (executable, args, env); Err(crate::ToadStoolError::runtime("native execution requires runtime feature".to_string())) }
            }
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
        self.active_jobs.write().unwrap_or_else(|e| e.into_inner()).remove(&job_id);

        result
    }

    /// Get active job count
    pub fn get_active_job_count(&self) -> usize {
        self.active_jobs.read().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Find primals by capability using the registry
    pub fn find_primals_by_capability(&self, capability: &PrimalCapability) -> Vec<Arc<P>> {
        self.primal_registry.find_by_capability(capability)
    }

    /// Access primal registry (for execution submodule)
    pub(crate) const fn primal_registry(&self) -> &Arc<UniversalPrimalRegistry<P>> {
        &self.primal_registry
    }

    /// Access runtime engines (for execution submodule)
    pub(crate) fn runtime_engines(&self) -> &Arc<RwLock<HashMap<RuntimeType, Arc<E>>>> {
        &self.runtime_engines
    }
}

impl<P> UniversalScheduler<P, StubRuntimeEngine>
where
    P: UniversalPrimalProvider + Send + Sync + 'static,
{
    /// Create new scheduler using [`StubRuntimeEngine`] until real engines are registered.
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if resource coordinator initialization fails.
    #[must_use = "Scheduler creation should be checked"]
    pub async fn new(primal_registry: Arc<UniversalPrimalRegistry<P>>) -> ToadStoolResult<Self> {
        Self::create(primal_registry).await
    }
}
