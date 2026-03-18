// SPDX-License-Identifier: AGPL-3.0-or-later
//! System information types

use serde::{Deserialize, Serialize};

/// System information for benchmarking context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// OS name (e.g. linux, macos)
    pub os: String,
    /// CPU architecture (e.g. x86_64, aarch64)
    pub arch: String,
    /// CPU model string
    pub cpu_model: String,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Total memory in GB
    pub memory_gb: f64,
    /// Storage type (ssd, hdd, nvme)
    pub storage_type: String,
    /// GPU info string if present
    pub gpu_info: Option<String>,
}

/// Detailed hardware information
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    /// CPU model string
    pub cpu_model: String,
    /// Number of CPU cores
    pub cpu_cores: u32,
    /// Total memory in GB
    pub memory_gb: f64,
    /// Storage type
    pub storage_type: String,
    /// GPU info if present
    pub gpu_info: Option<GpuInfo>,
}

/// GPU information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// GPU vendor (nvidia, amd, intel)
    pub vendor: String,
    /// GPU model name
    pub model: String,
    /// VRAM in MB
    pub memory_mb: u32,
    /// Compute capability (e.g. 8.0 for Ampere)
    pub compute_capability: String,
}
