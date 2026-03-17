// SPDX-License-Identifier: AGPL-3.0-only
//! VAX/VMS System Adapter

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

use super::types::*;
use crate::{JobOutput, JobStatus, SpecialtyRuntimeConfig};
use crate::{
    LegacyAdapter, LegacyJob, LegacySystemType, MainframeConfig, SystemInfo, ToadStoolError,
    ToadStoolResult,
};

/// VAX/VMS Adapter
#[derive(Debug)]
pub struct VAXVMSAdapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// DCL command processor
    _dcl_processor: Arc<DCLProcessor>,
    /// VAX FORTRAN compiler
    _fortran_compiler: Arc<VAXFortranCompiler>,
    /// Terminal interface
    _terminal_interface: Arc<Mutex<Option<VAXTerminal>>>,
    /// File system manager
    _file_system: Arc<VMSFileSystem>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for VAXVMSAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            _dcl_processor: Arc::new(DCLProcessor::new()),
            _fortran_compiler: Arc::new(VAXFortranCompiler::new()),
            _terminal_interface: Arc::new(Mutex::new(None)),
            _file_system: Arc::new(VMSFileSystem::new()),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl VAXVMSAdapter {
    /// Create a new VAX/VMS adapter
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for VAXVMSAdapter {
    fn name(&self) -> &str {
        "VAX/VMS Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::VaxVms]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing VAX/VMS adapter");

        // Find VAX/VMS configuration
        for (name, mainframe_config) in &config.mainframe_configs {
            if mainframe_config.system_type == LegacySystemType::VaxVms {
                self.config = Some(mainframe_config.clone());
                info!("Found VAX/VMS configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime("No VAX/VMS configuration found"));
        }

        *self.connected.lock().await = true;

        info!("VAX/VMS adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down VAX/VMS adapter");

        *self.connected.lock().await = false;

        info!("VAX/VMS adapter shutdown complete");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to VAX/VMS: {:?}", job.job_id);

        // Create mainframe job
        let mainframe_job = MainframeJob {
            job_id: job.job_id,
            job_name: format!("VAXJOB{:08X}", job.job_id.as_u128() as u32),
            job_class: "A".to_string(),
            priority: job.priority,
            jcl_content: "! VAX/VMS DCL Job".to_string(),
            status: JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };

        self.active_jobs
            .write()
            .await
            .insert(job.job_id, mainframe_job);

        info!("Job submitted to VAX/VMS: {}", job.job_id);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        jobs.get(&job_id).map_or_else(
            || {
                Err(ToadStoolError::runtime(format!(
                    "Job not found: {}",
                    job_id
                )))
            },
            |job| Ok(job.status.clone()),
        )
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
            info!("Cancelled VAX/VMS job: {}", job_id);
            Ok(())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput> {
        let jobs = self.active_jobs.read().await;
        jobs.get(&job_id).map_or_else(
            || {
                Err(ToadStoolError::runtime(format!(
                    "Job not found: {}",
                    job_id
                )))
            },
            |job| {
                Ok(JobOutput {
                    stdout: job.job_log.clone(),
                    stderr: String::new(),
                    return_code: job.return_code,
                    output_files: vec![],
                    binary_output: None,
                })
            },
        )
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        // In a real implementation, this would query the VAX/VMS system
        Ok(SystemInfo {
            system_name: "VAX/VMS".to_string(),
            system_type: LegacySystemType::VaxVms,
            version: "7.3".to_string(),
            architecture: crate::LegacyArchitecture::VAX,
            cpu_info: crate::CpuInfo {
                model: "VAX-11/780".to_string(),
                speed: 5_000_000, // 5 MHz
                cores: 1,
                features: vec!["VAX instruction set".to_string()],
                usage: 15.0,
            },
            memory_info: crate::MemoryInfo {
                total: 8 * 1024 * 1024,     // 8 MB
                available: 4 * 1024 * 1024, // 4 MB
                used: 4 * 1024 * 1024,      // 4 MB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 300 * 1024 * 1024,     // 300 MB
                available: 150 * 1024 * 1024, // 150 MB
                used: 150 * 1024 * 1024,      // 150 MB
                storage_type: crate::StorageType::HardDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::DECnet],
                status: crate::NetworkStatus::Online,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        let connected = self.connected.lock().await;
        Ok(*connected)
    }
}

// Implementation for AS/400 Adapter
