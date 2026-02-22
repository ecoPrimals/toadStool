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
pub mod types;

// Public re-exports (main API)
pub use compliance::{
    CheckResult, CloudComplianceEnforcer, ComplianceCheck, ComplianceError, ComplianceReport,
    SecurityTier,
};
pub use core::{CloudProvider, CloudProviderInterface, UniversalCloudOrchestrator};
pub use cost::{
    CloudCostModel, CloudCostOptimizer, CostError, CostEstimate, CostLineItem, PricingTier,
};
pub use credentials::{
    AWSCredentials, AuthMethod, AzureCredentials, EdgeMeshConfig, EncryptionLevel, GCPCredentials,
    KubernetesConfig,
};
pub use federation::{CloudFederationManager, FederationError, FederationMember};
pub use load_balancing::MultiCloudLoadBalancer;
pub use scheduling::{HybridCloudScheduler, HybridSchedulingStrategy};
pub use types::CloudJobHandle;

// Re-export commonly used types
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
