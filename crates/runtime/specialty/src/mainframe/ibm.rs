// SPDX-License-Identifier: AGPL-3.0-only
//! IBM Mainframe Adapter (System/360, System/370, z/Series)

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
use toadstool::JobPriority;

/// IBM Mainframe Adapter for System/360, System/370, z/Series
#[derive(Debug)]
pub struct IBMMainframeAdapter {
    /// Adapter configuration
    config: Option<MainframeConfig>,
    /// Active jobs
    active_jobs: Arc<RwLock<HashMap<Uuid, MainframeJob>>>,
    /// JCL generator
    jcl_generator: Arc<Mutex<JCLGenerator>>,
    /// COBOL compiler interface
    cobol_compiler: Arc<Mutex<COBOLCompiler>>,
    /// 3270 terminal emulator
    terminal_emulator: Arc<Mutex<Option<Terminal3270>>>,
    /// Dataset manager
    dataset_manager: Arc<Mutex<DatasetManager>>,
    /// Connection status
    connected: Arc<Mutex<bool>>,
}

impl Default for IBMMainframeAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
            jcl_generator: Arc::new(Mutex::new(JCLGenerator::new())),
            cobol_compiler: Arc::new(Mutex::new(COBOLCompiler::new())),
            terminal_emulator: Arc::new(Mutex::new(None)),
            dataset_manager: Arc::new(Mutex::new(DatasetManager::new())),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

impl IBMMainframeAdapter {
    /// Create a new IBM Mainframe adapter
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate JCL for a job
    async fn generate_jcl(&self, job: &LegacyJob) -> ToadStoolResult<String> {
        self.jcl_generator.lock().await.generate_jcl(job).await
    }

    /// Submit JCL job
    async fn submit_jcl_job(&self, jcl: &str) -> ToadStoolResult<Uuid> {
        let job_id = Uuid::new_v4();
        let job_name = format!("JOB{:08X}", job_id.as_u128() as u32);

        let mainframe_job = MainframeJob {
            job_id,
            job_name: job_name.clone(),
            job_class: "A".to_string(),
            priority: JobPriority::Normal,
            jcl_content: jcl.to_string(),
            status: JobStatus::Queued,
            start_time: None,
            end_time: None,
            output_datasets: vec![],
            return_code: None,
            job_log: String::new(),
        };

        self.active_jobs.write().await.insert(job_id, mainframe_job);

        // In a real implementation, this would submit to the mainframe job queue
        info!("Submitted JCL job {} to mainframe", job_name);

        Ok(job_id)
    }

    /// Connect to mainframe via 3270 terminal
    async fn connect_3270(&self) -> ToadStoolResult<()> {
        if let Some(ref config) = self.config {
            let mut term_3270 = Terminal3270::new();
            term_3270.connect(&config.connection).await?;
            *self.terminal_emulator.lock().await = Some(term_3270);

            *self.connected.lock().await = true;

            info!("Connected to IBM mainframe via 3270 terminal");
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for IBMMainframeAdapter {
    fn name(&self) -> &'static str {
        "IBM Mainframe Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![
            LegacySystemType::IbmSystem360,
            LegacySystemType::IbmSystem370,
            LegacySystemType::IbmZSeries,
        ]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing IBM Mainframe adapter");

        // Find mainframe configuration
        for (name, mainframe_config) in &config.mainframe_configs {
            if mainframe_config.system_type == LegacySystemType::IbmSystem360
                || mainframe_config.system_type == LegacySystemType::IbmSystem370
                || mainframe_config.system_type == LegacySystemType::IbmZSeries
            {
                self.config = Some(mainframe_config.clone());
                info!("Found IBM mainframe configuration: {}", name);
                break;
            }
        }

        if self.config.is_none() {
            return Err(ToadStoolError::runtime(
                "No IBM mainframe configuration found",
            ));
        }

        // Initialize components - config must be initialized before these calls
        let config = self.config.as_ref().ok_or_else(|| {
            ToadStoolError::configuration("Mainframe adapter config not initialized")
        })?;

        self.jcl_generator
            .lock()
            .await
            .initialize(&config.jcl_settings)
            .await?;
        self.cobol_compiler
            .lock()
            .await
            .initialize(&config.cobol_settings)
            .await?;
        self.dataset_manager
            .lock()
            .await
            .initialize(&config.datasets)
            .await?;

        // Connect to mainframe
        self.connect_3270().await?;

        info!("IBM Mainframe adapter initialized successfully");
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down IBM Mainframe adapter");

        // Disconnect terminal
        let mut terminal = self.terminal_emulator.lock().await;
        if let Some(ref mut term) = *terminal {
            term.disconnect().await?;
        }
        *terminal = None;
        drop(terminal);

        *self.connected.lock().await = false;

        info!("IBM Mainframe adapter shutdown complete");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        info!("Submitting job to IBM mainframe: {:?}", job.job_id);

        // Generate JCL for the job
        let jcl = self.generate_jcl(&job).await?;

        // Submit JCL job
        let job_id = self.submit_jcl_job(&jcl).await?;

        info!("Job submitted to IBM mainframe: {}", job_id);
        Ok(job_id)
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
            info!("Cancelled IBM mainframe job: {}", job_id);
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
        // In a real implementation, this would query the mainframe system
        Ok(SystemInfo {
            system_name: "IBM z/OS".to_string(),
            system_type: LegacySystemType::IbmZSeries,
            version: "2.4".to_string(),
            architecture: crate::LegacyArchitecture::IbmSystem360,
            cpu_info: crate::CpuInfo {
                model: "IBM z14".to_string(),
                speed: 5_200_000_000, // 5.2 GHz
                cores: 32,
                features: vec!["z/Architecture".to_string()],
                usage: 25.0,
            },
            memory_info: crate::MemoryInfo {
                total: 1024 * 1024 * 1024 * 1024,    // 1 TB
                available: 512 * 1024 * 1024 * 1024, // 512 GB
                used: 512 * 1024 * 1024 * 1024,      // 512 GB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 100 * 1024 * 1024 * 1024 * 1024,    // 100 TB
                available: 50 * 1024 * 1024 * 1024 * 1024, // 50 TB
                used: 50 * 1024 * 1024 * 1024 * 1024,      // 50 TB
                storage_type: crate::StorageType::HardDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::TCPIP],
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

// Implementation for VAX/VMS Adapter
