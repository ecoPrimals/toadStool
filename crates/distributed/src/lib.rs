//! # `ToadStool` Distributed Computing Integration
//!
//! Simplified distributed computing integration focused on:
//! - Songbird ecosystem integration for network effects
//! - Standalone execution capabilities
//! - Local resource management
//! - Service registration and health reporting

#![cfg_attr(test, allow(deprecated))] // Allow deprecated items in tests during transition
#![allow(clippy::unused_async)] // Many placeholder/stub async fns for future impl; per-function allow in hot paths
#![deny(unsafe_code)]

// Core modules
pub mod compatibility;
pub mod core;
pub mod ecosystem;
pub mod error;
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

// Coordination integration - vendor-agnostic service coordination (NEW)
pub mod coordination_integration;

// Songbird integration - universal signal coordination (DEPRECATED: use coordination_integration)
#[deprecated(
    since = "2.0.0",
    note = "Use coordination_integration for vendor-agnostic coordination services"
)]
pub mod songbird_integration;

// Crypto lock system - primal-agnostic cryptographic access control
pub mod crypto_lock;

// Security provider abstraction - ANY primal can implement (NEW: Phase 1B)
pub mod security_provider;

// Crypto integration - vendor-agnostic cryptographic services (NEW)
pub mod crypto_integration;

// BearDog integration - capability-based encryption services (DEPRECATED: use crypto_integration)
#[deprecated(
    since = "2.0.0",
    note = "Use crypto_integration for vendor-agnostic crypto services"
)]
pub mod beardog_integration;

// Substrate detection for universal compute platforms
pub mod substrate_detection;

// Primal capability system - agnostic integration with any primal (Songbird, Squirrel, BearDog, etc.)
pub mod primal_capabilities;

// Re-export main types and functionality
// Note: compatibility now re-exports from canonical core implementation
pub use compatibility::{
    CompatibilityLayer, LegacyCompatConfig, LegacyCompatibilityLayer, LinuxCompatConfig,
    LinuxCompatibilityLayer, MacOSCompatConfig, MacOSCompatibilityLayer, WindowsCompatConfig,
    WindowsCompatibilityLayer,
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
    BackoffStrategy, CompatibilityMode, CpuRequirements, DistributedRetryConfig, ExecutionTarget,
    GpuRequirements, InstanceStatus, JobPriority, LoadBalancingStrategy, MemoryRequirements,
    NetworkRequirements, ProcessHandle, ResourceAllocation, ResourceConstraints, ResourceLimits,
    ResourceRequirements, RetryCondition, StorageRequirements, ToadStoolHostingConfig,
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
    AccessPolicies, CryptoValidator, DelegationValidator, PermissionHolder,
    PermissionRevocationList, SecurityPermissionValidator, SecurityProviderPermission,
    ToadStoolCryptoLock,
};
pub use primal_capabilities::{
    Capability, CapabilityProvider, CapabilityRegistry, PrimalAdapter, SongbirdAdapter,
    WorkloadExecutor, WorkloadRequest, WorkloadResponse,
};
pub use security_provider::ExternalTarget;

// Re-export deprecated songbird types for backward compatibility
#[allow(deprecated)]
pub use songbird_integration::{
    JobCoordinator, NetworkRequirements as SongbirdNetworkRequirements, SongbirdIntegrationConfig,
    SongbirdJobRequest, SongbirdJobResponse, SongbirdNetworkDiscovery,
};
pub use substrate_detection::*;

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_distributed_coordinator_creation() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_universal_job_queue() {
        let queue = UniversalJobQueue::new();
        assert_eq!(queue.total_jobs(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_universal_scheduler_creation() {
        let config = UniversalSchedulerConfig::default();
        let scheduler = UniversalScheduler::new(config).await;
        assert!(scheduler.is_ok());
    }
}
