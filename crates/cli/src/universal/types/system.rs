//! System information types

use serde::{Deserialize, Serialize};

/// System information for benchmarking context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub storage_type: String,
    pub gpu_info: Option<String>,
}

/// Detailed hardware information
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_gb: f64,
    pub storage_type: String,
    pub gpu_info: Option<GpuInfo>,
}

/// GPU information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: String,
    pub memory_mb: u32,
    pub compute_capability: String,
}
