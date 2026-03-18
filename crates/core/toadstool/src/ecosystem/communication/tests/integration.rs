// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration and edge-case tests

use crate::ecosystem::{CommunicationManager, ServiceStatus};
#[cfg(not(feature = "networking"))]
use crate::ecosystem::{ServiceChannel, ServiceClient};

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
    assert!(
        result
            .expect_err("expected no endpoint error")
            .to_string()
            .contains("No endpoint")
    );
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
            last_heartbeat: std::time::SystemTime::now(),
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
    let ch = result.expect("create_channel should succeed");
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

#[cfg(not(feature = "networking"))]
#[test]
fn test_service_channel_debug_impl() {
    let ch = ServiceChannel {
        service_id: "debug-svc".to_string(),
        service_name: "Debug Svc".to_string(),
        endpoint: "unix:///tmp/debug.sock".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: std::time::SystemTime::now(),
        status: ServiceStatus::Discovered,
    };
    let debug_str = format!("{ch:?}");
    assert!(debug_str.contains("debug-svc"));
    assert!(debug_str.contains("Debug Svc"));
}

#[tokio::test]
async fn test_communication_manager_new_uses_default_timeout() {
    let manager = CommunicationManager::new();
    let channels = manager.get_all_channels().await;
    assert!(channels.is_empty());
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_create_channel_with_http_endpoint() {
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool_common::primal_identity::ServiceEndpoint;
    use toadstool_common::service_discovery::DiscoveredService;

    let manager = CommunicationManager::new();
    let service = DiscoveredService {
        id: "http-svc".to_string(),
        name: "HttpService".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("127.0.0.1", 12345)],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let result = manager.create_channel(&service).await;
    assert!(result.is_ok());
    let channel = result.unwrap();
    assert_eq!(channel.service_id, "http-svc");
    assert_eq!(channel.endpoint, "http://127.0.0.1:12345");
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn test_send_heartbeat_updates_last_heartbeat() {
    let manager = CommunicationManager::new();
    let channel = ServiceChannel {
        service_id: "hb-svc".to_string(),
        service_name: "HeartbeatSvc".to_string(),
        endpoint: "http://localhost:1".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: std::time::SystemTime::UNIX_EPOCH,
        status: ServiceStatus::Connected,
    };

    {
        let mut channels = manager.channels.write().await;
        channels.insert("hb-svc".to_string(), channel.clone());
    }

    let result = manager.send_heartbeat("hb-svc").await;
    assert!(result.is_ok());

    let updated = manager.get_channel("hb-svc").await.unwrap();
    assert!(updated.last_heartbeat > channel.last_heartbeat);
}
