// SPDX-License-Identifier: AGPL-3.0-or-later

use super::helpers::{
    sample_coordination_config, sample_coordination_connection_config_with_endpoints,
};
use crate::coordination::types::*;
use std::time::Duration;

// Serde round-trips for config structs without tests elsewhere

#[test]
fn test_coordination_connection_config_serde() {
    let config =
        sample_coordination_connection_config_with_endpoints(vec!["http://a:8080".to_string()]);
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CoordinationConnectionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.endpoints.len(), 1);
}

#[test]
fn test_coordination_discovery_config_serde() {
    let config = CoordinationDiscoveryConfig {
        discovery_interval: Duration::from_secs(30),
        node_timeout: Duration::from_secs(10),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CoordinationDiscoveryConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.discovery_interval.as_secs(), 30);
}

#[test]
fn test_load_balancer_config_serde() {
    let config = LoadBalancerConfig {
        strategy: "least-loaded".to_string(),
        feedback_interval: Duration::from_secs(5),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: LoadBalancerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.strategy, "least-loaded");
}

#[test]
fn test_broadcast_config_serde() {
    let config = BroadcastConfig {
        channels: vec!["events".to_string()],
        message_retention: Duration::from_secs(60),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: BroadcastConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channels.len(), 1);
}

#[test]
fn test_capacity_config_serde() {
    let config = CapacityConfig {
        monitoring_interval: Duration::from_secs(20),
        resource_buffer: 0.2,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CapacityConfig = serde_json::from_str(&json).unwrap();
    assert!((parsed.resource_buffer - 0.2).abs() < 0.001);
}

#[test]
fn test_receiver_config_serde() {
    let config = ReceiverConfig {
        max_concurrent_jobs: 16,
        job_timeout: Duration::from_secs(600),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ReceiverConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_concurrent_jobs, 16);
}

#[test]
fn test_coordination_config_serde() {
    let config = sample_coordination_config();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CoordinationIntegrationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.receiver_config.max_concurrent_jobs, 4);
}
