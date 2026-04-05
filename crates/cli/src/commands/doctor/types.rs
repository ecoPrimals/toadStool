// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::Serialize;

/// Results of the doctor diagnostic checks
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    /// Hardware detection results (CPU, GPU, NPU, memory)
    pub hardware: HardwareReport,
    /// Ecosystem connectivity results (sockets, primals)
    pub ecosystem: EcosystemReport,
    /// Configuration validity results
    pub config: ConfigReport,
    /// Aggregated summary (passed, warnings, errors)
    pub summary: Summary,
}

/// Hardware detection results from doctor
#[derive(Debug, Serialize)]
pub struct HardwareReport {
    /// Number of CPU cores detected
    pub cpu_cores: usize,
    /// CPU feature flags (e.g. avx2, sse4)
    pub cpu_features: Vec<String>,
    /// Whether a GPU was detected
    pub gpu_detected: bool,
    /// GPU model/info string if detected
    pub gpu_info: Option<String>,
    /// Whether an NPU was detected
    pub npu_detected: bool,
    /// NPU model/info if detected
    pub npu_info: Option<String>,
    /// Total memory in MB
    pub memory_total_mb: u64,
    /// Detected issues or warnings
    pub issues: Vec<String>,
}

/// Ecosystem connectivity results from doctor
#[derive(Debug, Serialize)]
pub struct EcosystemReport {
    /// Whether biomeOS directory exists
    pub biomeos_dir_exists: bool,
    /// Path to biomeOS directory
    pub biomeos_dir: String,
    /// Primal socket paths found
    pub sockets_found: Vec<String>,
    /// Status of each primal (reachable or not)
    pub primals_reachable: Vec<PrimalStatus>,
    /// Detected issues
    pub issues: Vec<String>,
}

/// Status of a single primal (BearDog, NestGate, Songbird, etc.)
#[derive(Debug, Serialize)]
pub struct PrimalStatus {
    /// Primal name (e.g. beardog, nestgate)
    pub name: String,
    /// Whether the primal socket file exists
    pub socket_exists: bool,
    /// Whether the primal responds
    pub reachable: bool,
}

/// Configuration validity results from doctor
#[derive(Debug, Serialize)]
pub struct ConfigReport {
    /// Whether config file exists
    pub config_file_exists: bool,
    /// Path to config file if found
    pub config_file_path: Option<String>,
    /// Environment variables that are set
    pub env_vars_set: Vec<String>,
    /// Configuration issues found
    pub issues: Vec<String>,
}

/// Aggregated doctor check summary
#[derive(Debug, Serialize)]
pub struct Summary {
    /// Total number of checks run
    pub total_checks: usize,
    /// Checks that passed
    pub passed: usize,
    /// Checks with warnings
    pub warnings: usize,
    /// Checks with errors
    pub errors: usize,
    /// Overall status (ok, warning, error)
    pub overall_status: String,
}
