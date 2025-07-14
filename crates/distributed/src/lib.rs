//! # ToadStool Distributed Computing Integration
//!
//! Simplified distributed computing integration focused on:
//! - Songbird ecosystem integration for network effects
//! - Standalone execution capabilities
//! - Local resource management
//! - Service registration and health reporting

// Core modules
pub mod compatibility;
pub mod core;
pub mod ecosystem;
pub mod hosting;
pub mod metrics;
pub mod network;
pub mod os_layer;
pub mod types;
pub mod universal;

// Cloud integration - universal cloud orchestration
pub mod cloud;

// Songbird integration - universal signal coordination
pub mod songbird_integration;

// Crypto lock system - BearDog cryptographic access control
pub mod crypto_lock;

// Substrate detection for universal compute platforms
pub mod substrate_detection;

// Re-export main types and functionality
pub use compatibility::*;
pub use core::*;
pub use ecosystem::*;
pub use hosting::*;
pub use metrics::*;
pub use network::*;
pub use os_layer::*;
pub use types::*;
pub use universal::*;

// Re-export existing modules  
pub use cloud::*;
pub use crypto_lock::*;
pub use songbird_integration::*;
pub use substrate_detection::*;

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_coordinator_creation() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_universal_job_queue() {
        let queue = UniversalJobQueue::new();
        assert_eq!(queue.total_jobs(), 0);
    }

    #[tokio::test]
    async fn test_universal_scheduler_creation() {
        let config = UniversalSchedulerConfig::default();
        let scheduler = UniversalScheduler::new(config).await;
        assert!(scheduler.is_ok());
    }
}
