// SPDX-License-Identifier: AGPL-3.0-only
//! Supporting types for cloud integration
//!
//! This module contains all the shared types, enums, and configuration structs
//! used across the cloud integration layer.

mod capabilities;
mod compliance;
mod config;
mod cost;
mod deployment;

/// Capability, region, pricing, and resource-spec types for cloud providers.
pub use capabilities::{
    AvailabilityInfo, CloudCapabilities, CloudProviderMetadata, ComplianceCertification,
    ComputeType, MultiCloudAvailability, NetworkingFeature, PricingInfo, Region, ResourceSpec,
    SecurityFeature, StorageType,
};
/// Health checking, compliance requirements, trust levels, and job constraints.
pub use compliance::{
    CloudHealthChecker, ComplianceConstraints, ComplianceRequirements, TrustConfig, TrustLevel,
};
/// Orchestrator, networking, load balancing, and multi-cloud configuration.
pub use config::{
    CloudLoadBalancingStrategy, CloudOrchestratorConfig, ComplianceConfig, CostConfig,
    CrossCloudNetworking, DataSovereigntyRequirement, DisasterRecoveryConfig, DnsConfig,
    FailoverConfig, FederationConfig, LoadBalancerConfig, LoadBalancingAlgorithm, MultiCloudConfig,
    NetworkConfig, VpnConfig,
};
/// Cost models, spend tracking, budgets, alerts, and spot preferences.
pub use cost::{
    AlertSeverity, BudgetManager, CostAlert, CostModel, PerformanceMetric, SpendTracker,
    SpotInstanceManager,
};
/// Deployment topology, replication, federation, and job handle types.
pub use deployment::{
    BurstDistribution, CloudDeploymentResult, CloudJobHandle, CloudJobStatus, ConnectionStatus,
    ConsistencyLevel, DataReplica, DeploymentStrategy, DistributionStrategy, FederatedDeployment,
    FederationNode, MultiCloudDistribution, NetworkConnection, NodeConnection, ReplicaStatus,
    ReplicationConfig, ScaleConfig, TopologyType,
};
