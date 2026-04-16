// SPDX-License-Identifier: AGPL-3.0-or-later
//! AS/400 System Adapter

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

use super::types::{IFSManager, MainframeJob, RPGCompiler, Terminal5250};

mod compiler;
mod connection;
mod jobs;
mod terminal;

use crate::{JobOutput, JobStatus, SpecialtyRuntimeConfig};
use crate::{
    LegacyAdapter, LegacyJob, LegacySystemType, MainframeConfig, SystemInfo, ToadStoolError,
    ToadStoolResult,
};

/// AS/400 Adapter
#[derive(Debug)]
pub struct AS400Adapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// RPG compiler
    _rpg_compiler: Arc<RPGCompiler>,
    /// 5250 terminal emulator
    _terminal_emulator: Arc<Mutex<Option<Terminal5250>>>,
    /// IFS (Integrated File System) manager
    _ifs_manager: Arc<IFSManager>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for AS400Adapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            _rpg_compiler: Arc::new(RPGCompiler::new()),
            _terminal_emulator: Arc::new(Mutex::new(None)),
            _ifs_manager: Arc::new(IFSManager::new()),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl AS400Adapter {
    /// Create a new AS/400 adapter
    pub fn new() -> Self {
        Self::default()
    }
}

impl LegacyAdapter for AS400Adapter {
    fn name(&self) -> &'static str {
        "AS/400 Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::AS400]
    }

    fn initialize<'a>(
        &'a mut self,
        config: &'a SpecialtyRuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async {
            info!("Initializing AS/400 adapter");

            // Find AS/400 configuration
            for (name, mainframe_config) in &config.mainframe_configs {
                if mainframe_config.system_type == LegacySystemType::AS400 {
                    self.config = Some(mainframe_config.clone());
                    info!("Found AS/400 configuration: {}", name);
                    break;
                }
            }

            if self.config.is_none() {
                return Err(ToadStoolError::runtime("No AS/400 configuration found"));
            }

            *self.connected.lock().await = true;

            info!("AS/400 adapter initialized successfully");
            Ok(())
        })
    }

    fn shutdown<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async {
            info!("Shutting down AS/400 adapter");

            *self.connected.lock().await = false;

            info!("AS/400 adapter shutdown complete");
            Ok(())
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )] // label uses low bits of UUID only
    fn submit_job(
        &self,
        job: LegacyJob,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Uuid>> + Send + '_>> {
        Box::pin(async move {
            info!("Submitting job to AS/400: {:?}", job.job_id);

            // Create mainframe job
            let mainframe_job = MainframeJob {
                job_id: job.job_id,
                job_name: format!("AS400JOB{:08X}", job.job_id.as_u128() as u32),
                job_class: "A".to_string(),
                priority: job.priority,
                jcl_content: "// AS/400 Job".to_string(),
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

            info!("Job submitted to AS/400: {}", job.job_id);
            Ok(job.job_id)
        })
    }

    fn get_job_status(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<JobStatus>> + Send + '_>> {
        Box::pin(async move {
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
        })
    }

    fn cancel_job(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            let mut jobs = self.active_jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Cancelled;
                info!("Cancelled AS/400 job: {}", job_id);
                Ok(())
            } else {
                Err(ToadStoolError::runtime(format!(
                    "Job not found: {}",
                    job_id
                )))
            }
        })
    }

    fn get_job_output(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<JobOutput>> + Send + '_>> {
        Box::pin(async move {
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
        })
    }

    fn get_system_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemInfo>> + Send + '_>> {
        Box::pin(async {
            // In a real implementation, this would query the AS/400 system
            Ok(SystemInfo {
                system_name: "IBM AS/400".to_string(),
                system_type: LegacySystemType::AS400,
                version: "V7R4".to_string(),
                architecture: crate::LegacyArchitecture::PowerPc601,
                cpu_info: crate::CpuInfo {
                    model: "POWER8".to_string(),
                    speed: 3_000_000_000, // 3 GHz
                    cores: 8,
                    features: vec!["POWER8 instruction set".to_string()],
                    usage: 30.0,
                },
                memory_info: crate::MemoryInfo {
                    total: 16 * 1024 * 1024 * 1024,    // 16 GB
                    available: 8 * 1024 * 1024 * 1024, // 8 GB
                    used: 8 * 1024 * 1024 * 1024,      // 8 GB
                    memory_type: crate::MemoryType::RAM,
                },
                storage_info: crate::StorageInfo {
                    total: 1024 * 1024 * 1024 * 1024,    // 1 TB
                    available: 512 * 1024 * 1024 * 1024, // 512 GB
                    used: 512 * 1024 * 1024 * 1024,      // 512 GB
                    storage_type: crate::StorageType::HardDisk,
                },
                network_info: crate::NetworkInfo {
                    interfaces: vec![],
                    protocols: vec![crate::NetworkProtocol::TCPIP],
                    status: crate::NetworkStatus::Online,
                },
                status: crate::SystemStatus::Online,
            })
        })
    }

    fn test_connectivity(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + '_>> {
        Box::pin(async {
            let connected = self.connected.lock().await;
            Ok(*connected)
        })
    }
}

#[cfg(test)]
mod as400_tests;
