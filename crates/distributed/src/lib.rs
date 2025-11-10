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

// Common distributed abstractions (shared across Songbird, Cloud, etc.)
pub mod common;

// Cloud integration - universal cloud orchestration
pub mod cloud;

// Songbird integration - universal signal coordination
pub mod songbird_integration;

// Crypto lock system - BearDog cryptographic access control
pub mod crypto_lock;

// Substrate detection for universal compute platforms
pub mod substrate_detection;

// Primal capability system - agnostic integration with any primal (Songbird, Squirrel, BearDog, etc.)
pub mod primal_capabilities;

// Re-export main types and functionality
// Note: compatibility now re-exports from canonical core implementation
pub use compatibility::{
    CompatibilityLayer,
    LinuxCompatibilityLayer, LinuxCompatConfig,
    WindowsCompatibilityLayer, WindowsCompatConfig,
    MacOSCompatibilityLayer, MacOSCompatConfig,
    LegacyCompatibilityLayer, LegacyCompatConfig,
};
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
    BackoffStrategy, CompatibilityMode, CpuRequirements, ExecutionTarget, GpuRequirements,
    InstanceStatus, JobPriority, LoadBalancingStrategy, MemoryRequirements, NetworkRequirements,
    ProcessHandle, ResourceAllocation, ResourceConstraints, ResourceLimits, ResourceRequirements,
    RetryCondition, DistributedRetryConfig, StorageRequirements, ToadStoolHostingConfig,
    UniversalExecutionResult, UniversalJob, UniversalJobQueue, UniversalJobType,
};
pub use universal::{
    AdapterConfig, RecursiveHostingConfig, UniversalAdapter, UniversalScheduler,
    UniversalSchedulerConfig,
};

// Export substrate module and types for examples
pub mod substrate {
    pub use crate::universal::substrate::*;
}

// Also re-export substrate types directly
pub use universal::substrate::{
    BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
    LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
    SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
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
pub use primal_capabilities::{
    Capability, CapabilityProvider, CapabilityRegistry,
    PrimalAdapter, SongbirdAdapter,
    WorkloadRequest, WorkloadResponse, WorkloadExecutor,
};

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
