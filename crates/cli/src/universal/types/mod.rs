//! Type definitions for universal compute operations
//!
//! **Zero-Copy Optimization** (Phase 2): Federation types now use `Arc<str>`.

mod benchmark;
pub mod federation; // Made public for operations module access
mod migration;
mod platform;
mod system;

// Re-export all types
pub use benchmark::{BenchmarkResult, BenchmarkTest, BenchmarkType};
pub use federation::{
    FederationPeer, FederationRequest, FederationResponse, FederationStatus, TrustLevel,
};
pub use migration::{
    MigrationPlan, MigrationType, ReplicationHandle, WorkloadCheckpoint, WorkloadExport,
    WorkloadSnapshot,
};
pub use platform::{DetectedPlatform, PlatformStatus};
pub use system::{GpuInfo, HardwareInfo, SystemInfo};
