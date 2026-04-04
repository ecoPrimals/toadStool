// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for [`super::ProtocolClient`] and related client helpers.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;

use super::routing;
use super::{ProtocolClient, SimpleMessageHandler};
use crate::config::{ProtocolConfig, RoutingStrategy};
use crate::types::{
    HealthStatus, MessageFormat, MessageHandler, MessagePriority, ProtocolError, ProtocolEvent,
    ProtocolMessage, ServiceEndpoint, ServiceInfo, TransportType,
};
use toadstool_config::defaults;

fn create_test_config() -> ProtocolConfig {
    use crate::config::HealthConfig;
    ProtocolConfig {
        health_config: HealthConfig {
            base: toadstool_common::config_bases::HealthCheckConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

fn create_test_service(id: &str, name: &str, status: HealthStatus) -> crate::types::ServiceInfo {
    crate::types::ServiceInfo {
        id: Arc::from(id),
        name: Arc::from(name),
        version: "1.0.0".to_string(),
        endpoints: vec![ServiceEndpoint {
            id: format!("{id}-endpoint"),
            transport: TransportType::Http,
            address: defaults::network::LOCALHOST.to_string(),
            port: 9000,
            path: Some("/".to_string()),
            tls_enabled: false,
            health_status: status.clone(),
        }],
        metadata: HashMap::new(),
        health_status: status,
        last_seen: std::time::SystemTime::now(),
        capabilities: vec!["test".to_string()],
    }
}

fn create_test_message(source: &str, msg_type: &str) -> ProtocolMessage {
    ProtocolMessage {
        id: Uuid::new_v4(),
        message_type: Arc::from(msg_type),
        source: Arc::from(source),
        destination: None,
        payload: serde_json::json!({"test": "data"}),
        headers: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
        format: MessageFormat::Json,
        correlation_id: None,
        reply_to: None,
        ttl: Some(Duration::from_secs(60)),
        priority: MessagePriority::Normal,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_client_creation() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok(), "Failed to create protocol client");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let service = create_test_service("service-1", "test-service", HealthStatus::Healthy);

    let result = client.register_service(service.clone()).await;
    assert!(result.is_ok(), "Failed to register service");

    let health = client.get_service_health("service-1").await.unwrap();
    assert_eq!(health, HealthStatus::Healthy);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_services() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let service1 = create_test_service("service-1", "test-service", HealthStatus::Healthy);
    let service2 = create_test_service("service-2", "test-service", HealthStatus::Degraded);

    assert!(client.register_service(service1).await.is_ok());
    assert!(client.register_service(service2).await.is_ok());

    assert_eq!(
        client.get_service_health("service-1").await.unwrap(),
        HealthStatus::Healthy
    );
    assert_eq!(
        client.get_service_health("service-2").await.unwrap(),
        HealthStatus::Degraded
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_services_from_cache() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let service = create_test_service("service-1", "test-service", HealthStatus::Healthy);
    client.register_service(service).await.unwrap();

    let discovered = client.discover_services("test-service").await.unwrap();
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].id.as_ref(), "service-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_services_empty() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let discovered = client
        .discover_services("nonexistent-service")
        .await
        .unwrap();
    assert_eq!(discovered.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_service_health_unknown() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let health = client.get_service_health("nonexistent").await.unwrap();
    assert_eq!(health, HealthStatus::Unknown);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_handler() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let handler = SimpleMessageHandler::new(|msg| {
        Ok(Some(ProtocolMessage {
            id: Uuid::new_v4(),
            message_type: Arc::from("response"),
            source: Arc::from("handler"),
            destination: Some(msg.source.clone()),
            payload: msg.payload.clone(),
            headers: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
            format: MessageFormat::Json,
            correlation_id: Some(msg.id),
            reply_to: None,
            ttl: None,
            priority: MessagePriority::Normal,
        }))
    });

    client
        .register_handler("test-message", Box::new(handler))
        .await;

    let test_msg = create_test_message("test-source", "test-message");
    let response = client.handle_message(test_msg.clone()).await.unwrap();

    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(&*response.message_type, "response");
    assert_eq!(response.correlation_id, Some(test_msg.id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_handle_message_no_handler() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let test_msg = create_test_message("test-source", "unknown-type");
    let response = client.handle_message(test_msg).await.unwrap();

    assert!(response.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_select_service_healthy_preferred() {
    let config = create_test_config();
    let _client = ProtocolClient::new(config).await.unwrap();

    let services = vec![
        create_test_service("service-1", "test", HealthStatus::Degraded),
        create_test_service("service-2", "test", HealthStatus::Healthy),
        create_test_service("service-3", "test", HealthStatus::Unhealthy),
    ];

    let selected = routing::select_service(&services, &RoutingStrategy::RoundRobin).unwrap();
    assert_eq!(
        selected.id.as_ref(),
        "service-2",
        "Should select healthy service"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_select_service_no_healthy() {
    let config = create_test_config();
    let _client = ProtocolClient::new(config).await.unwrap();

    let services = vec![
        create_test_service("service-1", "test", HealthStatus::Degraded),
        create_test_service("service-2", "test", HealthStatus::Unhealthy),
    ];

    let selected = routing::select_service(&services, &RoutingStrategy::RoundRobin).unwrap();
    assert_eq!(selected.id.as_ref(), "service-1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_select_service_empty() {
    let services: Vec<ServiceInfo> = vec![];
    let result = routing::select_service(&services, &RoutingStrategy::RoundRobin);

    assert!(result.is_err());
    match result.unwrap_err() {
        ProtocolError::Routing(msg) => assert!(msg.contains("No services available")),
        _ => panic!("Expected Routing error"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_event_subscription() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut event_receiver = client.subscribe_events();

    let service = create_test_service("service-1", "test", HealthStatus::Healthy);
    client.register_service(service.clone()).await.unwrap();

    let event = event_receiver.try_recv();
    assert!(event.is_ok(), "Should receive event");

    match event.unwrap() {
        ProtocolEvent::ServiceRegistered { service: s } => {
            assert_eq!(s.id.as_ref(), "service-1");
        }
        _ => panic!("Expected ServiceRegistered event"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_message_priority_ordering() {
    let priorities = [
        MessagePriority::Low,
        MessagePriority::Normal,
        MessagePriority::High,
        MessagePriority::Critical,
        MessagePriority::Emergency,
    ];

    for i in 0..priorities.len() - 1 {
        assert!(
            priorities[i] < priorities[i + 1],
            "Priority ordering incorrect"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_status_equality() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_eq!(HealthStatus::Degraded, HealthStatus::Degraded);
    assert_eq!(HealthStatus::Unhealthy, HealthStatus::Unhealthy);
    assert_eq!(HealthStatus::Unknown, HealthStatus::Unknown);

    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Unknown);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_message_creation() {
    let msg = create_test_message("test-source", "test-type");

    assert_eq!(&*msg.source, "test-source");
    assert_eq!(&*msg.message_type, "test-type");
    assert_eq!(msg.format, MessageFormat::Json);
    assert_eq!(msg.priority, MessagePriority::Normal);
    assert!(msg.ttl.is_some());
    assert_eq!(msg.ttl.unwrap(), Duration::from_secs(60));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_message_sets_fields() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let msg = client.create_message("test-type", serde_json::json!({"key": "value"}));

    assert_eq!(&*msg.message_type, "test-type");
    assert_eq!(msg.payload, serde_json::json!({"key": "value"}));
    assert!(msg.destination.is_none());
    assert_eq!(msg.format, MessageFormat::Json);
    assert_eq!(msg.priority, MessagePriority::Normal);
    assert!(msg.ttl.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_connection_pool_config_defaults() {
    let pool = crate::config::ConnectionPoolConfig::default();
    assert_eq!(pool.max_connections_per_service, 10);
    assert_eq!(pool.idle_timeout, Duration::from_secs(300));
    assert_eq!(pool.keep_alive_interval, Duration::from_secs(30));
    assert_eq!(pool.max_concurrent_requests, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_config_default_service_id_format() {
    let config = ProtocolConfig::default();
    assert!(config.service_id.starts_with("toadstool-"));
    assert!(config.service_id.len() > 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_simple_message_handler() {
    let handler = SimpleMessageHandler::new(|msg| {
        Ok(Some(ProtocolMessage {
            id: Uuid::new_v4(),
            message_type: Arc::from("echo"),
            source: Arc::from("handler"),
            destination: Some(msg.source.clone()),
            payload: msg.payload.clone(),
            headers: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
            format: msg.format.clone(),
            correlation_id: Some(msg.id),
            reply_to: None,
            ttl: None,
            priority: msg.priority,
        }))
    });

    let test_msg = create_test_message("test", "test-type");
    let msg_id = test_msg.id;
    let payload = test_msg.payload.clone();

    let response = handler.handle_message(test_msg).unwrap();
    assert!(response.is_some());

    let response = response.unwrap();
    assert_eq!(&*response.message_type, "echo");
    assert_eq!(response.payload, payload);
    assert_eq!(response.correlation_id, Some(msg_id));
}
