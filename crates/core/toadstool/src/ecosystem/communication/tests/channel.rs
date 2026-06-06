// SPDX-License-Identifier: AGPL-3.0-or-later
//! Channel management and message routing tests

use crate::ecosystem::{CommunicationManager, ServiceStatus};
#[cfg(not(feature = "networking"))]
use crate::ecosystem::{ServiceChannel, ServiceClient};

#[tokio::test]
async fn test_communication_manager_creation() {
    let manager = CommunicationManager::new();
    let channels = manager.get_all_channels().await;
    assert_eq!(channels.len(), 0);
}

#[tokio::test]
async fn test_custom_timeout() {
    let manager = CommunicationManager::with_timeout(std::time::Duration::from_secs(60));
    assert_eq!(manager.default_timeout.as_secs(), 60);
}

#[test]
fn test_fallback_response() {
    use crate::ecosystem::EcosystemMessageType;

    let _manager = CommunicationManager::new();
    let _original = crate::ecosystem::EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );

    #[cfg(not(feature = "networking"))]
    {
        let response = _manager.fallback_response(_original.clone());
        assert_eq!(response.to, _original.from);
        assert_eq!(response.from, "toadstool_local");
        let status = response.payload.get("status").and_then(|v| v.as_str());
        assert_eq!(status, Some("networking_disabled"));
    }
}

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
    let err = result.expect_err("expected channel not found error");
    assert!(err.to_string().contains("Channel not found"));
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
        last_heartbeat: std::time::SystemTime::now(),
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("test-service".to_string(), channel.clone());
    }

    let retrieved = manager.get_channel("test-service").await;
    assert!(retrieved.is_some());
    let retrieved = retrieved.expect("test-service channel should exist");
    assert_eq!(retrieved.service_id, "test-service");
    assert_eq!(retrieved.service_name, "Test Service");
    assert_eq!(retrieved.status, ServiceStatus::Connected);

    let all = manager.get_all_channels().await;
    assert_eq!(all.len(), 1);

    manager
        .update_channel_status("test-service", ServiceStatus::Disconnected)
        .await;
    let updated = manager
        .get_channel("test-service")
        .await
        .expect("test-service channel should exist after update");
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
        last_heartbeat: std::time::SystemTime::now(),
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

#[tokio::test]
async fn test_with_timeout_zero_duration() {
    let manager = CommunicationManager::with_timeout(std::time::Duration::ZERO);
    assert_eq!(manager.default_timeout, std::time::Duration::ZERO);
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
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No endpoint discovered")
    );
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
    let channel = result.expect("create_channel should succeed");
    assert_eq!(channel.service_id, "svc-with-ep");
    assert_eq!(channel.service_name, "WithEndpoint");
    assert_eq!(channel.endpoint, "http://localhost:9999");
    assert_eq!(channel.status, ServiceStatus::Discovered);
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_send_message_degraded_mode_returns_fallback() {
    use crate::ecosystem::EcosystemMessageType;

    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "test".to_string(),
        service_name: "Test".to_string(),
        endpoint: "http://localhost:1234".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: std::time::SystemTime::now(),
        status: ServiceStatus::Connected,
    };

    let msg = crate::ecosystem::EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({"ping": true}),
    );

    let result = manager.send_message(&channel, msg.clone()).await;
    assert!(result.is_ok());
    let response = result.expect("send_message should succeed");
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
        last_heartbeat: std::time::SystemTime::now(),
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
        last_heartbeat: std::time::SystemTime::now() - std::time::Duration::from_secs(300),
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("heartbeat-svc".to_string(), channel.clone());
    }

    let result = manager.send_heartbeat("heartbeat-svc").await;
    assert!(result.is_ok());

    let updated = manager
        .get_channel("heartbeat-svc")
        .await
        .expect("heartbeat-svc channel should exist");
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
        last_heartbeat: std::time::SystemTime::now(),
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
        last_heartbeat: std::time::SystemTime::now(),
        status: ServiceStatus::Discovered,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("status-svc".to_string(), channel);
    }

    manager
        .update_channel_status("status-svc", ServiceStatus::Connected)
        .await;

    let updated = manager
        .get_channel("status-svc")
        .await
        .expect("status-svc channel should exist");
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

    let ch1 = manager
        .create_channel(&service)
        .await
        .expect("first create_channel should succeed");
    let ch2 = manager
        .create_channel(&service)
        .await
        .expect("second create_channel should succeed");
    assert_eq!(ch1.service_id, ch2.service_id);
    let all = manager.get_all_channels().await;
    assert_eq!(all.len(), 1);
}
