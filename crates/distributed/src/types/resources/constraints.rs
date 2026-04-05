// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};

/// Resource constraints for job placement and scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    /// Maximum CPU cores allowed for placement.
    pub max_cpu_cores: Option<f64>,
    /// Maximum memory in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Required hardware/software features (e.g. gpu, nvme).
    pub required_features: Vec<String>,
    /// Node IDs to exclude from placement.
    pub excluded_nodes: Vec<String>,
}
