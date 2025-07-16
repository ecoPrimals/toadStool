//! # `ToadStool` Distributed Computing Integration
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
pub use metrics::*;
pub use network::*;

// Specific exports to avoid ambiguous glob re-exports
pub use hosting::{
    ChildResourceAllocator, ChildToadStoolInstance, CommunicationChannel, HostingResourceConfig,
    HostingResourceManager, InterInstanceCommunication, RecursiveHostingManager,
};
pub use os_layer::{OSLayerConfig, OSLayerManager};
pub use types::{
    CompatibilityMode, CpuRequirements, ExecutionTarget, GpuRequirements, InstanceStatus,
    JobPriority, LoadBalancingStrategy, MemoryRequirements, NetworkRequirements, ProcessHandle,
    ResourceAllocation, ResourceConstraints, ResourceLimits, ResourceRequirements, RetryConfig,
    StorageRequirements, ToadStoolHostingConfig, UniversalJob, UniversalJobQueue, UniversalJobType,
};
pub use universal::{
    RecursiveHostingConfig, UniversalAdapter, UniversalScheduler, UniversalSchedulerConfig,
};

// Re-export existing modules with specific types
pub use cloud::{AWSCredentials, AzureCredentials, CloudProvider, GCPCredentials};
pub use crypto_lock::{
    AccessPolicies, BearDogCryptoPermission, BearDogPermissionValidator, CryptoValidator,
    DelegationValidator, ExternalTarget, PermissionHolder, PermissionRevocationList,
    ToadStoolCryptoLock,
};
pub use songbird_integration::{
    JobCoordinator, NetworkRequirements as SongbirdNetworkRequirements, SongbirdIntegrationConfig,
    SongbirdJobRequest, SongbirdJobResponse, SongbirdNetworkDiscovery,
};
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
