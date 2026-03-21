// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `ProtocolClient`

use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use toadstool_common::auth::ServiceAuthConfig;
use toadstool_integration_protocols::client::ProtocolClient;
use toadstool_integration_protocols::config::*;
use toadstool_integration_protocols::types::*;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_config() -> ProtocolConfig {
    ProtocolConfig {
        service_id: Arc::from("test-service"),
        default_format: MessageFormat::Json,
        supported_transports: vec![TransportType::Http],
        auth_config: None,
        request_timeout: Duration::from_secs(30),
        connection_pool: ConnectionPoolConfig::default(),
        discovery_config: None,
        routing_config: RoutingConfig::default(),
        // Disable background health probing — tests use fake endpoints that
        // fail TCP connection, which would race with service registration and
        // mark services unhealthy before get_service_health can be called.
        health_config: HealthConfig {
            base: toadstool_common::config_bases::HealthCheckConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        },
    }
}

fn create_test_service_info() -> ServiceInfo {
    ServiceInfo {
        id: Arc::from("test-service-1"),
        name: Arc::from("test-service"),
        version: "1.0.0".to_string(),
        endpoints: vec![ServiceEndpoint {
            id: "endpoint-1".to_string(),
            transport: TransportType::Http,
            address: "localhost".to_string(),
            port: 8080,
            path: Some("/api".to_string()),
            tls_enabled: false,
            health_status: HealthStatus::Healthy,
        }],
        health_status: HealthStatus::Healthy,
        metadata: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
        capabilities: vec!["compute".to_string()],
    }
}

// ============================================================================
// Client Creation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_client_creation() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await;

    assert!(client.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_client_with_custom_service_id() {
    let mut config = create_test_config();
    config.service_id = Arc::from("custom-service-id");

    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_client_with_multiple_transports() {
    let mut config = create_test_config();
    config.supported_transports = vec![
        TransportType::Http,
        TransportType::TRpc,
        TransportType::TRpc,
    ];

    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_protocol_client_with_auth_config() {
    let mut config = create_test_config();
    config.auth_config = Some(ServiceAuthConfig::bearer("test-token"));

    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok());
}

// ============================================================================
// Service Registration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();
    let service_info = create_test_service_info();

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_services() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let service1 = ServiceInfo {
        id: Arc::from("service-1"),
        name: Arc::from("test-service-1"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        health_status: HealthStatus::Healthy,
        metadata: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let service2 = ServiceInfo {
        id: Arc::from("service-2"),
        name: Arc::from("test-service-2"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        health_status: HealthStatus::Healthy,
        metadata: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let result1 = client.register_service(service1).await;
    let result2 = client.register_service(service2).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service_with_metadata() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("region".to_string(), "us-west-2".to_string());
    metadata.insert("zone".to_string(), "us-west-2a".to_string());

    let mut service_info = create_test_service_info();
    service_info.metadata = metadata;

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

// ============================================================================
// Service Discovery Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_services_no_discovery_config() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let services = client.discover_services("test-service").await;
    assert!(services.is_ok());
    assert!(services.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_services_from_cache() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();
    let service_info = create_test_service_info();

    // Register service first
    client.register_service(service_info.clone()).await.unwrap();

    // Discover should find it in cache
    let services = client.discover_services("test-service").await.unwrap();
    assert_eq!(services.len(), 1);
    assert_eq!(&*services[0].name, "test-service");
}

// ============================================================================
// Message Creation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_message() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let message = client.create_message("test-type", json!({"data": "test"}));

    assert_eq!(&*message.message_type, "test-type");
    assert_eq!(&*message.source, "test-service");
    assert_eq!(message.format, MessageFormat::Json);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_message_with_complex_payload() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let payload = json!({
        "operation": "compute",
        "parameters": {
            "cpu": 2,
            "memory": "4GB"
        },
        "metadata": {
            "priority": "high",
            "timeout": 30
        }
    });

    let message = client.create_message("compute-request", payload);
    assert_eq!(&*message.message_type, "compute-request");
    assert!(message.payload.is_object());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_message_has_unique_id() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let msg1 = client.create_message("type1", json!({}));
    let msg2 = client.create_message("type2", json!({}));

    assert_ne!(msg1.id, msg2.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_message_has_timestamp() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let before = std::time::SystemTime::now();
    let message = client.create_message("test", json!({}));
    let after = std::time::SystemTime::now();

    assert!(message.timestamp >= before && message.timestamp <= after);
}

// ============================================================================
// Service Health Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_service_health_unknown() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let health = client.get_service_health("non-existent").await.unwrap();
    assert!(matches!(health, HealthStatus::Unknown));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_service_health_after_registration() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();
    let service_info = create_test_service_info();

    client.register_service(service_info.clone()).await.unwrap();

    let health = client
        .get_service_health(service_info.id.as_ref())
        .await
        .unwrap();
    assert!(matches!(health, HealthStatus::Healthy));
}

// ============================================================================
// Event Subscription Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_subscribe_events() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let _receiver = client.subscribe_events();
    // Just verify we can get a receiver without panicking
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_event_subscriptions() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let _receiver1 = client.subscribe_events();
    let _receiver2 = client.subscribe_events();
    let _receiver3 = client.subscribe_events();
    // Verify multiple subscribers work
}

// ============================================================================
// Config Validation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_client_with_minimal_config() {
    let config = ProtocolConfig::default();
    let client = ProtocolClient::new(config).await;

    assert!(client.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_client_with_short_timeout() {
    let mut config = create_test_config();
    config.request_timeout = Duration::from_millis(100);

    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_client_with_long_timeout() {
    let mut config = create_test_config();
    config.request_timeout = Duration::from_secs(300);

    let client = ProtocolClient::new(config).await;
    assert!(client.is_ok());
}

// ============================================================================
// Message Format Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_client_with_messagepack_format() {
    let mut config = create_test_config();
    config.default_format = MessageFormat::MessagePack;

    let client = ProtocolClient::new(config).await.unwrap();
    let message = client.create_message("test", json!({}));

    assert_eq!(message.format, MessageFormat::MessagePack);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_client_with_cbor_format() {
    let mut config = create_test_config();
    config.default_format = MessageFormat::Cbor;

    let client = ProtocolClient::new(config).await.unwrap();
    let message = client.create_message("test", json!({}));

    assert_eq!(message.format, MessageFormat::Cbor);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service_with_empty_endpoints() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut service_info = create_test_service_info();
    service_info.endpoints = vec![];

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service_with_multiple_endpoints() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut service_info = create_test_service_info();
    service_info.endpoints = vec![
        ServiceEndpoint {
            id: "endpoint-1".to_string(),
            transport: TransportType::Http,
            address: "localhost".to_string(),
            port: 8080,
            path: Some("/api".to_string()),
            tls_enabled: false,
            health_status: HealthStatus::Healthy,
        },
        ServiceEndpoint {
            id: "endpoint-2".to_string(),
            transport: TransportType::TRpc,
            address: "localhost".to_string(),
            port: 8081,
            path: Some("/ws".to_string()),
            tls_enabled: false,
            health_status: HealthStatus::Healthy,
        },
    ];

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_message_with_empty_payload() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let message = client.create_message("empty", json!({}));
    assert_eq!(&*message.message_type, "empty");
    assert!(message.payload.is_object());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_message_with_array_payload() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let message = client.create_message("array-test", json!([1, 2, 3]));
    assert!(message.payload.is_array());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_services_with_different_names() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let service1 = ServiceInfo {
        id: Arc::from("service-1"),
        name: Arc::from("compute-service"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        health_status: HealthStatus::Healthy,
        metadata: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    let service2 = ServiceInfo {
        id: Arc::from("service-2"),
        name: Arc::from("storage-service"),
        version: "1.0.0".to_string(),
        endpoints: vec![],
        health_status: HealthStatus::Healthy,
        metadata: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
        capabilities: vec![],
    };

    client.register_service(service1).await.unwrap();
    client.register_service(service2).await.unwrap();

    let compute_services = client.discover_services("compute-service").await.unwrap();
    let storage_services = client.discover_services("storage-service").await.unwrap();

    assert_eq!(compute_services.len(), 1);
    assert_eq!(storage_services.len(), 1);
    assert_eq!(&*compute_services[0].name, "compute-service");
    assert_eq!(&*storage_services[0].name, "storage-service");
}

// ============================================================================
// Service Capabilities Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service_with_capabilities() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut service_info = create_test_service_info();
    service_info.capabilities = vec![
        "compute".to_string(),
        "storage".to_string(),
        "networking".to_string(),
    ];

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_service_with_no_capabilities() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let mut service_info = create_test_service_info();
    service_info.capabilities = vec![];

    let result = client.register_service(service_info).await;
    assert!(result.is_ok());
}

// ============================================================================
// Protocol Message Priority Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_message_default_priority() {
    let config = create_test_config();
    let client = ProtocolClient::new(config).await.unwrap();

    let message = client.create_message("test", json!({}));
    assert!(matches!(message.priority, MessagePriority::Normal));
}
