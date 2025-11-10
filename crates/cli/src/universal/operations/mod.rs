//! Universal Compute Manager - Operation Traits
//!
//! This module contains extension traits that organize the manager's operations
//! into logical groups. This pattern keeps files manageable while maintaining
//! clean separation of concerns.
//!
//! ## Operation Categories
//!
//! - **Detection**: Platform detection and capability testing
//! - **Benchmarking**: Performance benchmarking operations
//! - **Migration**: Workload migration between platforms
//! - **Federation**: Federation with other ToadStool instances
//! - **Capabilities**: Capability display and reporting
//! - **Utilities**: Helper utilities and system info

// Re-export all operation traits
pub mod detection;
pub mod benchmarking;
pub mod migration;
pub mod federation;
pub mod capabilities;
pub mod utilities;

pub use detection::PlatformDetectionOps;
pub use benchmarking::BenchmarkingOps;
pub use migration::MigrationOps;
pub use federation::FederationOps;
pub use capabilities::CapabilityDisplayOps;
pub use utilities::UtilityOps;

