// SPDX-License-Identifier: AGPL-3.0-or-later
//! Message type and serialization tests

use crate::ecosystem::{
    DiscoveryMethodConfig, EcosystemMessage, EcosystemMessageType, ServiceStatus,
};
#[cfg(not(feature = "networking"))]
use crate::ecosystem::{ServiceChannel, ServiceClient};

#[test]
fn test_ecosystem_message_new_constructor() {
    let msg = EcosystemMessage::new(
        "from-svc".to_string(),
        "to-svc".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({"extra": true}),
    );
    assert_eq!(msg.from, "from-svc");
    assert_eq!(msg.to, "to-svc");
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
    assert!(
        msg.payload
            .get("extra")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    );
}

#[test]
fn test_ecosystem_message_heartbeat_factory() {
    let msg = EcosystemMessage::heartbeat("sender".to_string(), "receiver".to_string());
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
}

#[test]
fn test_ecosystem_message_error_factory() {
    let msg = EcosystemMessage::error(
        "a".to_string(),
        "b".to_string(),
        "something failed".to_string(),
    );
    assert_eq!(msg.message_type, EcosystemMessageType::Error);
    assert_eq!(msg.payload["error"], "something failed");
}

#[test]
fn test_ecosystem_message_serialization_roundtrip() {
    let msg = EcosystemMessage::new(
        "a".to_string(),
        "b".to_string(),
        EcosystemMessageType::StatusUpdate,
        serde_json::json!({"k": "v"}),
    );
    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: EcosystemMessage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.from, msg.from);
    assert_eq!(parsed.to, msg.to);
}

#[test]
fn test_service_status_all_variants() {
    assert!(!ServiceStatus::Discovered.is_usable());
    assert!(!ServiceStatus::Connecting.is_usable());
    assert!(ServiceStatus::Connected.is_usable());
    assert!(!ServiceStatus::Disconnected.is_usable());
    let failed = ServiceStatus::Failed("err".to_string());
    assert!(failed.is_error());
    assert_eq!(failed.error_message(), Some("err"));
}

#[cfg(not(feature = "networking"))]
#[test]
fn test_service_channel_debug_clone() {
    let ch = ServiceChannel {
        service_id: "id".to_string(),
        service_name: "Name".to_string(),
        endpoint: "http://x".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: std::time::SystemTime::now(),
        status: ServiceStatus::Connected,
    };
    let ch2 = ch.clone();
    assert_eq!(ch.service_id, ch2.service_id);
    assert_eq!(format!("{ch:?}").len(), format!("{ch2:?}").len());
}

#[test]
fn test_ecosystem_message_type_all_variants_serde() {
    for mt in [
        EcosystemMessageType::Heartbeat,
        EcosystemMessageType::CapabilityAnnouncement,
        EcosystemMessageType::ResourceRequest,
        EcosystemMessageType::ResourceResponse,
        EcosystemMessageType::WorkloadRequest,
        EcosystemMessageType::WorkloadResponse,
        EcosystemMessageType::StatusUpdate,
        EcosystemMessageType::Error,
    ] {
        let json = serde_json::to_value(&mt).expect("serialize message type");
        let _: EcosystemMessageType =
            serde_json::from_value(json).expect("deserialize message type");
    }
}

#[test]
fn test_ecosystem_message_capability_announcement() {
    let msg = EcosystemMessage::new(
        "svc-a".to_string(),
        "svc-b".to_string(),
        EcosystemMessageType::CapabilityAnnouncement,
        serde_json::json!({"caps": ["compute"]}),
    );
    assert_eq!(
        msg.message_type,
        EcosystemMessageType::CapabilityAnnouncement
    );
}

#[test]
fn test_ecosystem_message_resource_request() {
    let msg = EcosystemMessage::new(
        "requester".to_string(),
        "provider".to_string(),
        EcosystemMessageType::ResourceRequest,
        serde_json::json!({"cpu": 4}),
    );
    assert!(msg.message_type.requires_response());
}

#[test]
fn test_service_status_removing() {
    let status = ServiceStatus::Removing;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_ecosystem_message_serialization_all_types() {
    use EcosystemMessageType;

    let types = [
        EcosystemMessageType::StatusUpdate,
        EcosystemMessageType::CapabilityAnnouncement,
        EcosystemMessageType::ResourceResponse,
        EcosystemMessageType::WorkloadResponse,
    ];
    for mt in types {
        let msg = EcosystemMessage::new(
            "a".to_string(),
            "b".to_string(),
            mt.clone(),
            serde_json::json!({}),
        );
        let json = serde_json::to_string(&msg).expect("serialize message");
        let parsed: EcosystemMessage = serde_json::from_str(&json).expect("deserialize message");
        assert_eq!(parsed.message_type, mt);
    }
}

#[test]
fn test_discovery_method_config_serde() {
    let config = DiscoveryMethodConfig::Environment;
    let json = serde_json::to_value(&config).expect("serialize config");
    let _: DiscoveryMethodConfig = serde_json::from_value(json).expect("deserialize config");
}

#[test]
fn test_service_status_connecting() {
    assert!(!ServiceStatus::Connecting.is_usable());
}

#[test]
fn test_ecosystem_message_with_complex_payload() {
    let msg = EcosystemMessage::new(
        "from".to_string(),
        "to".to_string(),
        EcosystemMessageType::WorkloadRequest,
        serde_json::json!({
            "job_id": "j1",
            "resources": {"cpu": 8},
            "nested": {"a": 1}
        }),
    );
    assert!(msg.payload.get("nested").is_some());
}

#[test]
fn test_service_status_error_message() {
    let status = ServiceStatus::Failed("connection refused".to_string());
    assert!(status.is_error());
    assert_eq!(status.error_message(), Some("connection refused"));
}

#[test]
fn test_service_status_error_message_none_for_non_failed() {
    let status = ServiceStatus::Connected;
    assert!(status.error_message().is_none());
}

#[test]
fn test_discovery_method_config_config_file_serde() {
    let config = DiscoveryMethodConfig::ConfigFile {
        path: "/etc/biomeos/discovery.json".to_string(),
    };
    let json = serde_json::to_value(&config).expect("serialize config");
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).expect("deserialize config");
    match parsed {
        DiscoveryMethodConfig::ConfigFile { path } => {
            assert_eq!(path, "/etc/biomeos/discovery.json")
        }
        _ => panic!("expected ConfigFile"),
    }
}

#[test]
fn test_discovery_method_config_registry_serde() {
    let config = DiscoveryMethodConfig::Registry {
        endpoint: "http://registry:8080".to_string(),
    };
    let json = serde_json::to_value(&config).expect("serialize config");
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).expect("deserialize config");
    match parsed {
        DiscoveryMethodConfig::Registry { endpoint } => {
            assert_eq!(endpoint, "http://registry:8080")
        }
        _ => panic!("expected Registry"),
    }
}

#[test]
fn test_ecosystem_message_type_workload_response_no_response_required() {
    use EcosystemMessageType;

    assert!(!EcosystemMessageType::WorkloadResponse.requires_response());
}

#[test]
fn test_ecosystem_message_type_resource_request_requires_response() {
    assert!(EcosystemMessageType::ResourceRequest.requires_response());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_fallback_response_preserves_original_id() {
    let manager = CommunicationManager::new();
    let original = EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );
    let original_id_str = original.id.to_string();
    let response = manager.fallback_response(original);
    let stored_id = response
        .payload
        .get("original_message_id")
        .and_then(|v| v.as_str());
    assert_eq!(stored_id, Some(original_id_str.as_str()));
}

#[test]
fn test_discovery_method_config_auto_serde() {
    let config = DiscoveryMethodConfig::Auto;
    let json = serde_json::to_value(&config).expect("serialize Auto");
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).expect("deserialize Auto");
    assert!(matches!(parsed, DiscoveryMethodConfig::Auto));
}

#[test]
fn test_discovery_method_config_mdns_serde() {
    let config = DiscoveryMethodConfig::Mdns;
    let json = serde_json::to_value(&config).expect("serialize Mdns");
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).expect("deserialize Mdns");
    assert!(matches!(parsed, DiscoveryMethodConfig::Mdns));
}

#[test]
fn test_ecosystem_message_new_sets_id_and_timestamp() {
    let msg = EcosystemMessage::new(
        "a".to_string(),
        "b".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );
    assert!(!msg.id.is_nil());
    assert_eq!(msg.from, "a");
    assert_eq!(msg.to, "b");
}

#[test]
fn test_ecosystem_message_type_is_response() {
    use EcosystemMessageType;

    assert!(EcosystemMessageType::ResourceResponse.is_response());
    assert!(EcosystemMessageType::WorkloadResponse.is_response());
    assert!(!EcosystemMessageType::Heartbeat.is_response());
    assert!(!EcosystemMessageType::StatusUpdate.is_response());
}

#[cfg(not(feature = "networking"))]
#[test]
fn test_fallback_response_contains_reason_and_mode() {
    use EcosystemMessageType;

    let manager = CommunicationManager::new();
    let original = EcosystemMessage::new(
        "src".to_string(),
        "dst".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );
    let response = manager.fallback_response(original);
    assert_eq!(
        response.payload.get("reason").and_then(|v| v.as_str()),
        Some("Networking feature not compiled")
    );
    assert_eq!(
        response.payload.get("mode").and_then(|v| v.as_str()),
        Some("degraded")
    );
}

#[test]
fn test_service_status_discovered_serialization() {
    let status = ServiceStatus::Discovered;
    let json = serde_json::to_value(&status).expect("serialize Discovered");
    let parsed: ServiceStatus = serde_json::from_value(json).expect("deserialize Discovered");
    assert_eq!(parsed, status);
}

#[test]
fn test_service_status_connected_serialization() {
    let status = ServiceStatus::Connected;
    let json = serde_json::to_value(&status).expect("serialize Connected");
    let parsed: ServiceStatus = serde_json::from_value(json).expect("deserialize Connected");
    assert_eq!(parsed, status);
}

#[test]
fn test_ecosystem_message_status_update_type() {
    let msg = EcosystemMessage::new(
        "a".to_string(),
        "b".to_string(),
        EcosystemMessageType::StatusUpdate,
        serde_json::json!({"state": "ready"}),
    );
    assert!(!msg.message_type.requires_response());
}

#[test]
fn test_ecosystem_message_type_error_variant() {
    use EcosystemMessageType;

    let mt = EcosystemMessageType::Error;
    let json = serde_json::to_value(&mt).expect("serialize");
    let _: EcosystemMessageType = serde_json::from_value(json).expect("deserialize");
}

#[test]
fn test_service_status_disconnected_is_not_usable() {
    assert!(!ServiceStatus::Disconnected.is_usable());
}
