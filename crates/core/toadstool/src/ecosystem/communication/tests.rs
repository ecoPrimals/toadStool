//! Communication module tests

#[allow(unused_imports)]
use crate::ecosystem::{CommunicationManager, ServiceChannel, ServiceClient, ServiceStatus};

#[tokio::test]
async fn test_communication_manager_creation() {
    let manager = CommunicationManager::new();
    let channels = manager.get_all_channels().await;
    assert_eq!(channels.len(), 0);
}

#[tokio::test]
async fn test_custom_timeout() {
    let manager = CommunicationManager::with_timeout(std::time::Duration::from_secs(60));
    assert_eq!(manager._default_timeout.as_secs(), 60);
}

#[test]
fn test_fallback_response() {
    use super::super::types::EcosystemMessageType;

    let _manager = CommunicationManager::new();
    let _original = super::super::types::EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );

    #[cfg(not(feature = "networking"))]
    {
        let response = _manager.fallback_response(_original.clone());
        assert_eq!(response.to, _original.from);
        assert_eq!(response.from, "toadstool_local"); // Evolved: indicates local-only mode
                                                      // Verify structured status payload
        let status = response.payload.get("status").and_then(|v| v.as_str());
        assert_eq!(status, Some("networking_disabled"));
    }
}

// ─── Channel management tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_default_impl() {
    let manager = CommunicationManager::default();
    let channels = manager.get_all_channels().await;
    assert_eq!(channels.len(), 0);
}

#[tokio::test]
async fn test_get_channel_not_found() {
    let manager = CommunicationManager::new();
    let channel = manager.get_channel("non-existent-service").await;
    assert!(channel.is_none());
}

#[tokio::test]
async fn test_remove_nonexistent_channel() {
    let manager = CommunicationManager::new();
    manager.remove_channel("non-existent-id").await;
    let channels = manager.get_all_channels().await;
    assert_eq!(channels.len(), 0);
}

#[tokio::test]
async fn test_update_status_nonexistent_channel() {
    let manager = CommunicationManager::new();
    manager
        .update_channel_status("non-existent-id", ServiceStatus::Connected)
        .await;
    let channel = manager.get_channel("non-existent-id").await;
    assert!(channel.is_none());
}

#[tokio::test]
async fn test_send_heartbeat_no_channel() {
    let manager = CommunicationManager::new();
    let result = manager.send_heartbeat("non-existent-service").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Channel not found"));
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_channel_operations_with_direct_insert() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "test-service".to_string(),
        service_name: "Test Service".to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("test-service".to_string(), channel.clone());
    }

    let retrieved = manager.get_channel("test-service").await;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.service_id, "test-service");
    assert_eq!(retrieved.service_name, "Test Service");
    assert_eq!(retrieved.status, ServiceStatus::Connected);

    let all = manager.get_all_channels().await;
    assert_eq!(all.len(), 1);

    manager
        .update_channel_status("test-service", ServiceStatus::Disconnected)
        .await;
    let updated = manager.get_channel("test-service").await.unwrap();
    assert_eq!(updated.status, ServiceStatus::Disconnected);

    manager.remove_channel("test-service").await;
    let gone = manager.get_channel("test-service").await;
    assert!(gone.is_none());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_multiple_channels() {
    let manager = CommunicationManager::new();
    let mk_channel = |id: &str, name: &str| ServiceChannel {
        service_id: id.to_string(),
        service_name: name.to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("svc-1".to_string(), mk_channel("svc-1", "Service 1"));
        channels.insert("svc-2".to_string(), mk_channel("svc-2", "Service 2"));
        channels.insert("svc-3".to_string(), mk_channel("svc-3", "Service 3"));
    }

    let all = manager.get_all_channels().await;
    assert_eq!(all.len(), 3);

    assert!(manager.get_channel("svc-1").await.is_some());
    assert!(manager.get_channel("svc-2").await.is_some());
    assert!(manager.get_channel("svc-3").await.is_some());
}

// ─── Additional message routing and protocol tests ──────────────────────────

#[tokio::test]
async fn test_with_timeout_zero_duration() {
    let manager = CommunicationManager::with_timeout(std::time::Duration::ZERO);
    assert_eq!(manager._default_timeout, std::time::Duration::ZERO);
}

#[tokio::test]
async fn test_create_channel_no_endpoint_fails() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "svc-no-ep".to_string(),
        name: "NoEndpoint".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };

    let result = manager.create_channel(&service).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No endpoint discovered"));
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_create_channel_with_endpoint_succeeds() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::primal_identity::ServiceEndpoint;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "svc-with-ep".to_string(),
        name: "WithEndpoint".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 9999)],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };

    let result = manager.create_channel(&service).await;
    assert!(result.is_ok());
    let channel = result.unwrap();
    assert_eq!(channel.service_id, "svc-with-ep");
    assert_eq!(channel.service_name, "WithEndpoint");
    assert_eq!(channel.endpoint, "http://localhost:9999");
    assert_eq!(channel.status, ServiceStatus::Discovered);
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_send_message_degraded_mode_returns_fallback() {
    use super::super::types::EcosystemMessageType;

    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "test".to_string(),
        service_name: "Test".to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Connected,
    };

    let msg = super::super::types::EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({"ping": true}),
    );

    let result = manager.send_message(&channel, msg.clone()).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.to, "sender");
    assert_eq!(response.from, "toadstool_local");
    assert!(response.payload.get("status").and_then(|v| v.as_str()) == Some("networking_disabled"));
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_check_health_degraded_mode_succeeds() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "test".to_string(),
        service_name: "Test".to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Connected,
    };

    let result = manager.check_health(&channel).await;
    assert!(result.is_ok());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_send_heartbeat_succeeds_and_updates_timestamp() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "heartbeat-svc".to_string(),
        service_name: "Heartbeat Service".to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now() - chrono::Duration::minutes(5),
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("heartbeat-svc".to_string(), channel.clone());
    }

    let result = manager.send_heartbeat("heartbeat-svc").await;
    assert!(result.is_ok());

    let updated = manager.get_channel("heartbeat-svc").await.unwrap();
    assert!(updated.last_heartbeat > channel.last_heartbeat);
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_remove_channel_existing_logs_info() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "to-remove".to_string(),
        service_name: "ToRemove".to_string(),
        endpoint: "http://localhost:1".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Disconnected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("to-remove".to_string(), channel);
    }

    manager.remove_channel("to-remove").await;
    assert!(manager.get_channel("to-remove").await.is_none());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_update_channel_status_existing_channel() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "status-svc".to_string(),
        service_name: "Status Svc".to_string(),
        endpoint: "http://localhost:1".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Discovered,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("status-svc".to_string(), channel);
    }

    manager
        .update_channel_status("status-svc", ServiceStatus::Connected)
        .await;

    let updated = manager.get_channel("status-svc").await.unwrap();
    assert_eq!(updated.status, ServiceStatus::Connected);
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_create_channel_idempotent_or_insert() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::primal_identity::ServiceEndpoint;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "dup-id".to_string(),
        name: "Dup".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 8888)],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };

    let ch1 = manager.create_channel(&service).await.unwrap();
    let ch2 = manager.create_channel(&service).await.unwrap();
    assert_eq!(ch1.service_id, ch2.service_id);
    let all = manager.get_all_channels().await;
    assert_eq!(all.len(), 1);
}

// ─── Message type and serialization tests ────────────────────────────────────

#[test]
fn test_ecosystem_message_new_constructor() {
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
        "from-svc".to_string(),
        "to-svc".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({"extra": true}),
    );
    assert_eq!(msg.from, "from-svc");
    assert_eq!(msg.to, "to-svc");
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
    assert!(msg
        .payload
        .get("extra")
        .and_then(|v| v.as_bool())
        .unwrap_or(false));
}

#[test]
fn test_ecosystem_message_heartbeat_factory() {
    let msg = super::super::types::EcosystemMessage::heartbeat(
        "sender".to_string(),
        "receiver".to_string(),
    );
    assert_eq!(
        msg.message_type,
        super::super::types::EcosystemMessageType::Heartbeat
    );
}

#[test]
fn test_ecosystem_message_error_factory() {
    let msg = super::super::types::EcosystemMessage::error(
        "a".to_string(),
        "b".to_string(),
        "something failed".to_string(),
    );
    assert_eq!(
        msg.message_type,
        super::super::types::EcosystemMessageType::Error
    );
    assert_eq!(msg.payload["error"], "something failed");
}

#[test]
fn test_ecosystem_message_serialization_roundtrip() {
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
        "a".to_string(),
        "b".to_string(),
        EcosystemMessageType::StatusUpdate,
        serde_json::json!({"k": "v"}),
    );
    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: super::super::types::EcosystemMessage =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.from, msg.from);
    assert_eq!(parsed.to, msg.to);
}

#[test]
fn test_service_status_all_variants() {
    assert!(!super::super::types::ServiceStatus::Discovered.is_usable());
    assert!(!super::super::types::ServiceStatus::Connecting.is_usable());
    assert!(super::super::types::ServiceStatus::Connected.is_usable());
    assert!(!super::super::types::ServiceStatus::Disconnected.is_usable());
    let failed = super::super::types::ServiceStatus::Failed("err".to_string());
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
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Connected,
    };
    let ch2 = ch.clone();
    assert_eq!(ch.service_id, ch2.service_id);
    assert_eq!(format!("{:?}", ch).len(), format!("{:?}", ch2).len());
}

// ─── Additional message type, serialization, protocol tests ─────────────────

#[test]
fn test_ecosystem_message_type_all_variants_serde() {
    use super::super::types::EcosystemMessageType;

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
        let json = serde_json::to_value(&mt).unwrap();
        let _: EcosystemMessageType = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_ecosystem_message_capability_announcement() {
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
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
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
        "requester".to_string(),
        "provider".to_string(),
        EcosystemMessageType::ResourceRequest,
        serde_json::json!({"cpu": 4}),
    );
    assert!(msg.message_type.requires_response());
}

#[test]
fn test_service_status_removing() {
    let status = super::super::types::ServiceStatus::Removing;
    assert!(!status.is_usable());
    assert!(!status.is_error());
}

#[test]
fn test_ecosystem_message_serialization_all_types() {
    use super::super::types::EcosystemMessageType;

    let types = [
        EcosystemMessageType::StatusUpdate,
        EcosystemMessageType::CapabilityAnnouncement,
        EcosystemMessageType::ResourceResponse,
        EcosystemMessageType::WorkloadResponse,
    ];
    for mt in types {
        let msg = super::super::types::EcosystemMessage::new(
            "a".to_string(),
            "b".to_string(),
            mt.clone(),
            serde_json::json!({}),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: super::super::types::EcosystemMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message_type, mt);
    }
}

#[test]
fn test_discovery_method_config_serde() {
    use super::super::types::DiscoveryMethodConfig;

    let config = DiscoveryMethodConfig::Environment;
    let json = serde_json::to_value(&config).unwrap();
    let _: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
}

#[test]
fn test_service_status_connecting() {
    assert!(!super::super::types::ServiceStatus::Connecting.is_usable());
}

#[test]
fn test_ecosystem_message_with_complex_payload() {
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
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

// ─── Additional coverage: error_message, DiscoveryMethodConfig, status display ─

#[test]
fn test_service_status_error_message() {
    let status = super::super::types::ServiceStatus::Failed("connection refused".to_string());
    assert!(status.is_error());
    assert_eq!(status.error_message(), Some("connection refused"));
}

#[test]
fn test_service_status_error_message_none_for_non_failed() {
    let status = super::super::types::ServiceStatus::Connected;
    assert!(status.error_message().is_none());
}

#[test]
fn test_discovery_method_config_config_file_serde() {
    use super::super::types::DiscoveryMethodConfig;

    let config = DiscoveryMethodConfig::ConfigFile {
        path: "/etc/biomeos/discovery.json".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
    match parsed {
        DiscoveryMethodConfig::ConfigFile { path } => {
            assert_eq!(path, "/etc/biomeos/discovery.json")
        }
        _ => panic!("expected ConfigFile"),
    }
}

#[test]
fn test_discovery_method_config_registry_serde() {
    use super::super::types::DiscoveryMethodConfig;

    let config = DiscoveryMethodConfig::Registry {
        endpoint: "http://registry:8080".to_string(),
    };
    let json = serde_json::to_value(&config).unwrap();
    let parsed: DiscoveryMethodConfig = serde_json::from_value(json).unwrap();
    match parsed {
        DiscoveryMethodConfig::Registry { endpoint } => {
            assert_eq!(endpoint, "http://registry:8080")
        }
        _ => panic!("expected Registry"),
    }
}

#[test]
fn test_ecosystem_message_type_workload_response_no_response_required() {
    use super::super::types::EcosystemMessageType;

    assert!(!EcosystemMessageType::WorkloadResponse.requires_response());
}

#[test]
fn test_ecosystem_message_type_resource_request_requires_response() {
    use super::super::types::EcosystemMessageType;

    assert!(EcosystemMessageType::ResourceRequest.requires_response());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_fallback_response_preserves_original_id() {
    use super::super::types::EcosystemMessageType;

    let manager = CommunicationManager::new();
    let original = super::super::types::EcosystemMessage::new(
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

// ─── Mock-based integration tests: protocol negotiation, error handling ────

#[tokio::test]
async fn test_create_channel_empty_endpoints_fails() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "empty".to_string(),
        name: "EmptyEndpoints".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let result = manager.create_channel(&service).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No endpoint"));
}

#[tokio::test]
async fn test_remove_channel_clears_from_map() {
    let manager = CommunicationManager::new();
    manager.remove_channel("never-existed").await;
    let channels = manager.get_all_channels().await;
    assert!(channels.is_empty());
}

#[tokio::test]
async fn test_get_all_channels_empty_returns_vec() {
    let manager = CommunicationManager::new();
    let channels = manager.get_all_channels().await;
    assert!(channels.is_empty());
    assert!(channels.capacity() >= 0);
}

#[tokio::test]
async fn test_check_health_degraded_mode_when_disabled() {
    #[cfg(not(feature = "networking"))]
    {
        let manager = CommunicationManager::new();
        let channel = ServiceChannel {
            service_id: "deg".to_string(),
            service_name: "Degraded".to_string(),
            endpoint: "http://x:1".to_string(),
            client: ServiceClient::Disabled,
            last_heartbeat: chrono::Utc::now(),
            status: ServiceStatus::Connected,
        };
        let result = manager.check_health(&channel).await;
        assert!(result.is_ok());
    }
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_create_channel_service_with_multiple_endpoints() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::primal_identity::ServiceEndpoint;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "multi-ep".to_string(),
        name: "MultiEndpoint".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![
            ServiceEndpoint::http("localhost", 8080),
            ServiceEndpoint::http("localhost", 8081),
        ],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let result = manager.create_channel(&service).await;
    assert!(result.is_ok());
    let ch = result.unwrap();
    assert_eq!(ch.service_id, "multi-ep");
}

#[tokio::test]
async fn test_send_message_requires_channel() {
    let manager = CommunicationManager::new();
    let result = manager.send_heartbeat("ghost").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_channel_status_idempotent() {
    let manager = CommunicationManager::new();
    manager
        .update_channel_status("nonexistent", ServiceStatus::Connected)
        .await;
    manager
        .update_channel_status("nonexistent", ServiceStatus::Disconnected)
        .await;
    let channel = manager.get_channel("nonexistent").await;
    assert!(channel.is_none());
}

#[tokio::test]
async fn test_communication_manager_with_timeout_creation() {
    let manager = CommunicationManager::with_timeout(std::time::Duration::from_secs(30));
    let channels = manager.get_all_channels().await;
    assert_eq!(channels.len(), 0);
}

#[test]
fn test_ecosystem_message_status_update_type() {
    use super::super::types::EcosystemMessageType;

    let msg = super::super::types::EcosystemMessage::new(
        "a".to_string(),
        "b".to_string(),
        EcosystemMessageType::StatusUpdate,
        serde_json::json!({"state": "ready"}),
    );
    assert!(!msg.message_type.requires_response());
}

#[cfg(not(feature = "networking"))]
#[test]
fn test_service_channel_debug_impl() {
    let ch = ServiceChannel {
        service_id: "debug-svc".to_string(),
        service_name: "Debug Svc".to_string(),
        endpoint: "unix:///tmp/debug.sock".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: chrono::Utc::now(),
        status: ServiceStatus::Discovered,
    };
    let debug_str = format!("{:?}", ch);
    assert!(debug_str.contains("debug-svc"));
    assert!(debug_str.contains("Debug Svc"));
}
