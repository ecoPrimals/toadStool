// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource management and monitoring for `ToadStool`

mod monitoring;
mod types;

#[cfg(test)]
mod tests;

// Re-export all public types for backward compatibility
#[cfg(any(test, feature = "test-mocks"))]
pub use monitoring::TestResourceMonitor;
pub use monitoring::{ResourceMonitor, ResourceMonitorDispatch, SystemResourceMonitor};
pub use types::{
    CpuLimits, CpuMetrics, CpuRequirements, GpuMetrics, GpuRequirements, LoadAverages,
    MemoryLimits, MemoryMetrics, MemoryRequirements, NetworkLimits, NetworkMetrics,
    NetworkRequirements, NetworkStats, ProcessInfo, ProcessStatus, ResourceLimits,
    ResourceRequirements, ResourceUsage, RuntimeMetrics, StorageLimits, StorageMetrics,
    StorageRequirements, SystemResources, TimingMetrics,
};
