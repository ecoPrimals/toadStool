// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait definitions for legacy runtime adapters and interfaces

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
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
/// Async methods return `Pin<Box<dyn Future>>` for `dyn LegacyAdapter` compatibility.
pub trait LegacyAdapter: Send + Sync {
    /// Get the adapter name
    fn name(&self) -> &'static str;

    /// Get supported legacy system types
    fn supported_systems(&self) -> Vec<LegacySystemType>;

    /// Initialize the adapter
    fn initialize<'a>(
        &'a mut self,
        config: &'a SpecialtyRuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Shutdown the adapter
    fn shutdown<'a>(&'a mut self)
    -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Submit a legacy job
    fn submit_job(
        &self,
        job: LegacyJob,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Uuid>> + Send + '_>>;

    /// Get job status
    fn get_job_status(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<JobStatus>> + Send + '_>>;

    /// Cancel a job
    fn cancel_job(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get job output
    fn get_job_output(
        &self,
        job_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<JobOutput>> + Send + '_>>;

    /// Get system information
    fn get_system_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<SystemInfo>> + Send + '_>>;

    /// Test connectivity
    fn test_connectivity(&self)
    -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + '_>>;
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
    /// Job failed with error.
    Failed {
        /// Error message.
        error: String,
    },
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
/// Async methods return `Pin<Box<dyn Future>>` for `dyn LegacyCommunicationSession` compatibility.
pub trait LegacyCommunicationSession: Send + Sync {
    /// Send a command to the legacy system
    fn send_command<'a>(
        &'a mut self,
        command: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>>;

    /// Check if session is connected
    fn is_connected(&self) -> bool;

    /// Close the session
    fn close<'a>(&'a mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;
}
