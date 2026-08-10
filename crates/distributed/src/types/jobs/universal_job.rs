// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal job envelope and job-type classification for distributed scheduling.

use std::str::FromStr;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use toadstool_common::ToadStoolError;
use toadstool_core::execution::ExecutionRequest;

use super::priority::JobPriority;

use super::execution_target::ExecutionTarget;
use crate::types::resources::{DistributedRetryConfig, ResourceRequirements};

/// Universal job for cross-platform execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalJob {
    /// Job identification
    pub job_id: Uuid,
    /// Job type (optional for auto-detection)
    pub job_type: Option<UniversalJobType>,
    /// Execution request
    pub execution_request: ExecutionRequest,
    /// Target destination
    pub target: ExecutionTarget,
    /// Priority level
    pub priority: JobPriority,
    /// Dependencies
    pub dependencies: Vec<Uuid>,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
    /// Retry configuration
    pub retry_config: DistributedRetryConfig,
    /// Created timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
}

/// Universal job types for different execution scenarios
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UniversalJobType {
    /// Local execution
    Local,
    /// Remote `ToadStool` execution
    RemoteToadStool {
        /// Remote endpoint URL.
        endpoint: String,
    },
    /// Ecosystem tool execution
    EcosystemTool {
        /// Tool name.
        tool_name: String,
        /// Tool endpoint URL.
        endpoint: String,
    },
    /// Recursive `ToadStool` hosting
    RecursiveHosting {
        /// Hosting config for child instance.
        toadstool_config: super::hosting::ToadStoolHostingConfig,
    },
    /// OS-layer compatibility execution
    OSLayerCompatibility {
        /// Compatibility mode.
        compatibility_mode: super::hosting::CompatibilityMode,
    },

    // Job classification types for distributed scheduling
    /// CPU-intensive computational work
    ComputeIntensive,
    /// Memory-intensive workloads
    MemoryIntensive,
    /// Network-intensive workloads
    NetworkIntensive,
    /// Storage-intensive workloads
    StorageIntensive,
    /// Hybrid workloads combining multiple resource types
    Hybrid,
    /// Data processing and analytics
    DataProcessing,
    /// Machine learning and AI workloads
    MachineLearning,
    /// Scientific simulations
    Simulation,
    /// Native execution
    Native,
    /// Container-based execution
    Container,
    /// WebAssembly execution
    WASM,
    /// GPU-accelerated execution
    GPU,
    /// Custom workload type
    Custom(String),
}

impl FromStr for UniversalJobType {
    type Err = ToadStoolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "compute_intensive" => Ok(Self::ComputeIntensive),
            "memory_intensive" => Ok(Self::MemoryIntensive),
            "network_intensive" => Ok(Self::NetworkIntensive),
            "storage_intensive" => Ok(Self::StorageIntensive),
            "hybrid" => Ok(Self::Hybrid),
            "data_processing" => Ok(Self::DataProcessing),
            "machine_learning" => Ok(Self::MachineLearning),
            "simulation" => Ok(Self::Simulation),
            "native" => Ok(Self::Native),
            "container" => Ok(Self::Container),
            "wasm" => Ok(Self::WASM),
            "gpu" => Ok(Self::GPU),
            _ => Ok(Self::Custom(s.to_string())),
        }
    }
}
