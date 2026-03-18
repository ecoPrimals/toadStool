// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
#![allow(unused_imports)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for ecosystem communication module
//! Target: exercise all branches via public API.

use std::collections::HashMap;
use std::time::SystemTime;

use toadstool::ecosystem::CommunicationManager;
use toadstool::ecosystem::{
    EcosystemMessage, EcosystemMessageType, ServiceChannel, ServiceClient, ServiceStatus,
};
use toadstool_common::primal_identity::ServiceEndpoint;
use toadstool_common::service_discovery::DiscoveredService;

// ─── Constructor and defaults ──────────────────────────────────────────────

#[test]
fn communication_manager_new() {
    let _m = CommunicationManager::new();
}

#[test]
fn communication_manager_default() {
    let _m = CommunicationManager::default();
}

#[tokio::test]
async fn communication_manager_with_timeout() {
    let m = CommunicationManager::with_timeout(std::time::Duration::from_secs(60));
    let channels: Vec<ServiceChannel> = m.get_all_channels().await;
    assert!(channels.is_empty());
}

// ─── Channel management ──────────────────────────────────────────────────────

#[tokio::test]
async fn get_channel_nonexistent() {
    let m = CommunicationManager::new();
    let ch: Option<ServiceChannel> = m.get_channel("nonexistent").await;
    assert!(ch.is_none());
}

#[tokio::test]
async fn get_all_channels_empty() {
    let m = CommunicationManager::new();
    let channels: Vec<ServiceChannel> = m.get_all_channels().await;
    assert!(channels.is_empty());
}

#[tokio::test]
async fn remove_channel_nonexistent_no_op() {
    let m = CommunicationManager::new();
    m.remove_channel("ghost").await;
    let ch: Option<ServiceChannel> = m.get_channel("ghost").await;
    assert!(ch.is_none());
}

#[tokio::test]
async fn update_channel_status_nonexistent_no_op() {
    let m = CommunicationManager::new();
    m.update_channel_status("ghost", ServiceStatus::Connected)
        .await;
    let ch: Option<ServiceChannel> = m.get_channel("ghost").await;
    assert!(ch.is_none());
}

#[tokio::test]
async fn send_heartbeat_nonexistent_err() {
    let m = CommunicationManager::new();
    type ErrType = toadstool::ToadStoolError;
    let result: Result<(), ErrType> = m.send_heartbeat("ghost").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Channel not found") || err.contains("not found"));
}

// ─── create_channel: no endpoint ─────────────────────────────────────────────

#[tokio::test]
async fn create_channel_empty_endpoints_err() {
    let m = CommunicationManager::new();
    let service = DiscoveredService {
        id: "no-ep".to_string(),
        name: "NoEndpoint".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    type ErrType = toadstool::ToadStoolError;
    let result: Result<ServiceChannel, ErrType> = m.create_channel(&service).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No endpoint"));
}

// ─── create_channel with endpoint (networking feature) ───────────────────────

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn create_channel_with_http_endpoint_succeeds() {
    let m = CommunicationManager::new();
    let service = DiscoveredService {
        id: "with-ep".to_string(),
        name: "WithEndpoint".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 9999)],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    type ChResult = Result<ServiceChannel, toadstool::ToadStoolError>;
    let result: ChResult = m.create_channel(&service).await;
    assert!(result.is_ok());
    let ch = result.unwrap();
    assert_eq!(ch.service_id, "with-ep");
    assert_eq!(ch.service_name, "WithEndpoint");
    assert_eq!(ch.status, ServiceStatus::Discovered);
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn create_channel_idempotent() {
    let m = CommunicationManager::new();
    let service = DiscoveredService {
        id: "dup".to_string(),
        name: "Dup".to_string(),
        version: "1.0".to_string(),
        capabilities: vec![],
        endpoints: vec![ServiceEndpoint::http("localhost", 8888)],
        metadata: HashMap::new(),
        discovered_at: SystemTime::now(),
        last_seen: SystemTime::now(),
        healthy: true,
    };
    let ch1 = m.create_channel(&service).await.expect("create channel 1");
    let ch2 = m.create_channel(&service).await.expect("create channel 2");
    assert_eq!(ch1.service_id, ch2.service_id);
    let all: Vec<ServiceChannel> = m.get_all_channels().await;
    assert_eq!(all.len(), 1);
}

// ─── send_message and check_health (networking disabled) ─────────────────────

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn send_message_disabled_returns_fallback() {
    let m = CommunicationManager::new();
    let ch = ServiceChannel {
        service_id: "t".to_string(),
        service_name: "T".to_string(),
        endpoint: "http://localhost:1".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: SystemTime::now(),
        status: ServiceStatus::Connected,
    };
    let msg = EcosystemMessage::new(
        "sender".to_string(),
        "receiver".to_string(),
        EcosystemMessageType::Heartbeat,
        serde_json::json!({}),
    );
    let result: Result<EcosystemMessage, _> = m.send_message(&ch, msg).await;
    assert!(result.is_ok());
    let resp = result.unwrap();
    assert_eq!(resp.to, "sender");
    assert!(
        resp.payload
            .get("status")
            .and_then(serde_json::Value::as_str)
            == Some("networking_disabled")
    );
}

#[cfg(not(feature = "networking"))]
#[tokio::test]
async fn check_health_disabled_succeeds() {
    let m = CommunicationManager::new();
    let ch = ServiceChannel {
        service_id: "t".to_string(),
        service_name: "T".to_string(),
        endpoint: "http://localhost:1".to_string(),
        client: ServiceClient::Disabled,
        last_heartbeat: SystemTime::now(),
        status: ServiceStatus::Connected,
    };
    let result: Result<(), _> = m.check_health(&ch).await;
    assert!(result.is_ok());
}

// ─── EcosystemMessage and ServiceStatus types ──────────────────────────────

#[test]
fn ecosystem_message_heartbeat_factory() {
    let msg = EcosystemMessage::heartbeat("a".to_string(), "b".to_string());
    assert_eq!(msg.message_type, EcosystemMessageType::Heartbeat);
}

#[test]
fn ecosystem_message_error_factory() {
    let msg = EcosystemMessage::error("a".to_string(), "b".to_string(), "err".to_string());
    assert_eq!(msg.message_type, EcosystemMessageType::Error);
}

#[test]
fn service_status_is_usable() {
    assert!(ServiceStatus::Connected.is_usable());
    assert!(!ServiceStatus::Discovered.is_usable());
    assert!(!ServiceStatus::Disconnected.is_usable());
}

#[test]
fn service_status_is_error() {
    let failed = ServiceStatus::Failed("e".to_string());
    assert!(failed.is_error());
    assert_eq!(failed.error_message(), Some("e"));
}
