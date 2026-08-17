// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn test_config_builder() {
    let config = EcosystemConfig::builder()
        .auto_discovery(true)
        .discovery_timeout(std::time::Duration::from_mins(1))
        .build();

    assert!(config.auto_discovery);
    assert_eq!(config.discovery_timeout.as_secs(), 60);
}

#[test]
fn test_service_status() {
    let status = ServiceStatus::Connected;
    assert!(status.is_usable());
    assert!(!status.is_error());

    let failed = ServiceStatus::Failed("test error".to_string());
    assert!(!failed.is_usable());
    assert!(failed.is_error());
    assert_eq!(failed.error_message(), Some("test error"));
}

#[test]
fn test_message_creation() {
    let msg = EcosystemMessage::new(
        "service-1",
        "service-2",
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );

    assert_eq!(msg.from, "service-1");
    assert_eq!(msg.to, "service-2");
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
}

#[test]
fn test_message_type_properties() {
    assert!(EcosystemMessageType::ResourceRequest.requires_response());
    assert!(!EcosystemMessageType::Heartbeat.requires_response());

    assert!(EcosystemMessageType::ResourceResponse.is_response());
    assert!(!EcosystemMessageType::ResourceRequest.is_response());
}

#[test]
fn test_default_ecosystem_config() {
    let config = EcosystemConfig::default();
    assert!(config.auto_discovery);
    assert_eq!(config.discovery_timeout, std::time::Duration::from_secs(30));
    assert!(matches!(
        config.discovery_method,
        DiscoveryMethodConfig::Auto
    ));
    assert!(config.required_capabilities.is_empty());
    assert!(config.optional_capabilities.is_empty());
}

#[test]
fn test_config_builder_with_all_options() {
    use toadstool_common::primal_identity::{Capability, ComputeCapability};

    let config = EcosystemConfig::builder()
        .auto_discovery(false)
        .discovery_timeout(std::time::Duration::from_mins(2))
        .discovery_method(DiscoveryMethodConfig::Mdns)
        .require_capability(Capability::Compute(ComputeCapability::NativeExecution))
        .optional_capability(Capability::Compute(ComputeCapability::GpuCompute))
        .build();

    assert!(!config.auto_discovery);
    assert_eq!(config.discovery_timeout.as_secs(), 120);
    assert!(matches!(
        config.discovery_method,
        DiscoveryMethodConfig::Mdns
    ));
    assert_eq!(config.required_capabilities.len(), 1);
    assert_eq!(config.optional_capabilities.len(), 1);
}

#[test]
fn test_discovery_method_config_variants() {
    let _ = DiscoveryMethodConfig::Auto;
    let _ = DiscoveryMethodConfig::Environment;
    let _ = DiscoveryMethodConfig::Mdns;
    let _ = DiscoveryMethodConfig::ConfigFile {
        path: "/etc/config.yaml".to_string(),
    };
    let _ = DiscoveryMethodConfig::Registry {
        endpoint: "http://localhost:8080".to_string(),
    };
}

#[test]
fn test_service_status_discovered_not_usable() {
    let status = ServiceStatus::Discovered;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_service_status_connecting_not_usable() {
    let status = ServiceStatus::Connecting;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_service_status_disconnected_not_usable() {
    let status = ServiceStatus::Disconnected;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_service_status_removing_not_usable() {
    let status = ServiceStatus::Removing;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_service_status_non_failed_no_error_message() {
    assert_eq!(ServiceStatus::Discovered.error_message(), None);
    assert_eq!(ServiceStatus::Connecting.error_message(), None);
    assert_eq!(ServiceStatus::Connected.error_message(), None);
    assert_eq!(ServiceStatus::Disconnected.error_message(), None);
    assert_eq!(ServiceStatus::Removing.error_message(), None);
}

#[test]
fn test_heartbeat_message() {
    let msg = EcosystemMessage::heartbeat("sender", "receiver");
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
    assert_eq!(msg.payload, serde_json::json!({}));
    assert_eq!(msg.from, "sender");
    assert_eq!(msg.to, "receiver");
}

#[test]
fn test_error_message() {
    let msg = EcosystemMessage::error("sender", "receiver", "oops");
    assert_eq!(msg.message_type, EcosystemMessageType::Error);
    assert_eq!(msg.payload["error"], "oops");
    assert_eq!(msg.from, "sender");
    assert_eq!(msg.to, "receiver");
}

#[test]
fn test_all_message_types_requires_response() {
    assert!(
        EcosystemMessageType::ResourceRequest.requires_response(),
        "ResourceRequest should require response"
    );
    assert!(
        EcosystemMessageType::WorkloadRequest.requires_response(),
        "WorkloadRequest should require response"
    );
    assert!(
        !EcosystemMessageType::Heartbeat.requires_response(),
        "Heartbeat should not require response"
    );
    assert!(
        !EcosystemMessageType::CapabilityAnnouncement.requires_response(),
        "CapabilityAnnouncement should not require response"
    );
    assert!(
        !EcosystemMessageType::ResourceResponse.requires_response(),
        "ResourceResponse should not require response"
    );
    assert!(
        !EcosystemMessageType::WorkloadResponse.requires_response(),
        "WorkloadResponse should not require response"
    );
    assert!(
        !EcosystemMessageType::StatusUpdate.requires_response(),
        "StatusUpdate should not require response"
    );
    assert!(
        !EcosystemMessageType::Error.requires_response(),
        "Error should not require response"
    );
}

#[test]
fn test_all_message_types_is_response() {
    assert!(
        EcosystemMessageType::ResourceResponse.is_response(),
        "ResourceResponse should be response"
    );
    assert!(
        EcosystemMessageType::WorkloadResponse.is_response(),
        "WorkloadResponse should be response"
    );
    assert!(
        !EcosystemMessageType::ResourceRequest.is_response(),
        "ResourceRequest should not be response"
    );
    assert!(
        !EcosystemMessageType::WorkloadRequest.is_response(),
        "WorkloadRequest should not be response"
    );
    assert!(
        !EcosystemMessageType::Heartbeat.is_response(),
        "Heartbeat should not be response"
    );
    assert!(
        !EcosystemMessageType::CapabilityAnnouncement.is_response(),
        "CapabilityAnnouncement should not be response"
    );
    assert!(
        !EcosystemMessageType::StatusUpdate.is_response(),
        "StatusUpdate should not be response"
    );
    assert!(
        !EcosystemMessageType::Error.is_response(),
        "Error should not be response"
    );
}

#[test]
fn test_ecosystem_config_serialization() {
    let config = EcosystemConfig {
        auto_discovery: true,
        discovery_timeout: std::time::Duration::from_secs(45),
        discovery_method: DiscoveryMethodConfig::ConfigFile {
            path: "/tmp/config.yaml".to_string(),
        },
        required_capabilities: vec![],
        optional_capabilities: vec![],
    };

    let serialized = serde_json::to_string(&config).expect("serialize config");
    let deserialized: EcosystemConfig =
        serde_json::from_str(&serialized).expect("deserialize config");

    assert_eq!(config.auto_discovery, deserialized.auto_discovery);
    assert_eq!(
        config.discovery_timeout.as_secs(),
        deserialized.discovery_timeout.as_secs()
    );
    match (&config.discovery_method, &deserialized.discovery_method) {
        (
            DiscoveryMethodConfig::ConfigFile { path: p1 },
            DiscoveryMethodConfig::ConfigFile { path: p2 },
        ) => assert_eq!(p1, p2),
        _ => panic!("discovery_method variant mismatch"),
    }
}

#[test]
fn test_ecosystem_message_serialization() {
    let msg = EcosystemMessage::heartbeat("service-a", "service-b");

    let serialized = serde_json::to_string(&msg).expect("serialize message");
    let deserialized: EcosystemMessage =
        serde_json::from_str(&serialized).expect("deserialize message");

    assert_eq!(msg.id, deserialized.id);
    assert_eq!(msg.from, deserialized.from);
    assert_eq!(msg.to, deserialized.to);
    assert_eq!(msg.message_type, deserialized.message_type);
    assert_eq!(msg.payload, deserialized.payload);
}

#[test]
fn test_service_status_serialization() {
    let status = ServiceStatus::Failed("connection refused".to_string());

    let serialized = serde_json::to_string(&status).expect("serialize status");
    let deserialized: ServiceStatus =
        serde_json::from_str(&serialized).expect("deserialize status");

    assert_eq!(status, deserialized);
    assert_eq!(deserialized.error_message(), Some("connection refused"));
}
