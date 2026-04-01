// SPDX-License-Identifier: AGPL-3.0-only
//! # Universal Cloud Integration
//!
//! ToadStool's cloud integration layer - use any cloud, anywhere, while maintaining
//! self-owned computing principles. We can use anybody's cloud, and they can use
//! ours (with bearDog permissions).

// Module declarations
mod compliance;
mod core;
mod cost;
mod credentials;
mod federation;
mod load_balancing;
mod orchestrator;
mod scheduling;
/// Shared cloud configuration, capabilities, deployment, and cost types.
pub mod types;

// Public re-exports (main API)
/// Compliance checks, reports, security tier, and the compliance enforcer.
pub use compliance::{
    CheckResult, CloudComplianceEnforcer, ComplianceCheck, ComplianceError, ComplianceReport,
    SecurityTier,
};
/// Cloud provider abstraction and universal orchestrator entry points.
pub use core::{CloudProvider, CloudProviderInterface, UniversalCloudOrchestrator};
/// Cost models, optimizer, estimates, and pricing tiers.
pub use cost::{
    CloudCostModel, CloudCostOptimizer, CostError, CostEstimate, CostLineItem, PricingTier,
};
/// Cloud credentials and mesh/Kubernetes auth configuration.
pub use credentials::{
    AWSCredentials, AuthMethod, AzureCredentials, EdgeMeshConfig, EncryptionLevel, GCPCredentials,
    KubernetesConfig,
};
/// Multi-cloud federation membership and errors.
pub use federation::{CloudFederationManager, FederationError, FederationMember};
/// Load balancer across multiple cloud backends.
pub use load_balancing::MultiCloudLoadBalancer;
/// Hybrid scheduler and scheduling strategy enum.
pub use scheduling::{HybridCloudScheduler, HybridSchedulingStrategy};
/// Opaque handle for a cloud-executed job.
pub use types::CloudJobHandle;

// Re-export commonly used types
/// Frequently used orchestrator, capability, deployment, and cost surface types.
pub use types::{
    AlertSeverity, AvailabilityInfo, BurstDistribution, CloudCapabilities, CloudDeploymentResult,
    CloudJobStatus, CloudLoadBalancingStrategy, CloudOrchestratorConfig, CloudProviderMetadata,
    ComplianceCertification, ComplianceConfig, ComplianceConstraints, ComplianceRequirements,
    ComputeType, ConnectionStatus, ConsistencyLevel, CostAlert, CostConfig, CostModel,
    DataSovereigntyRequirement, DeploymentStrategy, DisasterRecoveryConfig, DistributionStrategy,
    FederatedDeployment, FederationConfig, LoadBalancerConfig, LoadBalancingAlgorithm,
    MultiCloudAvailability, MultiCloudConfig, MultiCloudDistribution, NetworkingFeature,
    PerformanceMetric, PricingInfo, Region, ReplicaStatus, ResourceSpec, ScaleConfig,
    SecurityFeature, StorageType, TopologyType, TrustLevel,
};
