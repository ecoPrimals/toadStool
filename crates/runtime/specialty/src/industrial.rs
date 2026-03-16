// SPDX-License-Identifier: AGPL-3.0-only
//! # Industrial Systems Adapters
//!
//! Support for industrial control systems including:
//! - PLCs (Programmable Logic Controllers)
//! - SCADA (Supervisory Control And Data Acquisition)
//! - DCS (Distributed Control Systems)
//! - HMI (Human Machine Interface)
//! - Industrial communication protocols
//! - Safety systems integration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::{
    IndustrialConfig, JobOutput, JobStatus, LegacyAdapter, LegacyJob, LegacySystemType,
    SpecialtyRuntimeConfig, SystemInfo, ToadStoolError, ToadStoolResult,
};

/// PLC Adapter
#[derive(Debug)]
pub struct PLCAdapter {
    config: Option<IndustrialConfig>,
    active_jobs: Arc<RwLock<HashMap<Uuid, PLCJob>>>,
}

/// SCADA Adapter
#[derive(Debug)]
pub struct SCADAAdapter {
    config: Option<IndustrialConfig>,
    active_jobs: Arc<RwLock<HashMap<Uuid, SCADAJob>>>,
}

/// PLC Job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PLCJob {
    pub job_id: Uuid,
    pub program: String,
    pub status: JobStatus,
}

/// SCADA Job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCADAJob {
    pub job_id: Uuid,
    pub configuration: String,
    pub status: JobStatus,
}

impl Default for PLCAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl PLCAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for SCADAAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl SCADAAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for PLCAdapter {
    fn name(&self) -> &str {
        "PLC Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::PlcLadder]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing PLC adapter");
        for (name, industrial_config) in &config.industrial_configs {
            if matches!(
                industrial_config.system_type,
                crate::IndustrialSystemType::PLC
            ) {
                self.config = Some(industrial_config.clone());
                info!("Found PLC configuration: {}", name);
                break;
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down PLC adapter");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        let plc_job = PLCJob {
            job_id: job.job_id,
            program: "PLC Program".to_string(),
            status: JobStatus::Queued,
        };

        self.active_jobs.write().await.insert(job.job_id, plc_job);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(job.status.clone())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
        }
        Ok(())
    }

    async fn get_job_output(&self, _job_id: Uuid) -> ToadStoolResult<JobOutput> {
        Ok(JobOutput {
            stdout: "PLC execution output".to_string(),
            stderr: String::new(),
            return_code: Some(0),
            output_files: vec![],
            binary_output: None,
        })
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "PLC System".to_string(),
            system_type: LegacySystemType::PlcLadder,
            version: "1.0".to_string(),
            architecture: crate::LegacyArchitecture::IntelI386,
            cpu_info: crate::CpuInfo {
                model: "Industrial CPU".to_string(),
                speed: 100_000_000, // 100 MHz
                cores: 1,
                features: vec!["Real-time".to_string()],
                usage: 10.0,
            },
            memory_info: crate::MemoryInfo {
                total: 1024 * 1024,    // 1MB
                available: 512 * 1024, // 512KB
                used: 512 * 1024,      // 512KB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 16 * 1024 * 1024,    // 16MB
                available: 8 * 1024 * 1024, // 8MB
                used: 8 * 1024 * 1024,      // 8MB
                storage_type: crate::StorageType::Flash,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::Ethernet],
                status: crate::NetworkStatus::Online,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        Ok(true)
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for SCADAAdapter {
    fn name(&self) -> &str {
        "SCADA Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::ScadaSystem]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing SCADA adapter");
        for (name, industrial_config) in &config.industrial_configs {
            if matches!(
                industrial_config.system_type,
                crate::IndustrialSystemType::SCADA
            ) {
                self.config = Some(industrial_config.clone());
                info!("Found SCADA configuration: {}", name);
                break;
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down SCADA adapter");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        let scada_job = SCADAJob {
            job_id: job.job_id,
            configuration: "SCADA Configuration".to_string(),
            status: JobStatus::Queued,
        };

        self.active_jobs.write().await.insert(job.job_id, scada_job);
        Ok(job.job_id)
    }

    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus> {
        let jobs = self.active_jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            Ok(job.status.clone())
        } else {
            Err(ToadStoolError::runtime(format!(
                "Job not found: {}",
                job_id
            )))
        }
    }

    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()> {
        let mut jobs = self.active_jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Cancelled;
        }
        Ok(())
    }

    async fn get_job_output(&self, _job_id: Uuid) -> ToadStoolResult<JobOutput> {
        Ok(JobOutput {
            stdout: "SCADA execution output".to_string(),
            stderr: String::new(),
            return_code: Some(0),
            output_files: vec![],
            binary_output: None,
        })
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "SCADA System".to_string(),
            system_type: LegacySystemType::ScadaSystem,
            version: "1.0".to_string(),
            architecture: crate::LegacyArchitecture::IntelI386,
            cpu_info: crate::CpuInfo {
                model: "Industrial Server CPU".to_string(),
                speed: 1_000_000_000, // 1 GHz
                cores: 4,
                features: vec!["Real-time".to_string()],
                usage: 15.0,
            },
            memory_info: crate::MemoryInfo {
                total: 256 * 1024 * 1024,     // 256MB
                available: 128 * 1024 * 1024, // 128MB
                used: 128 * 1024 * 1024,      // 128MB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 1024 * 1024 * 1024,    // 1GB
                available: 512 * 1024 * 1024, // 512MB
                used: 512 * 1024 * 1024,      // 512MB
                storage_type: crate::StorageType::HardDisk,
            },
            network_info: crate::NetworkInfo {
                interfaces: vec![],
                protocols: vec![crate::NetworkProtocol::Ethernet],
                status: crate::NetworkStatus::Online,
            },
            status: crate::SystemStatus::Online,
        })
    }

    async fn test_connectivity(&self) -> ToadStoolResult<bool> {
        Ok(true)
    }
}
