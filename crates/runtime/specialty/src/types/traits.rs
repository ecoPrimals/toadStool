// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait definitions for legacy runtime adapters and interfaces

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

use crate::ToadStoolResult;

// Import types from parent modules
use crate::{
    LegacyArchitecture, LegacySystemType, MemoryType, NetworkProtocol, StorageType, SystemStatus,
};
use crate::{LegacyJob, SpecialtyRuntimeConfig};

/// Runtime metrics for specialty hardware systems
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecialtyRuntimeMetrics {
    /// Total jobs executed
    pub total_jobs: u64,
    /// Successful jobs
    pub successful_jobs: u64,
    /// Failed jobs
    pub failed_jobs: u64,
    /// Active jobs
    pub active_jobs: u64,
    /// Average job duration
    pub average_job_duration: Duration,
    /// Total CPU time
    pub total_cpu_time: Duration,
    /// Total memory usage
    pub total_memory_usage: u64,
    /// Communication sessions
    pub communication_sessions: u64,
    /// Error count
    pub error_count: u64,
    /// System uptime
    pub system_uptime: Duration,
}

/// Legacy adapter trait for different legacy systems
///
/// **Uses async_trait for trait object compatibility**
/// - Required for `Box<dyn LegacyAdapter>` and `Arc<dyn LegacyAdapter>`
/// - Enables plugin-style architecture
/// - Necessary for polymorphic legacy system support
#[async_trait::async_trait]
pub trait LegacyAdapter: Send + Sync {
    /// Get the adapter name
    fn name(&self) -> &str;

    /// Get supported legacy system types
    fn supported_systems(&self) -> Vec<LegacySystemType>;

    /// Initialize the adapter
    async fn initialize(&mut self, config: &SpecialtyRuntimeConfig) -> ToadStoolResult<()>;

    /// Shutdown the adapter
    async fn shutdown(&mut self) -> ToadStoolResult<()>;

    /// Submit a legacy job
    async fn submit_job(&self, job: LegacyJob) -> ToadStoolResult<Uuid>;

    /// Get job status
    async fn get_job_status(&self, job_id: Uuid) -> ToadStoolResult<JobStatus>;

    /// Cancel a job
    async fn cancel_job(&self, job_id: Uuid) -> ToadStoolResult<()>;

    /// Get job output
    async fn get_job_output(&self, job_id: Uuid) -> ToadStoolResult<JobOutput>;

    /// Get system information
    async fn get_system_info(&self) -> ToadStoolResult<SystemInfo>;

    /// Test connectivity
    async fn test_connectivity(&self) -> ToadStoolResult<bool>;
}

/// Job status for legacy systems
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is queued
    Queued,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed { error: String },
    /// Job was cancelled
    Cancelled,
    /// Job timed out
    TimedOut,
}

/// Job output for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Return code
    pub return_code: Option<i32>,
    /// Output files
    pub output_files: Vec<OutputFile>,
    /// Binary output
    pub binary_output: Option<Vec<u8>>,
}

/// Output file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// File name
    pub name: String,
    /// File path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// File type
    pub file_type: String,
    /// File content (for small files)
    pub content: Option<Vec<u8>>,
}

/// System information for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// System name
    pub system_name: String,
    /// System type
    pub system_type: LegacySystemType,
    /// System version
    pub version: String,
    /// Architecture
    pub architecture: LegacyArchitecture,
    /// CPU information
    pub cpu_info: CpuInfo,
    /// Memory information
    pub memory_info: MemoryInfo,
    /// Storage information
    pub storage_info: StorageInfo,
    /// Network information
    pub network_info: NetworkInfo,
    /// System status
    pub status: SystemStatus,
}

/// CPU information for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU model
    pub model: String,
    /// CPU speed
    pub speed: u64,
    /// Number of cores
    pub cores: u32,
    /// CPU features
    pub features: Vec<String>,
    /// CPU usage
    pub usage: f64,
}

/// Memory information for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// Total memory
    pub total: u64,
    /// Available memory
    pub available: u64,
    /// Used memory
    pub used: u64,
    /// Memory type
    pub memory_type: MemoryType,
}

/// Storage information for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Total storage
    pub total: u64,
    /// Available storage
    pub available: u64,
    /// Used storage
    pub used: u64,
    /// Storage type
    pub storage_type: StorageType,
}

/// Network information for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network interfaces
    pub interfaces: Vec<NetworkInterface>,
    /// Network protocols
    pub protocols: Vec<NetworkProtocol>,
    /// Network status
    pub status: NetworkStatus,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// Interface type
    pub interface_type: String,
    /// MAC address
    pub mac_address: String,
    /// IP address
    pub ip_address: Option<String>,
    /// Status
    pub status: InterfaceStatus,
}

/// Network interface status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterfaceStatus {
    /// Interface is up
    Up,
    /// Interface is down
    Down,
    /// Interface is unknown
    Unknown,
}

/// Network status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkStatus {
    /// Network is online
    Online,
    /// Network is offline
    Offline,
    /// Network is limited
    Limited,
    /// Network status is unknown
    Unknown,
}

/// Communication session trait for legacy systems
///
/// **Uses async_trait for trait object compatibility**
/// - Required for `Box<dyn LegacyCommunicationSession>`
/// - Enables polymorphic session management
#[async_trait::async_trait]
pub trait LegacyCommunicationSession: Send + Sync {
    /// Send a command to the legacy system
    async fn send_command(&mut self, command: &str) -> ToadStoolResult<String>;

    /// Check if session is connected
    fn is_connected(&self) -> bool;

    /// Close the session
    async fn close(&mut self) -> ToadStoolResult<()>;
}
