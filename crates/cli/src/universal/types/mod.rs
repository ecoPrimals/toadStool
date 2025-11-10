//! Type definitions for universal compute operations

mod benchmark;
mod federation;
mod migration;
mod platform;
mod system;

// Re-export all types
pub use benchmark::{BenchmarkResult, BenchmarkTest, BenchmarkType};
pub use federation::{FederationPeer, FederationStatus, TrustLevel};
pub use migration::{
    MigrationPlan, MigrationType, ReplicationHandle, WorkloadCheckpoint, WorkloadExport,
    WorkloadSnapshot,
};
pub use platform::{DetectedPlatform, PlatformStatus};
pub use system::{GpuInfo, HardwareInfo, SystemInfo};
