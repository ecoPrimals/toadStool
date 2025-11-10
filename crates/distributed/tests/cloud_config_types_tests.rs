//! Comprehensive tests for cloud configuration types

use std::time::Duration;
use toadstool_distributed::cloud::types::{
    CloudOrchestratorConfig, ComplianceConfig, CostConfig, CrossCloudNetworking,
    DataSovereigntyRequirement, DisasterRecoveryConfig, DnsConfig, FederationConfig,
    LoadBalancerConfig, VpnConfig,
};
use toadstool_distributed::cloud::{ComplianceCertification, HybridSchedulingStrategy};
use toadstool_distributed::common::load_balancing::LoadBalancingAlgorithm;

// ============================================================================
// DisasterRecoveryConfig Tests
// ============================================================================

#[test]
fn test_disaster_recovery_config_default() {
    let config = DisasterRecoveryConfig::default();

    assert!(config.auto_failover);
    assert_eq!(config.rto_seconds, 900);
    assert_eq!(config.rpo_seconds, 300);
    assert_eq!(config.backup_retention_days, 30);
}

#[test]
fn test_disaster_recovery_config_custom() {
    let config = DisasterRecoveryConfig {
        auto_failover: false,
        rto_seconds: 600,
        rpo_seconds: 120,
        backup_retention_days: 90,
    };

    assert!(!config.auto_failover);
    assert_eq!(config.rto_seconds, 600);
    assert_eq!(config.rpo_seconds, 120);
    assert_eq!(config.backup_retention_days, 90);
}

#[test]
fn test_disaster_recovery_config_serialization() {
    let config = DisasterRecoveryConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: DisasterRecoveryConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.rto_seconds, 900);
}

#[test]
fn test_disaster_recovery_config_rto_rpo_relationship() {
    let config = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 300,
        rpo_seconds: 60,
        backup_retention_days: 30,
    };

    // RTO should typically be >= RPO
    assert!(config.rto_seconds >= config.rpo_seconds);
}

// ============================================================================
// CrossCloudNetworking Tests
// ============================================================================

#[test]
fn test_cross_cloud_networking_with_vpn() {
    let vpn = VpnConfig {
        vpn_type: "ipsec".to_string(),
        endpoint: "vpn.example.com".to_string(),
        shared_key: "secret".to_string(),
    };

    let dns = DnsConfig {
        dns_provider: "route53".to_string(),
        zone_id: "Z123456".to_string(),
        ttl_seconds: 300,
    };

    let networking = CrossCloudNetworking {
        vpn_config: Some(vpn),
        dns_config: dns,
        encryption_required: true,
    };

    assert!(networking.vpn_config.is_some());
    assert!(networking.encryption_required);
}

#[test]
fn test_cross_cloud_networking_without_vpn() {
    let dns = DnsConfig {
        dns_provider: "cloudflare".to_string(),
        zone_id: "abc123".to_string(),
        ttl_seconds: 60,
    };

    let networking = CrossCloudNetworking {
        vpn_config: None,
        dns_config: dns,
        encryption_required: false,
    };

    assert!(networking.vpn_config.is_none());
    assert!(!networking.encryption_required);
}

#[test]
fn test_cross_cloud_networking_serialization() {
    let dns = DnsConfig {
        dns_provider: "route53".to_string(),
        zone_id: "Z123456".to_string(),
        ttl_seconds: 300,
    };

    let networking = CrossCloudNetworking {
        vpn_config: None,
        dns_config: dns,
        encryption_required: true,
    };

    let json = serde_json::to_string(&networking).unwrap();
    assert!(!json.is_empty());

    let deserialized: CrossCloudNetworking = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.dns_config.dns_provider, "route53");
}

// ============================================================================
// VpnConfig Tests
// ============================================================================

#[test]
fn test_vpn_config_creation() {
    let vpn = VpnConfig {
        vpn_type: "wireguard".to_string(),
        endpoint: "10.0.0.1".to_string(),
        shared_key: "key123".to_string(),
    };

    assert_eq!(vpn.vpn_type, "wireguard");
    assert_eq!(vpn.endpoint, "10.0.0.1");
}

#[test]
fn test_vpn_config_serialization() {
    let vpn = VpnConfig {
        vpn_type: "ipsec".to_string(),
        endpoint: "vpn.example.com".to_string(),
        shared_key: "secret".to_string(),
    };

    let json = serde_json::to_string(&vpn).unwrap();
    assert!(!json.is_empty());

    let deserialized: VpnConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.vpn_type, "ipsec");
}

// ============================================================================
// DnsConfig Tests
// ============================================================================

#[test]
fn test_dns_config_creation() {
    let dns = DnsConfig {
        dns_provider: "route53".to_string(),
        zone_id: "Z123456".to_string(),
        ttl_seconds: 300,
    };

    assert_eq!(dns.dns_provider, "route53");
    assert_eq!(dns.ttl_seconds, 300);
}

#[test]
fn test_dns_config_low_ttl() {
    let dns = DnsConfig {
        dns_provider: "cloudflare".to_string(),
        zone_id: "abc123".to_string(),
        ttl_seconds: 1,
    };

    assert_eq!(dns.ttl_seconds, 1);
}

#[test]
fn test_dns_config_serialization() {
    let dns = DnsConfig {
        dns_provider: "route53".to_string(),
        zone_id: "Z123456".to_string(),
        ttl_seconds: 300,
    };

    let json = serde_json::to_string(&dns).unwrap();
    assert!(!json.is_empty());

    let deserialized: DnsConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.zone_id, "Z123456");
}

// ============================================================================
// CostConfig Tests
// ============================================================================

#[test]
fn test_cost_config_with_budget() {
    let config = CostConfig {
        budget_limit: Some(1000.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.8,
    };

    assert_eq!(config.budget_limit, Some(1000.0));
    assert!(config.cost_tracking_enabled);
}

#[test]
fn test_cost_config_without_budget() {
    let config = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: true,
        spot_instance_preference: 0.5,
    };

    assert!(config.budget_limit.is_none());
}

#[test]
fn test_cost_config_spot_preference_range() {
    let config_never = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };

    let config_always = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 1.0,
    };

    assert_eq!(config_never.spot_instance_preference, 0.0);
    assert_eq!(config_always.spot_instance_preference, 1.0);
}

#[test]
fn test_cost_config_serialization() {
    let config = CostConfig {
        budget_limit: Some(5000.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.7,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: CostConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.budget_limit, Some(5000.0));
}

// ============================================================================
// ComplianceConfig Tests
// ============================================================================

#[test]
fn test_compliance_config_empty() {
    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec![],
        data_sovereignty_requirements: vec![],
    };

    assert!(config.required_certifications.is_empty());
    assert!(config.allowed_regions.is_empty());
}

#[test]
fn test_compliance_config_with_certifications() {
    let config = ComplianceConfig {
        required_certifications: vec![
            ComplianceCertification::SOC2,
            ComplianceCertification::HIPAA,
        ],
        allowed_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        data_sovereignty_requirements: vec![],
    };

    assert_eq!(config.required_certifications.len(), 2);
    assert_eq!(config.allowed_regions.len(), 2);
}

#[test]
fn test_compliance_config_with_sovereignty() {
    let sovereignty_req = DataSovereigntyRequirement {
        data_type: "pii".to_string(),
        allowed_regions: vec!["eu-west-1".to_string()],
        encryption_required: true,
    };

    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec!["eu-west-1".to_string()],
        data_sovereignty_requirements: vec![sovereignty_req],
    };

    assert_eq!(config.data_sovereignty_requirements.len(), 1);
}

#[test]
fn test_compliance_config_serialization() {
    let config = ComplianceConfig {
        required_certifications: vec![ComplianceCertification::ISO27001],
        allowed_regions: vec!["us-west-2".to_string()],
        data_sovereignty_requirements: vec![],
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: ComplianceConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.allowed_regions[0], "us-west-2");
}

// ============================================================================
// DataSovereigntyRequirement Tests
// ============================================================================

#[test]
fn test_data_sovereignty_requirement_creation() {
    let req = DataSovereigntyRequirement {
        data_type: "health_records".to_string(),
        allowed_regions: vec!["us-east-1".to_string()],
        encryption_required: true,
    };

    assert_eq!(req.data_type, "health_records");
    assert!(req.encryption_required);
}

#[test]
fn test_data_sovereignty_requirement_multiple_regions() {
    let req = DataSovereigntyRequirement {
        data_type: "financial".to_string(),
        allowed_regions: vec![
            "us-east-1".to_string(),
            "us-west-2".to_string(),
            "eu-west-1".to_string(),
        ],
        encryption_required: true,
    };

    assert_eq!(req.allowed_regions.len(), 3);
}

#[test]
fn test_data_sovereignty_requirement_serialization() {
    let req = DataSovereigntyRequirement {
        data_type: "pii".to_string(),
        allowed_regions: vec!["eu-west-1".to_string()],
        encryption_required: true,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.is_empty());

    let deserialized: DataSovereigntyRequirement = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.data_type, "pii");
}

// ============================================================================
// LoadBalancerConfig Tests
// ============================================================================

#[test]
fn test_load_balancer_config_creation() {
    let config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::RoundRobin,
        health_check_interval: Duration::from_secs(10),
        failover_timeout: Duration::from_secs(30),
    };

    assert!(matches!(
        config.algorithm,
        LoadBalancingAlgorithm::RoundRobin
    ));
    assert_eq!(config.health_check_interval, Duration::from_secs(10));
}

#[test]
fn test_load_balancer_config_weighted() {
    let config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::WeightedRoundRobin,
        health_check_interval: Duration::from_secs(5),
        failover_timeout: Duration::from_secs(60),
    };

    assert!(matches!(
        config.algorithm,
        LoadBalancingAlgorithm::WeightedRoundRobin
    ));
}

#[test]
fn test_load_balancer_config_least_connections() {
    let config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::LeastConnections,
        health_check_interval: Duration::from_secs(15),
        failover_timeout: Duration::from_secs(45),
    };

    assert!(matches!(
        config.algorithm,
        LoadBalancingAlgorithm::LeastConnections
    ));
}

#[test]
fn test_load_balancer_config_serialization() {
    let config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::RoundRobin,
        health_check_interval: Duration::from_secs(10),
        failover_timeout: Duration::from_secs(30),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: LoadBalancerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.health_check_interval, Duration::from_secs(10));
}

// ============================================================================
// FederationConfig Tests
// ============================================================================

#[test]
fn test_federation_config_creation() {
    let config = FederationConfig {
        federation_id: "fed-001".to_string(),
        discovery_endpoints: vec![
            "http://discovery1.example.com".to_string(),
            "http://discovery2.example.com".to_string(),
        ],
        trust_anchors: vec!["beardog-anchor-1".to_string()],
    };

    assert_eq!(config.federation_id, "fed-001");
    assert_eq!(config.discovery_endpoints.len(), 2);
    assert_eq!(config.trust_anchors.len(), 1);
}

#[test]
fn test_federation_config_multiple_trust_anchors() {
    let config = FederationConfig {
        federation_id: "fed-002".to_string(),
        discovery_endpoints: vec!["http://discovery.example.com".to_string()],
        trust_anchors: vec![
            "anchor-1".to_string(),
            "anchor-2".to_string(),
            "anchor-3".to_string(),
        ],
    };

    assert_eq!(config.trust_anchors.len(), 3);
}

#[test]
fn test_federation_config_serialization() {
    let config = FederationConfig {
        federation_id: "fed-003".to_string(),
        discovery_endpoints: vec!["http://discovery.example.com".to_string()],
        trust_anchors: vec!["anchor-1".to_string()],
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: FederationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.federation_id, "fed-003");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_cloud_orchestrator_config() {
    let cost_config = CostConfig {
        budget_limit: Some(10000.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.6,
    };

    let compliance_config = ComplianceConfig {
        required_certifications: vec![ComplianceCertification::SOC2],
        allowed_regions: vec!["us-east-1".to_string()],
        data_sovereignty_requirements: vec![],
    };

    let load_balancer_config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::RoundRobin,
        health_check_interval: Duration::from_secs(10),
        failover_timeout: Duration::from_secs(30),
    };

    let federation_config = FederationConfig {
        federation_id: "fed-001".to_string(),
        discovery_endpoints: vec!["http://discovery.example.com".to_string()],
        trust_anchors: vec!["anchor-1".to_string()],
    };

    let orchestrator_config = CloudOrchestratorConfig {
        scheduling_strategy: HybridSchedulingStrategy::CostOptimized,
        cost_config,
        compliance_config,
        load_balancer_config,
        federation_config,
    };

    // Verify all components are properly configured
    assert!(orchestrator_config.cost_config.cost_tracking_enabled);
    assert_eq!(
        orchestrator_config.federation_config.federation_id,
        "fed-001"
    );
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn test_disaster_recovery_zero_rto() {
    let config = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 0,
        rpo_seconds: 0,
        backup_retention_days: 7,
    };

    assert_eq!(config.rto_seconds, 0);
    assert_eq!(config.rpo_seconds, 0);
}

#[test]
fn test_dns_config_very_high_ttl() {
    let dns = DnsConfig {
        dns_provider: "custom".to_string(),
        zone_id: "zone-999".to_string(),
        ttl_seconds: 86400, // 24 hours
    };

    assert_eq!(dns.ttl_seconds, 86400);
}

#[test]
fn test_cost_config_disabled_tracking() {
    let config = CostConfig {
        budget_limit: Some(5000.0),
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };

    assert!(!config.cost_tracking_enabled);
    assert_eq!(config.spot_instance_preference, 0.0);
}

#[test]
fn test_compliance_config_multiple_sovereignty_requirements() {
    let req1 = DataSovereigntyRequirement {
        data_type: "pii".to_string(),
        allowed_regions: vec!["eu-west-1".to_string()],
        encryption_required: true,
    };

    let req2 = DataSovereigntyRequirement {
        data_type: "financial".to_string(),
        allowed_regions: vec!["us-east-1".to_string()],
        encryption_required: true,
    };

    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        data_sovereignty_requirements: vec![req1, req2],
    };

    assert_eq!(config.data_sovereignty_requirements.len(), 2);
}

#[test]
fn test_vpn_config_different_types() {
    let ipsec = VpnConfig {
        vpn_type: "ipsec".to_string(),
        endpoint: "vpn1.example.com".to_string(),
        shared_key: "key1".to_string(),
    };

    let wireguard = VpnConfig {
        vpn_type: "wireguard".to_string(),
        endpoint: "vpn2.example.com".to_string(),
        shared_key: "key2".to_string(),
    };

    assert_ne!(ipsec.vpn_type, wireguard.vpn_type);
}

#[test]
fn test_load_balancer_config_resource_based() {
    let config = LoadBalancerConfig {
        algorithm: LoadBalancingAlgorithm::ResourceBased,
        health_check_interval: Duration::from_secs(20),
        failover_timeout: Duration::from_secs(40),
    };

    assert!(matches!(
        config.algorithm,
        LoadBalancingAlgorithm::ResourceBased
    ));
}

#[test]
fn test_federation_config_empty_endpoints() {
    let config = FederationConfig {
        federation_id: "fed-empty".to_string(),
        discovery_endpoints: vec![],
        trust_anchors: vec![],
    };

    assert!(config.discovery_endpoints.is_empty());
    assert!(config.trust_anchors.is_empty());
}

#[test]
fn test_cross_cloud_networking_encryption_required() {
    let dns = DnsConfig {
        dns_provider: "azure".to_string(),
        zone_id: "az-zone-1".to_string(),
        ttl_seconds: 120,
    };

    let networking = CrossCloudNetworking {
        vpn_config: None,
        dns_config: dns,
        encryption_required: true,
    };

    assert!(networking.encryption_required);
    assert!(networking.vpn_config.is_none());
}

#[test]
fn test_disaster_recovery_long_retention() {
    let config = DisasterRecoveryConfig {
        auto_failover: false,
        rto_seconds: 3600,
        rpo_seconds: 1800,
        backup_retention_days: 365,
    };

    assert_eq!(config.backup_retention_days, 365);
}

#[test]
fn test_data_sovereignty_no_encryption() {
    let req = DataSovereigntyRequirement {
        data_type: "public_data".to_string(),
        allowed_regions: vec!["global".to_string()],
        encryption_required: false,
    };

    assert!(!req.encryption_required);
}

#[test]
fn test_compliance_config_custom_certification() {
    let config = ComplianceConfig {
        required_certifications: vec![ComplianceCertification::Custom(
            "Custom-Cert-2025".to_string(),
        )],
        allowed_regions: vec!["custom-region".to_string()],
        data_sovereignty_requirements: vec![],
    };

    assert_eq!(config.required_certifications.len(), 1);
}
