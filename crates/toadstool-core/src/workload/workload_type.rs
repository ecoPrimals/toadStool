// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload type classification

use serde::{Deserialize, Serialize};

/// Types of workloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    /// Native executable
    Native,
    /// WebAssembly module
    Wasm,
    /// Container
    Container,
    /// GPU program
    Gpu,
    /// Python script
    Python,
    /// AI/ML workload
    AiMl,
    /// CUDA workload
    Cuda,
}
