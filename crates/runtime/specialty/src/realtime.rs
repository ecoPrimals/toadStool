// SPDX-License-Identifier: AGPL-3.0-only
//! # Real-time Systems Adapters
//!
//! Support for real-time operating systems including:
//! - VxWorks
//! - QNX
//! - RT-11
//! - RTOS-32
//! - Real-time scheduling
//! - Interrupt handling
//! - Task management

// Migrated to native async traits
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::{
    JobOutput, JobStatus, LegacyAdapter, LegacyJob, LegacySystemType, RealtimeConfig, RealtimeOS,
    SpecialtyRuntimeConfig, SystemInfo, ToadStoolError, ToadStoolResult,
};

/// VxWorks Adapter
#[derive(Debug)]
pub struct VxWorksAdapter {
    config: Option<RealtimeConfig>,
    active_jobs: Arc<RwLock<HashMap<Uuid, RealtimeJob>>>,
}

/// QNX Adapter
#[derive(Debug)]
pub struct QNXAdapter {
    config: Option<RealtimeConfig>,
    active_jobs: Arc<RwLock<HashMap<Uuid, RealtimeJob>>>,
}

/// Real-time Job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeJob {
    pub job_id: Uuid,
    pub task_name: String,
    pub priority: u8,
    pub status: JobStatus,
}

impl Default for VxWorksAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl VxWorksAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for QNXAdapter {
    fn default() -> Self {
        Self {
            config: None,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl QNXAdapter {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl LegacyAdapter for VxWorksAdapter {
    fn name(&self) -> &str {
        "VxWorks Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::VxWorks]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing VxWorks adapter");
        for (name, realtime_config) in &config.realtime_configs {
            if matches!(realtime_config.rtos, RealtimeOS::VxWorks) {
                self.config = Some(realtime_config.clone());
                info!("Found VxWorks configuration: {}", name);
                break;
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down VxWorks adapter");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        let rt_job = RealtimeJob {
            job_id: job.job_id,
            task_name: "VxWorks Task".to_string(),
            priority: 100,
            status: JobStatus::Queued,
        };

        self.active_jobs.write().await.insert(job.job_id, rt_job);
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
            stdout: "VxWorks execution output".to_string(),
            stderr: String::new(),
            return_code: Some(0),
            output_files: vec![],
            binary_output: None,
        })
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "VxWorks".to_string(),
            system_type: LegacySystemType::VxWorks,
            version: "6.9".to_string(),
            architecture: crate::LegacyArchitecture::IntelI386,
            cpu_info: crate::CpuInfo {
                model: "Real-time CPU".to_string(),
                speed: 500_000_000, // 500 MHz
                cores: 1,
                features: vec!["Real-time".to_string()],
                usage: 5.0,
            },
            memory_info: crate::MemoryInfo {
                total: 64 * 1024 * 1024,     // 64MB
                available: 32 * 1024 * 1024, // 32MB
                used: 32 * 1024 * 1024,      // 32MB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 512 * 1024 * 1024,     // 512MB
                available: 256 * 1024 * 1024, // 256MB
                used: 256 * 1024 * 1024,      // 256MB
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
impl LegacyAdapter for QNXAdapter {
    fn name(&self) -> &str {
        "QNX Adapter"
    }

    fn supported_systems(&self) -> Vec<LegacySystemType> {
        vec![LegacySystemType::QnxLegacy]
    }

    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()> {
        info!("Initializing QNX adapter");
        for (name, realtime_config) in &config.realtime_configs {
            if matches!(realtime_config.rtos, RealtimeOS::QNX) {
                self.config = Some(realtime_config.clone());
                info!("Found QNX configuration: {}", name);
                break;
            }
        }
        Ok(())
    }

    async fn shutdown(&mut self) -> ToadStoolResult<()> {
        info!("Shutting down QNX adapter");
        Ok(())
    }

    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid> {
        let rt_job = RealtimeJob {
            job_id: job.job_id,
            task_name: "QNX Process".to_string(),
            priority: 10,
            status: JobStatus::Queued,
        };

        self.active_jobs.write().await.insert(job.job_id, rt_job);
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
            stdout: "QNX execution output".to_string(),
            stderr: String::new(),
            return_code: Some(0),
            output_files: vec![],
            binary_output: None,
        })
    }

    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo> {
        Ok(SystemInfo {
            system_name: "QNX".to_string(),
            system_type: LegacySystemType::QnxLegacy,
            version: "6.5".to_string(),
            architecture: crate::LegacyArchitecture::IntelI386,
            cpu_info: crate::CpuInfo {
                model: "QNX Real-time CPU".to_string(),
                speed: 400_000_000, // 400 MHz
                cores: 1,
                features: vec!["Real-time".to_string(), "Microkernel".to_string()],
                usage: 8.0,
            },
            memory_info: crate::MemoryInfo {
                total: 32 * 1024 * 1024,     // 32MB
                available: 16 * 1024 * 1024, // 16MB
                used: 16 * 1024 * 1024,      // 16MB
                memory_type: crate::MemoryType::RAM,
            },
            storage_info: crate::StorageInfo {
                total: 256 * 1024 * 1024,     // 256MB
                available: 128 * 1024 * 1024, // 128MB
                used: 128 * 1024 * 1024,      // 128MB
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
