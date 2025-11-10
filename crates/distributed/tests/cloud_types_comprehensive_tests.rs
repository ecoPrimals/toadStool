//! Comprehensive tests for Cloud module types
//!
//! Coverage targets:
//! - MultiCloudConfig
//! - DisasterRecoveryConfig
//! - CrossCloudNetworking
//! - VpnConfig, DnsConfig
//! - CloudOrchestratorConfig
//! - CostConfig, ComplianceConfig
//! - Various cloud enums and types

use toadstool_distributed::cloud::types::*;

// ============================================================================
// DisasterRecoveryConfig Tests (8 tests)
// ============================================================================

#[test]
fn test_disaster_recovery_config_default() {
    let config = DisasterRecoveryConfig::default();
    
    assert!(config.auto_failover);
    assert_eq!(config.rto_seconds, 900); // 15 minutes
    assert_eq!(config.rpo_seconds, 300); // 5 minutes
    assert_eq!(config.backup_retention_days, 30);
}

#[test]
fn test_disaster_recovery_config_custom() {
    let config = DisasterRecoveryConfig {
        auto_failover: false,
        rto_seconds: 60,
        rpo_seconds: 30,
        backup_retention_days: 7,
    };
    
    assert!(!config.auto_failover);
    assert_eq!(config.rto_seconds, 60);
    assert_eq!(config.rpo_seconds, 30);
    assert_eq!(config.backup_retention_days, 7);
}

#[test]
fn test_disaster_recovery_config_high_availability() {
    let config = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 5, // 5 seconds RTO
        rpo_seconds: 1, // 1 second RPO
        backup_retention_days: 90,
    };
    
    assert!(config.auto_failover);
    assert!(config.rto_seconds < 10);
    assert!(config.rpo_seconds < 5);
    assert_eq!(config.backup_retention_days, 90);
}

#[test]
fn test_disaster_recovery_config_clone() {
    let config = DisasterRecoveryConfig::default();
    let cloned = config.clone();
    
    assert_eq!(config.auto_failover, cloned.auto_failover);
    assert_eq!(config.rto_seconds, cloned.rto_seconds);
    assert_eq!(config.rpo_seconds, cloned.rpo_seconds);
}

#[test]
fn test_disaster_recovery_config_debug() {
    let config = DisasterRecoveryConfig::default();
    let debug_str = format!("{:?}", config);
    
    assert!(debug_str.contains("DisasterRecoveryConfig"));
    assert!(debug_str.contains("auto_failover"));
}

#[test]
fn test_disaster_recovery_config_serialization() {
    let config = DisasterRecoveryConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    
    assert!(json.contains("auto_failover"));
    assert!(json.contains("rto_seconds"));
}

#[test]
fn test_disaster_recovery_config_deserialization() {
    let json = r#"{
        "auto_failover": true,
        "rto_seconds": 120,
        "rpo_seconds": 60,
        "backup_retention_days": 14
    }"#;
    
    let config: DisasterRecoveryConfig = serde_json::from_str(json)
        .expect("Failed to deserialize");
    
    assert!(config.auto_failover);
    assert_eq!(config.rto_seconds, 120);
    assert_eq!(config.rpo_seconds, 60);
    assert_eq!(config.backup_retention_days, 14);
}

#[test]
fn test_disaster_recovery_config_rto_rpo_relationship() {
    let config = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 300,
        rpo_seconds: 60,
        backup_retention_days: 30,
    };
    
    // RPO should typically be <= RTO
    assert!(config.rpo_seconds <= config.rto_seconds);
}

// ============================================================================
// VpnConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_vpn_config_creation() {
    let config = VpnConfig {
        vpn_type: "WireGuard".to_string(),
        endpoint: "vpn.example.com".to_string(),
        shared_key: "secret-key-123".to_string(),
    };
    
    assert_eq!(config.vpn_type, "WireGuard");
    assert!(!config.endpoint.is_empty());
    assert!(!config.shared_key.is_empty());
}

#[test]
fn test_vpn_config_clone() {
    let config = VpnConfig {
        vpn_type: "OpenVPN".to_string(),
        endpoint: "10.0.0.1".to_string(),
        shared_key: "shared-secret".to_string(),
    };
    
    let cloned = config.clone();
    assert_eq!(config.vpn_type, cloned.vpn_type);
    assert_eq!(config.endpoint, cloned.endpoint);
}

#[test]
fn test_vpn_config_debug() {
    let config = VpnConfig {
        vpn_type: "IPSec".to_string(),
        endpoint: "vpn-gateway.cloud".to_string(),
        shared_key: "psk".to_string(),
    };
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("VpnConfig"));
    assert!(debug_str.contains("IPSec"));
}

#[test]
fn test_vpn_config_serialization() {
    let config = VpnConfig {
        vpn_type: "WireGuard".to_string(),
        endpoint: "192.168.1.1".to_string(),
        shared_key: "key123".to_string(),
    };
    
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(json.contains("WireGuard"));
    assert!(json.contains("endpoint"));
}

#[test]
fn test_vpn_config_different_types() {
    let wireguard = VpnConfig {
        vpn_type: "WireGuard".to_string(),
        endpoint: "wg.example.com".to_string(),
        shared_key: "wg-key".to_string(),
    };
    
    let openvpn = VpnConfig {
        vpn_type: "OpenVPN".to_string(),
        endpoint: "ovpn.example.com".to_string(),
        shared_key: "ovpn-key".to_string(),
    };
    
    assert_ne!(wireguard.vpn_type, openvpn.vpn_type);
}

// ============================================================================
// DnsConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_dns_config_creation() {
    let config = DnsConfig {
        dns_provider: "Route53".to_string(),
        zone_id: "Z1234567890ABC".to_string(),
        ttl_seconds: 300,
    };
    
    assert_eq!(config.dns_provider, "Route53");
    assert!(!config.zone_id.is_empty());
    assert_eq!(config.ttl_seconds, 300);
}

#[test]
fn test_dns_config_clone() {
    let config = DnsConfig {
        dns_provider: "CloudFlare".to_string(),
        zone_id: "cf-zone-123".to_string(),
        ttl_seconds: 60,
    };
    
    let cloned = config.clone();
    assert_eq!(config.dns_provider, cloned.dns_provider);
    assert_eq!(config.ttl_seconds, cloned.ttl_seconds);
}

#[test]
fn test_dns_config_debug() {
    let config = DnsConfig {
        dns_provider: "Azure DNS".to_string(),
        zone_id: "azure-zone-456".to_string(),
        ttl_seconds: 120,
    };
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("DnsConfig"));
    assert!(debug_str.contains("Azure DNS"));
}

#[test]
fn test_dns_config_serialization() {
    let config = DnsConfig {
        dns_provider: "Google Cloud DNS".to_string(),
        zone_id: "gcp-zone-789".to_string(),
        ttl_seconds: 180,
    };
    
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(json.contains("Google Cloud DNS"));
    assert!(json.contains("ttl_seconds"));
}

#[test]
fn test_dns_config_ttl_values() {
    let short_ttl = DnsConfig {
        dns_provider: "Route53".to_string(),
        zone_id: "zone-1".to_string(),
        ttl_seconds: 60, // 1 minute
    };
    
    let long_ttl = DnsConfig {
        dns_provider: "Route53".to_string(),
        zone_id: "zone-2".to_string(),
        ttl_seconds: 86400, // 24 hours
    };
    
    assert!(short_ttl.ttl_seconds < long_ttl.ttl_seconds);
    assert_eq!(short_ttl.ttl_seconds, 60);
    assert_eq!(long_ttl.ttl_seconds, 86400);
}

// ============================================================================
// CrossCloudNetworking Tests (6 tests)
// ============================================================================

#[test]
fn test_cross_cloud_networking_with_vpn() {
    let config = CrossCloudNetworking {
        vpn_config: Some(VpnConfig {
            vpn_type: "WireGuard".to_string(),
            endpoint: "vpn.cloud".to_string(),
            shared_key: "secret".to_string(),
        }),
        dns_config: DnsConfig {
            dns_provider: "Route53".to_string(),
            zone_id: "zone-abc".to_string(),
            ttl_seconds: 300,
        },
        encryption_required: true,
    };
    
    assert!(config.vpn_config.is_some());
    assert!(config.encryption_required);
    assert_eq!(config.dns_config.dns_provider, "Route53");
}

#[test]
fn test_cross_cloud_networking_without_vpn() {
    let config = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "CloudFlare".to_string(),
            zone_id: "zone-def".to_string(),
            ttl_seconds: 120,
        },
        encryption_required: false,
    };
    
    assert!(config.vpn_config.is_none());
    assert!(!config.encryption_required);
}

#[test]
fn test_cross_cloud_networking_clone() {
    let config = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "Azure DNS".to_string(),
            zone_id: "zone-ghi".to_string(),
            ttl_seconds: 60,
        },
        encryption_required: true,
    };
    
    let cloned = config.clone();
    assert_eq!(config.encryption_required, cloned.encryption_required);
    assert_eq!(config.dns_config.dns_provider, cloned.dns_config.dns_provider);
}

#[test]
fn test_cross_cloud_networking_debug() {
    let config = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "Route53".to_string(),
            zone_id: "zone-xyz".to_string(),
            ttl_seconds: 300,
        },
        encryption_required: true,
    };
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("CrossCloudNetworking"));
    assert!(debug_str.contains("encryption_required"));
}

#[test]
fn test_cross_cloud_networking_serialization() {
    let config = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "Google Cloud DNS".to_string(),
            zone_id: "zone-123".to_string(),
            ttl_seconds: 180,
        },
        encryption_required: true,
    };
    
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(json.contains("encryption_required"));
    assert!(json.contains("dns_config"));
}

#[test]
fn test_cross_cloud_networking_encryption_modes() {
    let encrypted = CrossCloudNetworking {
        vpn_config: Some(VpnConfig {
            vpn_type: "WireGuard".to_string(),
            endpoint: "vpn.secure.cloud".to_string(),
            shared_key: "encrypted-key".to_string(),
        }),
        dns_config: DnsConfig {
            dns_provider: "Route53".to_string(),
            zone_id: "zone-secure".to_string(),
            ttl_seconds: 60,
        },
        encryption_required: true,
    };
    
    let unencrypted = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "Route53".to_string(),
            zone_id: "zone-public".to_string(),
            ttl_seconds: 300,
        },
        encryption_required: false,
    };
    
    assert!(encrypted.encryption_required);
    assert!(!unencrypted.encryption_required);
    assert!(encrypted.vpn_config.is_some());
    assert!(unencrypted.vpn_config.is_none());
}

// ============================================================================
// CostConfig Tests (6 tests)
// ============================================================================

#[test]
fn test_cost_config_with_budget() {
    let config = CostConfig {
        budget_limit: Some(1000.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.5,
    };
    
    assert!(config.budget_limit.is_some());
    assert_eq!(config.budget_limit.unwrap(), 1000.0);
    assert!(config.cost_tracking_enabled);
    assert_eq!(config.spot_instance_preference, 0.5);
}

#[test]
fn test_cost_config_without_budget() {
    let config = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: true,
        spot_instance_preference: 0.0,
    };
    
    assert!(config.budget_limit.is_none());
    assert!(config.cost_tracking_enabled);
}

#[test]
fn test_cost_config_spot_preference_range() {
    let never_spot = CostConfig {
        budget_limit: Some(500.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.0,
    };
    
    let always_spot = CostConfig {
        budget_limit: Some(500.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 1.0,
    };
    
    assert_eq!(never_spot.spot_instance_preference, 0.0);
    assert_eq!(always_spot.spot_instance_preference, 1.0);
}

#[test]
fn test_cost_config_clone() {
    let config = CostConfig {
        budget_limit: Some(750.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.75,
    };
    
    let cloned = config.clone();
    assert_eq!(config.budget_limit, cloned.budget_limit);
    assert_eq!(config.spot_instance_preference, cloned.spot_instance_preference);
}

#[test]
fn test_cost_config_debug() {
    let config = CostConfig {
        budget_limit: Some(2000.0),
        cost_tracking_enabled: false,
        spot_instance_preference: 0.25,
    };
    
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("CostConfig"));
    assert!(debug_str.contains("budget_limit"));
}

#[test]
fn test_cost_config_serialization() {
    let config = CostConfig {
        budget_limit: Some(1500.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.6,
    };
    
    let json = serde_json::to_string(&config).expect("Failed to serialize");
    assert!(json.contains("budget_limit"));
    assert!(json.contains("cost_tracking_enabled"));
}

// ============================================================================
// Integration Tests (10 tests)
// ============================================================================

#[test]
fn test_disaster_recovery_config_round_trip() {
    let original = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 600,
        rpo_seconds: 120,
        backup_retention_days: 60,
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: DisasterRecoveryConfig = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.auto_failover, deserialized.auto_failover);
    assert_eq!(original.rto_seconds, deserialized.rto_seconds);
    assert_eq!(original.rpo_seconds, deserialized.rpo_seconds);
    assert_eq!(original.backup_retention_days, deserialized.backup_retention_days);
}

#[test]
fn test_vpn_config_round_trip() {
    let original = VpnConfig {
        vpn_type: "WireGuard".to_string(),
        endpoint: "vpn.example.com".to_string(),
        shared_key: "test-key".to_string(),
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: VpnConfig = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.vpn_type, deserialized.vpn_type);
    assert_eq!(original.endpoint, deserialized.endpoint);
}

#[test]
fn test_dns_config_round_trip() {
    let original = DnsConfig {
        dns_provider: "Route53".to_string(),
        zone_id: "zone-test".to_string(),
        ttl_seconds: 240,
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: DnsConfig = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.dns_provider, deserialized.dns_provider);
    assert_eq!(original.zone_id, deserialized.zone_id);
    assert_eq!(original.ttl_seconds, deserialized.ttl_seconds);
}

#[test]
fn test_cross_cloud_networking_round_trip() {
    let original = CrossCloudNetworking {
        vpn_config: Some(VpnConfig {
            vpn_type: "OpenVPN".to_string(),
            endpoint: "vpn.cloud".to_string(),
            shared_key: "secret".to_string(),
        }),
        dns_config: DnsConfig {
            dns_provider: "CloudFlare".to_string(),
            zone_id: "zone-cf".to_string(),
            ttl_seconds: 90,
        },
        encryption_required: true,
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: CrossCloudNetworking = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert!(deserialized.vpn_config.is_some());
    assert_eq!(deserialized.encryption_required, original.encryption_required);
}

#[test]
fn test_cost_config_round_trip() {
    let original = CostConfig {
        budget_limit: Some(3000.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.8,
    };
    
    let json = serde_json::to_string(&original).expect("Serialization failed");
    let deserialized: CostConfig = serde_json::from_str(&json)
        .expect("Deserialization failed");
    
    assert_eq!(original.budget_limit, deserialized.budget_limit);
    assert_eq!(original.cost_tracking_enabled, deserialized.cost_tracking_enabled);
    assert_eq!(original.spot_instance_preference, deserialized.spot_instance_preference);
}

#[test]
fn test_disaster_recovery_realistic_values() {
    // Mission critical system
    let mission_critical = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 10,
        rpo_seconds: 5,
        backup_retention_days: 365,
    };
    
    // Standard production
    let standard = DisasterRecoveryConfig {
        auto_failover: true,
        rto_seconds: 300,
        rpo_seconds: 60,
        backup_retention_days: 90,
    };
    
    // Development environment
    let dev = DisasterRecoveryConfig {
        auto_failover: false,
        rto_seconds: 3600,
        rpo_seconds: 1800,
        backup_retention_days: 7,
    };
    
    assert!(mission_critical.rto_seconds < standard.rto_seconds);
    assert!(standard.rto_seconds < dev.rto_seconds);
    assert!(mission_critical.backup_retention_days > dev.backup_retention_days);
}

#[test]
fn test_cost_optimization_strategies() {
    // Cost-optimized
    let cost_optimized = CostConfig {
        budget_limit: Some(100.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 1.0, // Always use spot
    };
    
    // Balanced
    let balanced = CostConfig {
        budget_limit: Some(500.0),
        cost_tracking_enabled: true,
        spot_instance_preference: 0.5,
    };
    
    // Performance-optimized
    let performance = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0, // Never use spot
    };
    
    assert!(cost_optimized.spot_instance_preference > balanced.spot_instance_preference);
    assert!(balanced.spot_instance_preference > performance.spot_instance_preference);
    assert!(cost_optimized.budget_limit.unwrap() < balanced.budget_limit.unwrap());
}

#[test]
fn test_networking_configuration_combinations() {
    // Secure multi-cloud
    let secure = CrossCloudNetworking {
        vpn_config: Some(VpnConfig {
            vpn_type: "WireGuard".to_string(),
            endpoint: "secure-vpn.cloud".to_string(),
            shared_key: "strong-key".to_string(),
        }),
        dns_config: DnsConfig {
            dns_provider: "Route53".to_string(),
            zone_id: "secure-zone".to_string(),
            ttl_seconds: 60, // Short TTL for fast failover
        },
        encryption_required: true,
    };
    
    // Simple single-cloud
    let simple = CrossCloudNetworking {
        vpn_config: None,
        dns_config: DnsConfig {
            dns_provider: "CloudFlare".to_string(),
            zone_id: "simple-zone".to_string(),
            ttl_seconds: 300,
        },
        encryption_required: false,
    };
    
    assert!(secure.vpn_config.is_some());
    assert!(simple.vpn_config.is_none());
    assert!(secure.encryption_required);
    assert!(!simple.encryption_required);
}

#[test]
fn test_dns_provider_diversity() {
    let providers = vec![
        "Route53",
        "CloudFlare",
        "Azure DNS",
        "Google Cloud DNS",
        "DigitalOcean DNS",
    ];
    
    for provider in providers {
        let config = DnsConfig {
            dns_provider: provider.to_string(),
            zone_id: format!("{}-zone", provider),
            ttl_seconds: 300,
        };
        
        assert_eq!(config.dns_provider, provider);
        assert!(!config.zone_id.is_empty());
    }
}

#[test]
fn test_vpn_types_diversity() {
    let vpn_types = vec!["WireGuard", "OpenVPN", "IPSec", "L2TP", "PPTP"];
    
    for vpn_type in vpn_types {
        let config = VpnConfig {
            vpn_type: vpn_type.to_string(),
            endpoint: format!("{}.vpn.cloud", vpn_type.to_lowercase()),
            shared_key: format!("{}-key", vpn_type),
        };
        
        assert_eq!(config.vpn_type, vpn_type);
        assert!(!config.endpoint.is_empty());
    }
}
