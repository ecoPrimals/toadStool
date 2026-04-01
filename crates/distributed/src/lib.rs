// SPDX-License-Identifier: AGPL-3.0-only
#![warn(missing_docs)]

//! # `ToadStool` Distributed Computing Integration
//!
//! Simplified distributed computing integration focused on:
//! - Songbird ecosystem integration for network effects
//! - Standalone execution capabilities
//! - Local resource management
//! - Service registration and health reporting

#![allow(
    clippy::assigning_clones,
    clippy::default_trait_access,
    clippy::match_wildcard_for_single_variants,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::cast_lossless,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::manual_is_variant_and,
    clippy::manual_let_else,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use,
    clippy::single_match_else,
    clippy::str_split_at_newline,
    clippy::struct_field_names,
    clippy::unreadable_literal,
    clippy::unnecessary_wraps,
    clippy::unused_async,
    clippy::unused_self,
    clippy::zero_sized_map_values
)]
#![cfg_attr(test, allow(deprecated))] // Allow deprecated items in tests during transition
#![forbid(unsafe_code)]

// Core modules
/// Compatibility layer for legacy distributed APIs.
pub mod compatibility;
/// Core coordinator, config, and execution environment types.
pub mod core;
/// Ecosystem auth, registry, and service discovery.
pub mod ecosystem;
/// Error types for the distributed layer.
pub mod error;
/// Recursive hosting and resource management.
pub mod hosting;
/// Metrics collection for distributed workloads.
pub mod metrics;
/// Network load balancing, distribution, and fault tolerance.
pub mod network;
/// OS-layer abstraction for cross-platform execution.
pub mod os_layer;
/// Shared types for jobs, resources, and execution.
pub mod types;
/// Universal adapter and substrate detection.
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
pub use core::{
    DistributedConfig, DistributedCoordinator, ExecutionEnvironment, PlatformCapabilities,
    SongbirdConfig, StandaloneConfig, StandaloneExecutor, ToadStoolCapabilities,
};
pub use ecosystem::{
    AuthToken, AuthenticationManager, Credentials, RegisteredService, ServiceRegistry,
};
pub use metrics::{
    EcosystemMetrics, LocalMetrics, NetworkMetrics, RecursiveHostingMetrics,
    UniversalMetricsCollector,
};
pub use network::{
    CircuitBreaker, CircuitBreakerState, FaultToleranceManager, NetworkDistributor,
    NetworkDistributorConfig, NetworkLoadBalancer, NetworkMetricsCollector, NetworkMetricsData,
    NodeHealth, RetryManager,
};

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

/// Substrate detection types for biological, quantum, neuromorphic, and edge platforms.
pub mod substrate {
    pub use crate::universal::substrate::{
        BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
        LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
        SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
    };
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

// Re-export deprecated Songbird types for backward compatibility
#[allow(deprecated)]
pub use songbird_integration::{
    JobCoordinator, NetworkRequirements as SongbirdNetworkRequirements, SongbirdIntegrationConfig,
    SongbirdJobRequest, SongbirdJobResponse, SongbirdNetworkDiscovery,
};
pub use substrate_detection::{PlatformType, SubstrateCapabilities, SubstrateDetector};

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
