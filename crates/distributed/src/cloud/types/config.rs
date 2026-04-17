// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::common::load_balancing::{
    LoadBalancingAlgorithm as CommonLoadBalancingAlgorithm,
    LoadBalancingStrategy as CommonLoadBalancingStrategy,
};

use super::capabilities::ComplianceCertification;

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

/// VPN configuration for cross-cloud networking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConfig {
    /// VPN type (wireguard, ipsec, etc.).
    pub vpn_type: String,
    /// VPN endpoint.
    pub endpoint: String,
    /// Shared key for auth.
    pub shared_key: String,
}

/// DNS configuration for service discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS provider.
    pub dns_provider: String,
    /// Zone ID.
    pub zone_id: String,
    /// TTL in seconds.
    pub ttl_seconds: u32,
}

/// Cloud orchestrator configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudOrchestratorConfig {
    /// Scheduling strategy.
    pub scheduling_strategy: crate::cloud::HybridSchedulingStrategy,
    /// Cost config.
    pub cost_config: CostConfig,
    /// Compliance config.
    pub compliance_config: ComplianceConfig,
    /// Load balancer config.
    pub load_balancer_config: LoadBalancerConfig,
    /// Federation config.
    pub federation_config: FederationConfig,
    /// Explicit federation control-plane URL (HTTPS or as deployed).
    ///
    /// When `None`, resolution uses `TOADSTOOL_FEDERATION_ENDPOINT`, then a synthesized
    /// `https://federation.{domain}:{port}` default (see cloud orchestrator) with a warning.
    #[serde(default)]
    pub federation_endpoint: Option<String>,
}

/// Cost configuration for cloud orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    /// Budget limit (optional).
    pub budget_limit: Option<f64>,
    /// Enable cost tracking.
    pub cost_tracking_enabled: bool,
    /// Spot instance preference (0.0–1.0).
    pub spot_instance_preference: f64,
}

/// Compliance configuration for cloud placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    /// Required certifications.
    pub required_certifications: Vec<ComplianceCertification>,
    /// Allowed regions.
    pub allowed_regions: Vec<String>,
    /// Data sovereignty requirements.
    pub data_sovereignty_requirements: Vec<DataSovereigntyRequirement>,
}

/// Data sovereignty requirement for compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSovereigntyRequirement {
    /// Data type.
    pub data_type: String,
    /// Allowed regions.
    pub allowed_regions: Vec<String>,
    /// Encryption required.
    pub encryption_required: bool,
}

/// Load balancer configuration (using common types).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancing algorithm.
    pub algorithm: LoadBalancingAlgorithm,
    /// Health check interval.
    pub health_check_interval: Duration,
    /// Failover timeout.
    pub failover_timeout: Duration,
}

/// Load balancing algorithm (re-exported from common for backward compatibility).
pub type LoadBalancingAlgorithm = CommonLoadBalancingAlgorithm;

/// Federation configuration for multi-cloud coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Federation ID.
    pub federation_id: String,
    /// Discovery endpoint URLs.
    pub discovery_endpoints: Vec<String>,
    /// Trust anchor certificates.
    pub trust_anchors: Vec<String>,
}

/// Failover configuration for high availability.
#[derive(Debug, Clone)]
pub struct FailoverConfig {
    /// Enable automatic failover.
    pub automatic_failover: bool,
    /// Failover threshold duration.
    pub failover_threshold: Duration,
    /// Backup provider names.
    pub backup_providers: Vec<String>,
}

/// Network configuration for cross-cloud communication.
#[derive(Debug, Clone, Default)]
pub struct NetworkConfig {
    /// Enable encryption.
    pub encryption: bool,
    /// Enable compression.
    pub compression: bool,
    /// Connection timeout.
    pub timeout: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_disaster_recovery_config_default() {
        let config = DisasterRecoveryConfig::default();
        assert!(config.auto_failover);
        assert_eq!(config.rto_seconds, 900);
        assert_eq!(config.rpo_seconds, 300);
        assert_eq!(config.backup_retention_days, 30);
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert!(!config.encryption);
        assert!(!config.compression);
    }

    #[test]
    fn test_cost_config_serialization_roundtrip() {
        let config = CostConfig {
            budget_limit: Some(1000.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.5,
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: CostConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.budget_limit, Some(1000.0));
        assert!(parsed.cost_tracking_enabled);
    }

    #[test]
    fn test_federation_config_serialization_roundtrip() {
        let config = FederationConfig {
            federation_id: "fed-1".to_string(),
            discovery_endpoints: vec!["https://ep1.example.com".to_string()],
            trust_anchors: vec!["anchor1".to_string()],
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: FederationConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.federation_id, "fed-1");
        assert_eq!(parsed.discovery_endpoints.len(), 1);
    }

    #[test]
    fn test_load_balancer_config_serialization_roundtrip() {
        let config = LoadBalancerConfig {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            health_check_interval: Duration::from_secs(30),
            failover_timeout: Duration::from_secs(60),
        };
        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: LoadBalancerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.health_check_interval, Duration::from_secs(30));
    }

    #[test]
    fn test_data_sovereignty_requirement_serialization() {
        let req = DataSovereigntyRequirement {
            data_type: "pii".to_string(),
            allowed_regions: vec!["eu-west-1".to_string()],
            encryption_required: true,
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(json.contains("pii"));
        assert!(json.contains("eu-west-1"));
    }
}
