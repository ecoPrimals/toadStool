//! Supporting types for cloud integration
//!
//! This module contains all the shared types, enums, and configuration structs
//! used across the cloud integration layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use toadstool_common::constants::timeouts;
use uuid::Uuid;

use crate::common::distribution::DistributionStrategy as CommonDistributionStrategy;
use crate::common::load_balancing::{
    LoadBalancingAlgorithm as CommonLoadBalancingAlgorithm,
    LoadBalancingStrategy as CommonLoadBalancingStrategy,
};

// ============================================================================
// Configuration Structures
// ============================================================================

/// Multi-cloud configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCloudConfig {
    /// Primary cloud provider
    pub primary_provider: crate::cloud::CloudProvider,
    /// Secondary cloud providers for failover
    pub secondary_providers: Vec<crate::cloud::CloudProvider>,
    /// Load balancing strategy across clouds
    pub load_balancing: CloudLoadBalancingStrategy,
    /// Disaster recovery configuration
    pub disaster_recovery: DisasterRecoveryConfig,
    /// Cross-cloud networking configuration
    pub networking: CrossCloudNetworking,
}

/// Cloud load balancing strategies (re-exported from common for backward compatibility)
pub type CloudLoadBalancingStrategy = CommonLoadBalancingStrategy;

/// Disaster recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisasterRecoveryConfig {
    /// Enable automatic failover
    pub auto_failover: bool,
    /// RTO (Recovery Time Objective) in seconds
    pub rto_seconds: u64,
    /// RPO (Recovery Point Objective) in seconds  
    pub rpo_seconds: u64,
    /// Backup retention policy
    pub backup_retention_days: u32,
}

impl Default for DisasterRecoveryConfig {
    fn default() -> Self {
        Self {
            auto_failover: true,
            rto_seconds: 900,
            rpo_seconds: 300,
            backup_retention_days: 30,
        }
    }
}

/// Cross-cloud networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCloudNetworking {
    /// VPN configuration for secure cross-cloud communication
    pub vpn_config: Option<VpnConfig>,
    /// DNS configuration for service discovery
    pub dns_config: DnsConfig,
    /// Traffic encryption requirements
    pub encryption_required: bool,
}

/// VPN configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConfig {
    pub vpn_type: String,
    pub endpoint: String,
    pub shared_key: String,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub dns_provider: String,
    pub zone_id: String,
    pub ttl_seconds: u32,
}

/// Cloud orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudOrchestratorConfig {
    pub scheduling_strategy: crate::cloud::HybridSchedulingStrategy,
    pub cost_config: CostConfig,
    pub compliance_config: ComplianceConfig,
    pub load_balancer_config: LoadBalancerConfig,
    pub federation_config: FederationConfig,
}

/// Cost configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    pub budget_limit: Option<f64>,
    pub cost_tracking_enabled: bool,
    pub spot_instance_preference: f64, // 0.0 = never, 1.0 = always
}

/// Compliance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub required_certifications: Vec<ComplianceCertification>,
    pub allowed_regions: Vec<String>,
    pub data_sovereignty_requirements: Vec<DataSovereigntyRequirement>,
}

/// Data sovereignty requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSovereigntyRequirement {
    pub data_type: String,
    pub allowed_regions: Vec<String>,
    pub encryption_required: bool,
}

/// Load balancer configuration (using common types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub algorithm: LoadBalancingAlgorithm,
    pub health_check_interval: Duration,
    pub failover_timeout: Duration,
}

/// Load balancing algorithm (re-exported from common for backward compatibility)
pub type LoadBalancingAlgorithm = CommonLoadBalancingAlgorithm;

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub federation_id: String,
    pub discovery_endpoints: Vec<String>,
    pub trust_anchors: Vec<String>, // bearDog trust anchors
}

/// Failover configuration
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    pub automatic_failover: bool,
    pub failover_threshold: Duration,
    pub backup_providers: Vec<String>,
}

// ============================================================================
// Capability & Metadata Types
// ============================================================================

/// Cloud capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCapabilities {
    pub compute_types: Vec<ComputeType>,
    pub storage_types: Vec<StorageType>,
    pub networking_features: Vec<NetworkingFeature>,
    pub security_features: Vec<SecurityFeature>,
    pub compliance_certifications: Vec<ComplianceCertification>,
    pub regions: Vec<Region>,
    pub max_cpu_cores: Option<u32>,
    pub max_memory_gb: Option<u32>,
    pub gpu_support: bool,
    pub kubernetes_support: bool,
    pub serverless_support: bool,
}

/// Cloud provider metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderMetadata {
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub supported_protocols: Vec<String>,
    pub documentation_url: String,
    pub support_contact: String,
}

/// Resource specifications
#[derive(Debug, Clone)]
pub struct ResourceSpec {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: Option<u32>,
    pub network_bandwidth_mbps: Option<u64>,
}

/// Pricing information
#[derive(Debug, Clone)]
pub struct PricingInfo {
    pub cpu_cost_per_hour: f64,
    pub memory_cost_per_gb_hour: f64,
    pub storage_cost_per_gb_month: f64,
    pub network_cost_per_gb: f64,
    pub total_estimated_cost: f64,
}

/// Availability information
#[derive(Debug, Clone)]
pub struct AvailabilityInfo {
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
    pub gpu_count: u32,
    pub regions: Vec<String>,
    pub availability_zones: Vec<String>,
}

/// Multi-cloud availability tracking
#[derive(Debug, Clone)]
pub struct MultiCloudAvailability {
    providers: HashMap<String, AvailabilityInfo>,
    unavailable_providers: Vec<String>,
}

impl Default for MultiCloudAvailability {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiCloudAvailability {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            unavailable_providers: Vec::new(),
        }
    }

    pub fn add_provider(&mut self, name: impl Into<String>, availability: AvailabilityInfo) {
        self.providers.insert(name.into(), availability);
    }

    pub fn mark_provider_unavailable(&mut self, name: impl Into<String>) {
        self.unavailable_providers.push(name.into());
    }
}

/// Region information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub location: String,
    pub availability_zones: Vec<String>,
}

// ============================================================================
// Enum Types
// ============================================================================

/// Compute type options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeType {
    VM,
    Container,
    Serverless,
    BareMetalC,
    GPU,
    FPGA,
}

/// Storage type options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    BlockStorage,
    ObjectStorage,
    FileStorage,
    DatabaseStorage,
}

/// Networking feature options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkingFeature {
    VPC,
    LoadBalancer,
    CDN,
    PrivateNetworking,
    VPN,
}

/// Security feature options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SecurityFeature {
    Encryption,
    IdentityManagement,
    NetworkSecurity,
    Compliance,
}

/// Compliance certifications
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceCertification {
    SOC2,
    ISO27001,
    HIPAA,
    PciDss,
    GDPR,
    FedRAMP,
    Custom(String),
}

// ============================================================================
// Job Types
// ============================================================================

/// Handle for a cloud job
#[derive(Debug, Clone)]
pub struct CloudJobHandle {
    pub job_id: Uuid,
    pub provider_job_id: String,
    pub provider_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Cloud job status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudJobStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String },
    Cancelled,
}

/// Scale configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleConfig {
    pub target_replicas: Option<u32>,
    pub cpu_scale_factor: Option<f64>,
    pub memory_scale_factor: Option<f64>,
}

// ============================================================================
// Deployment Strategy Types
// ============================================================================

/// Deployment strategy options
#[derive(Debug, Clone)]
pub enum DeploymentStrategy {
    SingleCloud {
        provider_name: String,
    },
    MultiCloud {
        providers: Vec<String>,
        distribution: MultiCloudDistribution,
    },
    HybridCloudBurst {
        primary: String,
        burst_providers: Vec<String>,
    },
    FederatedDeployment {
        federation_nodes: Vec<String>,
    },
}

/// Cloud deployment result
#[derive(Debug, Clone)]
pub enum CloudDeploymentResult {
    Single {
        provider: String,
        handle: CloudJobHandle,
    },
    Multi {
        handles: HashMap<String, CloudJobHandle>,
    },
    Federated {
        deployment: FederatedDeployment,
    },
}

/// Multi-cloud distribution configuration
#[derive(Debug, Clone)]
pub struct MultiCloudDistribution {
    pub providers: Vec<String>,
    pub strategy: DistributionStrategy,
}

/// Cloud distribution strategy (re-exported from common for backward compatibility)
pub type DistributionStrategy = CommonDistributionStrategy;

/// Burst distribution configuration
#[derive(Debug, Clone)]
pub struct BurstDistribution {
    pub providers: Vec<String>,
    pub primary_provider: String,
}

/// Federated deployment configuration
#[derive(Debug, Clone)]
pub struct FederatedDeployment {
    pub federation_id: Uuid,
    pub nodes: Vec<String>,
    pub coordination_endpoint: String,
}

// ============================================================================
// Federation Types
// ============================================================================

/// Topology type for federation
#[derive(Debug, Clone, Default)]
pub enum TopologyType {
    #[default]
    Centralized,
    Distributed,
    Mesh,
    Hierarchical,
}

/// Federation node information
#[derive(Debug, Clone, Default)]
pub struct FederationNode {
    pub id: String,
    pub provider: String,
    pub region: String,
    pub capabilities: Vec<String>,
}

/// Connection between federation nodes
#[derive(Debug, Clone, Default)]
pub struct NodeConnection {
    pub from: String,
    pub to: String,
    pub latency: f64,
    pub bandwidth: f64,
}

/// Network connection status
#[derive(Debug, Clone, Default)]
pub struct NetworkConnection {
    pub id: String,
    pub provider: String,
    pub status: ConnectionStatus,
}

/// Connection status enum
#[derive(Debug, Clone, Default)]
pub enum ConnectionStatus {
    #[default]
    Active,
    Inactive,
    Error,
}

// ============================================================================
// Replication Types
// ============================================================================

/// Data replica information
#[derive(Debug, Clone, Default)]
pub struct DataReplica {
    pub id: String,
    pub location: String,
    pub status: ReplicaStatus,
}

/// Replica status enum
#[derive(Debug, Clone, Default)]
pub enum ReplicaStatus {
    #[default]
    Synced,
    Syncing,
    OutOfSync,
}

/// Replication configuration
#[derive(Debug, Clone, Default)]
pub struct ReplicationConfig {
    pub factor: u32,
    pub consistency: ConsistencyLevel,
}

/// Consistency level for replication
#[derive(Debug, Clone, Default)]
pub enum ConsistencyLevel {
    #[default]
    Strong,
    Eventual,
    Weak,
}

// ============================================================================
// Trust & Security Types
// ============================================================================

/// Trust level for cloud providers
#[derive(Debug, Clone, Default)]
pub enum TrustLevel {
    #[default]
    Trusted,
    Untrusted,
    Conditional,
}

/// Network configuration
#[derive(Debug, Clone, Default)]
pub struct NetworkConfig {
    pub encryption: bool,
    pub compression: bool,
    pub timeout: Duration,
}

/// Trust configuration
#[derive(Debug, Clone, Default)]
pub struct TrustConfig {
    pub validation_required: bool,
    pub trust_threshold: f64,
}

// ============================================================================
// Monitoring & Alerting Types
// ============================================================================

/// Performance metric
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: std::time::SystemTime,
}

impl Default for PerformanceMetric {
    fn default() -> Self {
        Self {
            name: String::new(),
            value: 0.0,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Cost alert
#[derive(Debug, Clone, Default)]
pub struct CostAlert {
    pub threshold: f64,
    pub message: String,
    pub severity: AlertSeverity,
}

/// Alert severity levels
#[derive(Debug, Clone, Default)]
pub enum AlertSeverity {
    #[default]
    Info,
    Warning,
    Critical,
}

// ============================================================================
// Cost Management Types
// ============================================================================

/// Cost model for a provider
#[derive(Debug, Clone)]
pub struct CostModel {
    pub cpu_cost_per_core_hour: f64,
    pub memory_cost_per_gb_hour: f64,
    pub storage_cost_per_gb_month: f64,
    pub network_cost_per_gb: f64,
}

/// Spend tracker
#[derive(Debug, Clone)]
pub struct SpendTracker {
    pub current_spend: f64,
    pub monthly_spend: f64,
    pub projected_spend: f64,
}

/// Budget manager
#[derive(Debug, Clone)]
pub struct BudgetManager {
    pub monthly_budget: Option<f64>,
    pub alert_thresholds: Vec<f64>,
}

/// Spot instance manager
#[derive(Debug, Clone)]
pub struct SpotInstanceManager {
    pub spot_preference: f64,
    pub max_interruption_tolerance: Duration,
}

// ============================================================================
// Health Checking Types
// ============================================================================

/// Cloud health checker
#[derive(Debug, Clone)]
pub struct CloudHealthChecker {
    pub endpoint: String,
    pub check_interval: Duration,
    pub timeout: Duration,
}

impl CloudHealthChecker {
    pub fn new(provider: String) -> Self {
        Self {
            endpoint: format!("https://{}.amazonaws.com", provider),
            check_interval: timeouts::HEALTH_CHECK_INTERVAL,
            timeout: timeouts::TCP_CONNECT_TIMEOUT,
        }
    }
}

// ============================================================================
// Compliance Types
// ============================================================================

/// Compliance requirements
#[derive(Debug, Clone)]
pub struct ComplianceRequirements {
    pub certifications: Vec<ComplianceCertification>,
    pub regions: Vec<String>,
    pub data_sovereignty: Vec<DataSovereigntyRequirement>,
}

/// Compliance constraints for a job
#[derive(Debug, Clone)]
pub struct ComplianceConstraints {
    pub allowed_providers: Vec<String>,
    pub required_regions: Vec<String>,
    pub encryption_required: bool,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disaster_recovery_config_default() {
        let config = DisasterRecoveryConfig::default();
        assert!(config.auto_failover);
        assert_eq!(config.rto_seconds, 900);
        assert_eq!(config.rpo_seconds, 300);
        assert_eq!(config.backup_retention_days, 30);
    }

    #[test]
    fn test_multi_cloud_availability_new() {
        let avail = MultiCloudAvailability::new();
        assert!(avail.providers.is_empty());
    }

    #[test]
    fn test_multi_cloud_availability_add_provider() {
        let mut avail = MultiCloudAvailability::new();
        avail.add_provider(
            "aws",
            AvailabilityInfo {
                cpu_cores: 64.0,
                memory_gb: 256.0,
                storage_gb: 1000.0,
                gpu_count: 4,
                regions: vec!["us-east-1".to_string()],
                availability_zones: vec!["us-east-1a".to_string()],
            },
        );
        // Verify add_provider completes without panic
        let _ = &avail;
    }

    #[test]
    fn test_multi_cloud_availability_mark_unavailable() {
        let mut avail = MultiCloudAvailability::new();
        avail.mark_provider_unavailable("gcp");
        // Verify mark_provider_unavailable completes without panic
        let _ = &avail;
    }

    #[test]
    fn test_cloud_health_checker_new() {
        let checker = CloudHealthChecker::new("ec2".to_string());
        assert!(checker.endpoint.contains("ec2"));
        assert!(checker.endpoint.contains("amazonaws.com"));
    }

    #[test]
    fn test_region_construction() {
        let region = Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
        };
        assert_eq!(region.name, "us-east-1");
        assert_eq!(region.availability_zones.len(), 2);
    }

    #[test]
    fn test_topology_type_default() {
        let topo = TopologyType::default();
        assert!(matches!(topo, TopologyType::Centralized));
    }

    #[test]
    fn test_federation_node_default() {
        let node = FederationNode::default();
        assert!(node.id.is_empty());
        assert!(node.capabilities.is_empty());
    }

    #[test]
    fn test_replica_status_default() {
        let status = ReplicaStatus::default();
        assert!(matches!(status, ReplicaStatus::Synced));
    }

    #[test]
    fn test_connection_status_default() {
        let status = ConnectionStatus::default();
        assert!(matches!(status, ConnectionStatus::Active));
    }

    #[test]
    fn test_alert_severity_default() {
        let severity = AlertSeverity::default();
        assert!(matches!(severity, AlertSeverity::Info));
    }

    #[test]
    fn test_performance_metric_default() {
        let metric = PerformanceMetric::default();
        assert!(metric.name.is_empty());
        assert_eq!(metric.value, 0.0);
    }
}
