// SPDX-License-Identifier: AGPL-3.0-or-later
#![warn(missing_docs)]

//! # `ToadStool` Distributed Computing Integration
//!
//! Simplified distributed computing integration focused on:
//! - Coordination ecosystem integration for network effects
//! - Standalone execution capabilities
//! - Local resource management
//! - Service registration and health reporting

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::default_trait_access,
    clippy::cast_lossless,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::manual_is_variant_and,
    clippy::manual_let_else,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::return_self_not_must_use,
    clippy::single_match_else,
    clippy::str_split_at_newline,
    clippy::struct_field_names,
    clippy::unreadable_literal,
    clippy::unnecessary_wraps,
    clippy::unused_async,
    clippy::unused_self,
    reason = "distributed crate: numeric casts for network sizes; pedantic lints suppressed crate-wide"
)]
#![cfg_attr(
    test,
    allow(
        deprecated,
        reason = "tests exercise deprecated items during migration"
    )
)]
#![forbid(unsafe_code)]

// Always available: pure types and shared abstractions
/// Error types for the distributed layer.
pub mod error;
/// Shared types for jobs, resources, and execution.
pub mod types;
/// Common distributed abstractions (shared across Coordination, Cloud, etc.)
pub mod common;

// Universal substrate types (pure serde definitions, no async runtime)
/// Substrate type definitions for biological, quantum, neuromorphic, and edge platforms.
pub mod universal;

// Runtime-gated modules: networking, coordination, security, hosting, execution
#[cfg(feature = "runtime")]
/// Compatibility layer for legacy distributed APIs.
pub mod compatibility;
#[cfg(feature = "runtime")]
/// Core coordinator, config, and execution environment types.
pub mod core;
#[cfg(feature = "runtime")]
/// Ecosystem auth, registry, and service discovery.
pub mod ecosystem;
#[cfg(feature = "runtime")]
/// Recursive hosting and resource management.
pub mod hosting;
#[cfg(feature = "runtime")]
/// Metrics collection for distributed workloads.
pub mod metrics;
#[cfg(feature = "runtime")]
/// Network load balancing, distribution, and fault tolerance.
pub mod network;
#[cfg(feature = "runtime")]
/// OS-layer abstraction for cross-platform execution.
pub mod os_layer;
#[cfg(feature = "runtime")]
/// Cloud integration - universal cloud orchestration.
pub mod cloud;
#[cfg(feature = "runtime")]
/// Coordination integration - vendor-agnostic service coordination.
pub mod coordination_integration;
#[cfg(all(feature = "runtime", feature = "legacy-coordination"))]
#[deprecated(
    since = "2.0.0",
    note = "Use coordination_integration for vendor-agnostic coordination services"
)]
/// Coordination integration - universal signal coordination (DEPRECATED: use coordination_integration).
pub mod coordination;
#[cfg(feature = "runtime")]
/// Crypto lock system - primal-agnostic cryptographic access control.
pub mod crypto_lock;
#[cfg(feature = "runtime")]
/// Security provider abstraction - ANY primal can implement.
pub mod security_provider;
#[cfg(feature = "runtime")]
/// Crypto integration - vendor-agnostic cryptographic services.
pub mod crypto_integration;
#[cfg(feature = "runtime")]
#[deprecated(
    since = "2.0.0",
    note = "Use crypto_integration for vendor-agnostic crypto services"
)]
/// Security integration - capability-based encryption services (DEPRECATED: use crypto_integration).
pub mod security;
#[cfg(feature = "runtime")]
/// Substrate detection for universal compute platforms.
pub mod substrate_detection;
#[cfg(feature = "runtime")]
/// Primal capability system - agnostic integration with any primal.
pub mod primal_capabilities;

// Re-exports: always available
pub use types::{
    BackoffStrategy, CompatibilityMode, CpuRequirements, DistributedRetryConfig, ExecutionTarget,
    GpuRequirements, InstanceStatus, JobPriority, LoadBalancingStrategy, MemoryRequirements,
    NetworkRequirements, ProcessHandle, ResourceAllocation, ResourceConstraints, ResourceLimits,
    ResourceRequirements, RetryCondition, StorageRequirements, ToadStoolHostingConfig,
    UniversalExecutionResult, UniversalJob, UniversalJobQueue, UniversalJobType,
};

/// Substrate detection types for biological, quantum, neuromorphic, and edge platforms.
pub mod substrate {
    pub use crate::universal::substrate::{
        BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
        LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
        SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
    };
}

pub use universal::substrate::{
    BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
    LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
    SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
};

// Re-exports: runtime-gated
#[cfg(feature = "runtime")]
pub use compatibility::{
    CompatibilityLayer, LegacyCompatConfig, LegacyCompatibilityLayer, LinuxCompatConfig,
    LinuxCompatibilityLayer, MacOSCompatConfig, MacOSCompatibilityLayer, WindowsCompatConfig,
    WindowsCompatibilityLayer,
};
#[cfg(feature = "runtime")]
pub use core::{
    CoordinationConfig, DistributedConfig, DistributedCoordinator, ExecutionEnvironment,
    PlatformCapabilities, StandaloneConfig, StandaloneExecutor, ToadStoolCapabilities,
};
#[cfg(feature = "runtime")]
pub use ecosystem::{
    AuthToken, AuthenticationManager, Credentials, RegisteredService, ServiceRegistry,
};
#[cfg(feature = "runtime")]
pub use metrics::{
    EcosystemMetrics, LocalMetrics, NetworkMetrics, RecursiveHostingMetrics,
    UniversalMetricsCollector,
};
#[cfg(feature = "runtime")]
pub use network::{
    CircuitBreaker, CircuitBreakerState, FaultToleranceManager, NetworkDistributor,
    NetworkDistributorConfig, NetworkLoadBalancer, NetworkMetricsCollector, NetworkMetricsData,
    NodeHealth, RetryManager,
};
#[cfg(feature = "runtime")]
pub use hosting::{
    ChildResourceAllocator, ChildToadStoolInstance, CommunicationChannel, HostingResourceConfig,
    HostingResourceManager, InterInstanceCommunication, RecursiveHostingManager,
};
#[cfg(feature = "runtime")]
pub use os_layer::{OSLayerConfig, OSLayerManager};
#[cfg(feature = "runtime")]
pub use universal::{
    AdapterConfig, RecursiveHostingConfig, UniversalAdapter, UniversalScheduler,
    UniversalSchedulerConfig,
};
#[cfg(feature = "runtime")]
pub use cloud::{AWSCredentials, AzureCredentials, CloudProvider, GCPCredentials};
#[cfg(feature = "runtime")]
pub use crypto_lock::{
    AccessPolicies, CryptoValidator, DelegationValidator, PermissionHolder,
    PermissionRevocationList, SecurityPermissionValidator, SecurityProviderPermission,
    ToadStoolCryptoLock,
};
#[cfg(feature = "runtime")]
pub use primal_capabilities::{
    Capability, CapabilityProvider, CapabilityRegistry, CoordinationAdapter, PrimalAdapter,
    WorkloadExecutor, WorkloadRequest, WorkloadResponse,
};
#[cfg(feature = "runtime")]
pub use security_provider::ExternalTarget;
#[cfg(feature = "runtime")]
pub use substrate_detection::{PlatformType, SubstrateCapabilities, SubstrateDetector};

// Tests module
#[cfg(all(test, feature = "runtime"))]
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
