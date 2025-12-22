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

// Zero-copy constants
pub mod constants;

// Re-export all operation traits
pub mod benchmarking;
pub mod capabilities;
pub mod detection;
pub mod federation;
pub mod migration;
pub mod utilities;

pub use benchmarking::BenchmarkingOps;
pub use capabilities::CapabilityDisplayOps;
pub use detection::PlatformDetectionOps;
pub use federation::FederationOps;
pub use migration::MigrationOps;
pub use utilities::UtilityOps;
