// SPDX-License-Identifier: AGPL-3.0-or-later
//! Specialty runtime engine - orchestrates legacy adapters and job execution

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::config::SpecialtyRuntimeConfig;
use crate::types::configs::CompilationToolchainConfig as ToolchainConfig;
use crate::types::emulation::LegacyEmulator;
use crate::types::jobs::LegacyJob;
use crate::types::systems::{LegacyArchitecture, LegacySystemType};
use crate::types::traits::{
    JobOutput, JobStatus, LegacyAdapter, LegacyCommunicationSession, SpecialtyRuntimeMetrics,
};
use toadstool::{ToadStoolError, ToadStoolResult};

use super::embedded;
use super::emulation;
use super::industrial;
use super::mainframe;
use super::realtime;

/// Specialty Hardware Runtime Engine for universal specialty system support
pub struct SpecialtyRuntimeEngine {
    /// Runtime configuration
    pub(crate) config: SpecialtyRuntimeConfig,
    /// Active specialty hardware adapters (Arc for concurrent access across awaits)
    pub(crate) adapters: Arc<RwLock<HashMap<LegacySystemType, Arc<dyn LegacyAdapter>>>>,
    /// Cross-compilation toolchains
    pub(crate) toolchains: Arc<RwLock<HashMap<LegacyArchitecture, ToolchainConfig>>>,
    /// Active specialty jobs
    pub(crate) active_jobs: Arc<RwLock<HashMap<Uuid, LegacyJob>>>,
    /// Communication sessions
    pub(crate) _communication_sessions:
        Arc<RwLock<HashMap<Uuid, Box<dyn LegacyCommunicationSession>>>>,
    /// System emulators
    pub(crate) emulators: Arc<RwLock<HashMap<LegacySystemType, Box<dyn LegacyEmulator>>>>,
    /// Runtime metrics
    pub(crate) metrics: Arc<Mutex<SpecialtyRuntimeMetrics>>,
}

#[expect(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for SpecialtyRuntimeEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpecialtyRuntimeEngine")
            .field("config", &self.config)
            .field("adapters", &"<Arc<dyn LegacyAdapter> map>")
            .field("toolchains", &"<ToolchainConfig map>")
            .field("active_jobs", &"<LegacyJob map>")
            .field("communication_sessions", &"<sessions>")
            .field("emulators", &"<emulators>")
            .field("metrics", &"<SpecialtyRuntimeMetrics>")
            .finish()
    }
}

impl SpecialtyRuntimeEngine {
    /// Create a new specialty hardware runtime engine
    pub fn new(config: SpecialtyRuntimeConfig) -> Self {
        Self {
            config,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            _communication_sessions: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(SpecialtyRuntimeMetrics::default())),
        }
    }

    /// Initialize the legacy runtime engine
    ///
    /// # Errors
    ///
    /// Returns when any subsystem initialization fails.
    pub async fn initialize(&mut self) -> ToadStoolResult<()> {
        info!("Initializing Legacy Runtime Engine");

        if self.config.mainframe_enabled {
            self.initialize_mainframe_adapters().await?;
        }
        if self.config.embedded_enabled {
            self.initialize_embedded_adapters().await?;
        }
        if self.config.industrial_enabled {
            self.initialize_industrial_adapters().await?;
        }
        if self.config.realtime_enabled {
            self.initialize_realtime_adapters().await?;
        }
        if self.config.cross_compilation_enabled {
            self.initialize_cross_compilation_toolchains().await?;
        }
        if self.config.emulation_enabled {
            self.initialize_emulators().await?;
        }

        info!("Legacy Runtime Engine initialized successfully");
        Ok(())
    }

    async fn initialize_mainframe_adapters(&self) -> ToadStoolResult<()> {
        info!("Initializing mainframe adapters");
        let ibm_adapter = mainframe::IBMMainframeAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::IbmSystem360, Arc::new(ibm_adapter));

        let vax_adapter = mainframe::VAXVMSAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::VaxVms, Arc::new(vax_adapter));

        let as400_adapter = mainframe::AS400Adapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::AS400, Arc::new(as400_adapter));

        Ok(())
    }

    async fn initialize_embedded_adapters(&self) -> ToadStoolResult<()> {
        info!("Initializing embedded system adapters");
        let mcu_8bit_adapter = embedded::Microcontroller8BitAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::Intel8080, Arc::new(mcu_8bit_adapter));

        let system_16bit_adapter = embedded::System16BitAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::Intel8086, Arc::new(system_16bit_adapter));

        Ok(())
    }

    async fn initialize_industrial_adapters(&self) -> ToadStoolResult<()> {
        info!("Initializing industrial system adapters");
        let plc_adapter = industrial::PLCAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::PlcLadder, Arc::new(plc_adapter));

        let scada_adapter = industrial::SCADAAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::ScadaSystem, Arc::new(scada_adapter));

        Ok(())
    }

    async fn initialize_realtime_adapters(&self) -> ToadStoolResult<()> {
        info!("Initializing real-time system adapters");
        let vxworks_adapter = realtime::VxWorksAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::VxWorks, Arc::new(vxworks_adapter));

        let qnx_adapter = realtime::QNXAdapter::new();
        self.adapters
            .write()
            .await
            .insert(LegacySystemType::QnxLegacy, Arc::new(qnx_adapter));

        Ok(())
    }

    async fn initialize_cross_compilation_toolchains(&self) -> ToadStoolResult<()> {
        info!("Initializing cross-compilation toolchains");
        let mut toolchains = self.toolchains.write().await;
        for (arch, config) in &self.config.toolchain_configs {
            toolchains.insert(arch.clone(), config.clone());
        }
        Ok(())
    }

    async fn initialize_emulators(&self) -> ToadStoolResult<()> {
        info!("Initializing emulators");
        let pdp11_emulator = emulation::PDP11Emulator::new();
        self.emulators
            .write()
            .await
            .insert(LegacySystemType::PDP11, Box::new(pdp11_emulator));

        let apple2_emulator = emulation::Apple2Emulator::new();
        self.emulators
            .write()
            .await
            .insert(LegacySystemType::AppleIi, Box::new(apple2_emulator));

        Ok(())
    }

    /// Submit a legacy job for execution
    ///
    /// # Errors
    ///
    /// Returns when no adapter exists for the job's target system or submission fails.
    pub async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting legacy job: {:?}", job.job_id);

        let adapters = self.adapters.read().await;
        let adapter = Arc::clone(adapters.get(&job.target_system).ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No adapter found for system type: {:?}",
                job.target_system
            ))
        })?);
        drop(adapters);

        let job_id = adapter.submit_job(job.clone()).await?;
        self.active_jobs.write().await.insert(job_id, job);

        let mut metrics = self.metrics.lock().await;
        metrics.total_jobs += 1;
        metrics.active_jobs += 1;

        Ok(job_id)
    }

    /// Get the status of a legacy job
    ///
    /// # Errors
    ///
    /// Returns when the job is unknown or the adapter lookup fails.
    pub async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        let target_system = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?
            .target_system
            .clone();
        drop(jobs);

        let adapters = self.adapters.read().await;
        let adapter = Arc::clone(adapters.get(&target_system).ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No adapter found for system type: {:?}",
                target_system
            ))
        })?);
        drop(adapters);

        adapter.get_job_status(job_id).await
    }

    /// Cancel a legacy job
    ///
    /// # Errors
    ///
    /// Returns when the job is unknown, the adapter is missing, or cancellation fails.
    pub async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let jobs = self.active_jobs.read().await;
        let target_system = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?
            .target_system
            .clone();
        drop(jobs);

        let adapters = self.adapters.read().await;
        let adapter = Arc::clone(adapters.get(&target_system).ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No adapter found for system type: {:?}",
                target_system
            ))
        })?);
        drop(adapters);

        adapter.cancel_job(job_id).await?;
        self.active_jobs.write().await.remove(&job_id);

        let mut metrics = self.metrics.lock().await;
        metrics.active_jobs = metrics.active_jobs.saturating_sub(1);

        Ok(())
    }

    /// Get legacy job output
    ///
    /// # Errors
    ///
    /// Returns when the job is unknown, the adapter is missing, or output retrieval fails.
    pub async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        let jobs = self.active_jobs.read().await;
        let target_system = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?
            .target_system
            .clone();
        drop(jobs);

        let adapters = self.adapters.read().await;
        let adapter = Arc::clone(adapters.get(&target_system).ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No adapter found for system type: {:?}",
                target_system
            ))
        })?);
        drop(adapters);

        adapter.get_job_output(job_id).await
    }

    /// Get runtime metrics
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok` (metrics are cloned from an async mutex).
    pub async fn get_metrics(&self) -> ToadStoolResult<SpecialtyRuntimeMetrics> {
        let metrics = self.metrics.lock().await;
        Ok(metrics.clone())
    }

    /// Get supported legacy systems
    pub fn get_supported_systems(&self) -> Vec<LegacySystemType> {
        self.config.supported_systems.clone()
    }

    /// Test connectivity to a legacy system
    ///
    /// # Errors
    ///
    /// Returns when no adapter exists for `system_type` or the connectivity check fails.
    pub async fn test_connectivity(&self, system_type: LegacySystemType) -> ToadStoolResult<bool> {
        let adapters = self.adapters.read().await;
        let adapter = Arc::clone(adapters.get(&system_type).ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No adapter found for system type: {:?}",
                system_type
            ))
        })?);
        drop(adapters);

        adapter.test_connectivity().await
    }

    /// Shutdown the specialty hardware runtime engine
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`; reserved for future shutdown validation.
    pub async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down Specialty Hardware Runtime Engine");

        let mut adapters = self.adapters.write().await;
        for (name, adapter) in adapters.iter_mut() {
            if let Some(inner) = Arc::get_mut(adapter) {
                if let Err(e) = inner.shutdown().await {
                    error!("Error shutting down adapter: {}", e);
                }
            } else {
                warn!(
                    "Cannot shutdown adapter {:?}: multiple references held",
                    name
                );
            }
        }
        drop(adapters);

        let mut emulators = self.emulators.write().await;
        for (_, emulator) in emulators.iter_mut() {
            if let Err(e) = emulator.stop().await {
                error!("Error stopping emulator: {}", e);
            }
        }
        drop(emulators);

        info!("Specialty Hardware Runtime Engine shutdown complete");
        Ok(())
    }
}
