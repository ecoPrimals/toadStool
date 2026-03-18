// SPDX-License-Identifier: AGPL-3.0-or-later
//! Supporting types for cloud integration
//!
//! This module contains all the shared types, enums, and configuration structs
//! used across the cloud integration layer.

mod capabilities;
mod compliance;
mod config;
mod cost;
mod deployment;

pub use capabilities::{
    AvailabilityInfo, CloudCapabilities, CloudProviderMetadata, ComplianceCertification,
    ComputeType, MultiCloudAvailability, NetworkingFeature, PricingInfo, Region, ResourceSpec,
    SecurityFeature, StorageType,
};
pub use compliance::{
    CloudHealthChecker, ComplianceConstraints, ComplianceRequirements, TrustConfig, TrustLevel,
};
pub use config::{
    CloudLoadBalancingStrategy, CloudOrchestratorConfig, ComplianceConfig, CostConfig,
    CrossCloudNetworking, DataSovereigntyRequirement, DisasterRecoveryConfig, DnsConfig,
    FailoverConfig, FederationConfig, LoadBalancerConfig, LoadBalancingAlgorithm, MultiCloudConfig,
    NetworkConfig, VpnConfig,
};
pub use cost::{
    AlertSeverity, BudgetManager, CostAlert, CostModel, PerformanceMetric, SpendTracker,
    SpotInstanceManager,
};
pub use deployment::{
    BurstDistribution, CloudDeploymentResult, CloudJobHandle, CloudJobStatus, ConnectionStatus,
    ConsistencyLevel, DataReplica, DeploymentStrategy, DistributionStrategy, FederatedDeployment,
    FederationNode, MultiCloudDistribution, NetworkConnection, NodeConnection, ReplicaStatus,
    ReplicationConfig, ScaleConfig, TopologyType,
};
