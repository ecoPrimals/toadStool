// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource type definitions for ToadStool

mod limits;
mod metrics;
mod requirements;
mod system;

pub use limits::{
    CpuLimits, MemoryLimits, NetworkLimits, ResourceLimits, ResourceUsage, StorageLimits,
};
pub use metrics::{
    CpuMetrics, GpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics,
    TimingMetrics,
};
pub use requirements::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceRequirements, StorageRequirements,
};
pub use system::{LoadAverages, NetworkStats, ProcessInfo, ProcessStatus, SystemResources};
