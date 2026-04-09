// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared types and errors for cloud provider integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloud provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCapabilities {
    /// Provider name
    pub name: String,

    /// Available regions
    pub available_regions: Vec<String>,

    /// Supports GPU instances
    pub supports_gpu: bool,

    /// Available GPU types
    pub gpu_types: Vec<String>,

    /// Maximum memory per instance (GB)
    pub max_memory_gb: f64,

    /// Maximum CPU cores per instance
    pub max_cpu_cores: usize,

    /// Supports spot/preemptible instances
    pub supports_spot_instances: bool,

    /// Supports auto-scaling
    pub supports_autoscaling: bool,

    /// Custom capabilities (provider-specific)
    pub custom: HashMap<String, String>,
}

/// Workload location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadLocation {
    /// Local (bare metal, VM, container)
    Local {
        /// Hostname or identifier.
        hostname: String,
    },

    /// Cloud provider
    Cloud {
        /// Cloud provider name (e.g. aws, gcp).
        provider: String,
        /// Region identifier.
        region: String,
        /// Instance ID in the cloud provider.
        instance_id: String,
    },
}

/// Workload health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkloadHealth {
    /// Healthy and running
    Healthy,

    /// Degraded performance
    Degraded {
        /// Human-readable reason for degraded state.
        reason: String,
    },

    /// Unhealthy/failing
    Unhealthy {
        /// Human-readable reason for unhealthy state.
        reason: String,
    },

    /// Unknown status
    Unknown,
}

/// Workload specification for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSpec {
    /// Workload ID
    pub id: String,

    /// Required memory (GB)
    pub memory_gb: f64,

    /// Required CPU cores
    pub cpu_cores: usize,

    /// GPU required
    pub requires_gpu: bool,

    /// Preferred GPU type
    pub preferred_gpu_type: Option<String>,

    /// Estimated runtime (hours)
    pub estimated_runtime_hours: Option<f64>,

    /// Custom requirements
    pub custom: HashMap<String, String>,
}

/// Cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Estimated cost per hour (USD)
    pub cost_per_hour: f64,

    /// Estimated total cost (USD)
    pub estimated_total_cost: Option<f64>,

    /// Breakdown by resource
    pub breakdown: HashMap<String, f64>,
}

/// GPU type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuType {
    /// GPU model name
    pub name: String,

    /// Memory (GB)
    pub memory_gb: f64,

    /// Compute capability
    pub compute_capability: Option<String>,

    /// Cost per hour (USD)
    pub cost_per_hour: f64,

    /// Available in regions
    pub available_regions: Vec<String>,
}

/// Cloud provider error
#[derive(Debug, Clone, thiserror::Error)]
pub enum CloudError {
    /// Provider not available
    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    /// Region not supported
    #[error("Region not supported: {0}")]
    RegionUnsupported(String),

    /// Insufficient quota/capacity
    #[error("Insufficient capacity: {0}")]
    InsufficientCapacity(String),

    /// Deployment failed
    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),

    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}
