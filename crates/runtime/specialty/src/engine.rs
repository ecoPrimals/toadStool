// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};
use uuid::Uuid;

use crate::config::SpecialtyRuntimeConfig;
use crate::types::configs::CompilationToolchainConfig as ToolchainConfig;
use crate::types::systems::{LegacyArchitecture, LegacySystemType};
use crate::types::emulation::LegacyEmulator;
use crate::types::jobs::LegacyJob;
use crate::types::traits::{
    JobOutput, JobStatus, LegacyAdapter, LegacyCommunicationSession,
    SpecialtyRuntimeMetrics,
};
use toadstool::{ToadStoolError, ToadStoolResult};

use super::emulation;
use super::embedded;
use super::industrial;
use super::mainframe;
use super::realtime;

#[derive(Debug)]
pub struct SpecialtyRuntimeEngine {
    pub(super) config: SpecialtyRuntimeConfig,
    pub(super) adapters: Arc<RwLock<HashMap<LegacySystemType, Box<dyn LegacyAdapter>>>>,
    pub(super) toolchains: Arc<RwLock<HashMap<LegacyArchitecture, ToolchainConfig>>>,
    pub(super) active_jobs: Arc<RwLock<HashMap<Uuid, LegacyJob>>>,
    pub(super) communication_sessions: Arc<RwLock<HashMap<Uuid, Box<dyn LegacyCommunicationSession>>>>,
    pub(super) emulators: Arc<RwLock<HashMap<LegacySystemType, Box<dyn LegacyEmulator>>>>,
    pub(super) metrics: Arc<Mutex<SpecialtyRuntimeMetrics>>,
}

impl SpecialtyRuntimeEngine {
    pub fn new(config: SpecialtyRuntimeConfig) -> Self {
        Self {
            config,
            adapters: Arc::new(RwLock::new(HashMap::new())),
            toolchains: Arc::new(RwLock::new(HashMap::new())),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            communication_sessions: Arc::new(RwLock::new(HashMap::new())),
            emulators: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(SpecialtyRuntimeMetrics::default())),
        }
    }

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

    async fn initialize_mainframe_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing mainframe adapters");
        let mut adapters = self.adapters.write().await;
        adapters.insert(LegacySystemType::IbmSystem360, Box::new(mainframe::IBMMainframeAdapter::new()));
        adapters.insert(LegacySystemType::VaxVms, Box::new(mainframe::VAXVMSAdapter::new()));
        adapters.insert(LegacySystemType::AS400, Box::new(mainframe::AS400Adapter::new()));
        Ok(())
    }

    async fn initialize_embedded_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing embedded system adapters");
        let mut adapters = self.adapters.write().await;
        adapters.insert(LegacySystemType::Intel8080, Box::new(embedded::Microcontroller8BitAdapter::new()));
        adapters.insert(LegacySystemType::Intel8086, Box::new(embedded::System16BitAdapter::new()));
        Ok(())
    }

    async fn initialize_industrial_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing industrial system adapters");
        let mut adapters = self.adapters.write().await;
        adapters.insert(LegacySystemType::PlcLadder, Box::new(industrial::PLCAdapter::new()));
        adapters.insert(LegacySystemType::ScadaSystem, Box::new(industrial::SCADAAdapter::new()));
        Ok(())
    }

    async fn initialize_realtime_adapters(&mut self) -> ToadStoolResult<()> {
        info!("Initializing real-time system adapters");
        let mut adapters = self.adapters.write().await;
        adapters.insert(LegacySystemType::VxWorks, Box::new(realtime::VxWorksAdapter::new()));
        adapters.insert(LegacySystemType::QnxLegacy, Box::new(realtime::QNXAdapter::new()));
        Ok(())
    }

    async fn initialize_cross_compilation_toolchains(&mut self) -> ToadStoolResult<()> {
        info!("Initializing cross-compilation toolchains");
        let mut toolchains = self.toolchains.write().await;
        for (arch, config) in &self.config.toolchain_configs {
            toolchains.insert(*arch, config.clone());
        }
        Ok(())
    }

    async fn initialize_emulators(&mut self) -> ToadStoolResult<()> {
        info!("Initializing emulators");
        let mut emulators = self.emulators.write().await;
        emulators.insert(LegacySystemType::PDP11, Box::new(emulation::PDP11Emulator::new()));
        emulators.insert(LegacySystemType::AppleIi, Box::new(emulation::Apple2Emulator::new()));
        Ok(())
    }

    pub async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting legacy job: {:?}", job.job_id);
        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        let job_id = adapter.submit_job(job.clone()).await?;
        drop(adapters);
        self.active_jobs.write().await.insert(job_id, job);
        let mut metrics = self.metrics.lock().await;
        metrics.total_jobs += 1;
        metrics.active_jobs += 1;
        Ok(job_id)
    }

    pub async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        adapter.get_job_status(job_id).await
    }

    pub async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let jobs = self.active_jobs.read().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        adapter.cancel_job(job_id).await?;
        drop(jobs);
        self.active_jobs.write().await.remove(&job_id);
        let mut metrics = self.metrics.lock().await;
        metrics.active_jobs = metrics.active_jobs.saturating_sub(1);
        Ok(())
    }

    pub async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        let jobs = self.active_jobs.read().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| ToadStoolError::runtime(format!("Job not found: {}", job_id)))?;
        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&job.target_system)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", job.target_system)))?;
        adapter.get_job_output(job_id).await
    }

    pub async fn get_metrics(&self) -> ToadStoolResult<SpecialtyRuntimeMetrics> {
        let metrics = self.metrics.lock().await;
        Ok(metrics.clone())
    }

    pub fn get_supported_systems(&self) -> Vec<LegacySystemType> {
        self.config.supported_systems.clone()
    }

    pub async fn test_connectivity(&self, system_type: LegacySystemType) -> ToadStoolResult<bool> {
        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&system_type)
            .ok_or_else(|| ToadStoolError::runtime(format!("No adapter found for system type: {:?}", system_type)))?;
        adapter.test_connectivity().await
    }

    pub async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down Specialty Hardware Runtime Engine");
        let mut adapters = self.adapters.write().await;
        for (_, adapter) in adapters.iter_mut() {
            if let Err(e) = adapter.shutdown().await {
                error!("Error shutting down adapter: {}", e);
            }
        }
        let mut emulators = self.emulators.write().await;
        for (_, emulator) in emulators.iter_mut() {
            if let Err(e) = emulator.stop().await {
                error!("Error stopping emulator: {}", e);
            }
        }
        info!("Specialty Hardware Runtime Engine shutdown complete");
        Ok(())
    }
}
