// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol client implementation for service communication
//!
//! Domain modules:
//! - `discovery` — Service discovery and registration
//! - `health` — Health monitoring for registered services
//! - `routing` — Service and endpoint selection
//! - `handler` — Simple message handler implementation

mod discovery;
mod handler;
mod health;
mod routing;

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};
use tracing::{debug, info};

use crate::config::ProtocolConfig;
use crate::transport::TransportManager;
use crate::types::{
    HealthStatus, MessageHandler, MessagePriority, ProtocolError, ProtocolEvent, ProtocolMessage,
    ProtocolResult, ServiceInfo,
};

pub use handler::SimpleMessageHandler;

// =============================================================================
// Client struct and connection management
// =============================================================================

/// Main protocol client for service communication
pub struct ProtocolClient {
    config: ProtocolConfig,
    transport_manager: Arc<TransportManager>,
    /// Keyed by service id (Arc<str> = zero-copy clone)
    services: Arc<RwLock<HashMap<Arc<str>, ServiceInfo>>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler>>>>,
    event_bus: broadcast::Sender<ProtocolEvent>,
}

impl ProtocolClient {
    /// Create new protocol client with configuration
    pub async fn new(config: ProtocolConfig) -> ProtocolResult<Self> {
        let transport_manager = Arc::new(TransportManager::new());
        let (event_bus, _) = broadcast::channel(1000);

        let client = Self {
            config,
            transport_manager,
            services: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        };

        client.start_background_tasks().await;

        Ok(client)
    }

    // =========================================================================
    // Service discovery and registration
    // =========================================================================

    /// Register a service with the protocol client
    pub async fn register_service(&self, service_info: ServiceInfo) -> ProtocolResult<()> {
        let service_id = Arc::clone(&service_info.id);
        info!("Registering service: {}", service_id);

        {
            let mut services = self.services.write().await;
            match services.entry(Arc::clone(&service_id)) {
                std::collections::hash_map::Entry::Occupied(mut o) => {
                    *o.get_mut() = service_info;
                }
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(service_info);
                }
            }
        }

        let service_for_events = self
            .services
            .read()
            .await
            .get(&service_id)
            .cloned()
            .unwrap();

        if let Some(ref discovery_config) = self.config.discovery_config
            && discovery_config.auto_register
        {
            discovery::register_with_discovery(&service_for_events, discovery_config).await?;
        }

        if let Err(e) = self.event_bus.send(ProtocolEvent::ServiceRegistered {
            service: service_for_events,
        }) {
            tracing::debug!("Event bus send failed (no listeners): {e}");
        }

        info!("Successfully registered service: {}", service_id);
        Ok(())
    }

    /// Discover services by name
    pub async fn discover_services(&self, service_name: &str) -> ProtocolResult<Vec<ServiceInfo>> {
        info!("Discovering services: {}", service_name);

        {
            let cached_services: Vec<ServiceInfo> = self
                .services
                .read()
                .await
                .values()
                .filter(|s| s.name.as_ref() == service_name)
                .cloned()
                .collect();

            if !cached_services.is_empty() {
                debug!(
                    "Found {} cached services for: {}",
                    cached_services.len(),
                    service_name
                );
                return Ok(cached_services);
            }
        }

        if let Some(ref discovery_config) = self.config.discovery_config {
            let discovered_services =
                discovery::discover_from_registry(service_name, discovery_config).await?;

            {
                let mut services = self.services.write().await;
                for service in &discovered_services {
                    services
                        .entry(Arc::clone(&service.id))
                        .or_insert_with(|| service.clone());
                }
            }

            return Ok(discovered_services);
        }

        Ok(Vec::new())
    }

    // =========================================================================
    // Protocol message handling
    // =========================================================================

    /// Create a new protocol message
    pub fn create_message(
        &self,
        message_type: &str,
        payload: serde_json::Value,
    ) -> ProtocolMessage {
        ProtocolMessage {
            id: uuid::Uuid::new_v4(),
            message_type: Arc::from(message_type),
            source: self.config.service_id.clone(),
            destination: None,
            payload,
            headers: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
            format: self.config.default_format.clone(),
            correlation_id: None,
            reply_to: None,
            ttl: Some(self.config.request_timeout),
            priority: MessagePriority::Normal,
        }
    }

    /// Send message to a service
    pub async fn send_message(
        &self,
        destination: &str,
        mut message: ProtocolMessage,
    ) -> ProtocolResult<ProtocolMessage> {
        info!(
            "Sending message to: {} (type: {})",
            destination, message.message_type
        );

        message.destination = Some(Arc::from(destination));

        let services = self.discover_services(destination).await?;
        if services.is_empty() {
            return Err(ProtocolError::Discovery(format!(
                "No services found for: {destination}"
            )));
        }

        let service =
            routing::select_service(&services, &self.config.routing_config.default_strategy)?;
        let supported_transports = self.transport_manager.get_supported_transports();
        let endpoint = routing::select_endpoint(service, &supported_transports)?;

        let response = self
            .transport_manager
            .send_message(&message, endpoint)
            .await?;

        if let Err(e) = self.event_bus.send(ProtocolEvent::MessageSent {
            message_id: message.id,
            destination: destination.to_owned(),
        }) {
            tracing::debug!("Event bus send failed (no listeners): {e}");
        }

        if let Err(e) = self.event_bus.send(ProtocolEvent::MessageReceived {
            message_id: response.id,
            source: response.source.to_string(),
        }) {
            tracing::debug!("Event bus send failed (no listeners): {e}");
        }

        info!("Successfully sent message to: {}", destination);
        Ok(response)
    }

    /// Register message handler for a specific message type
    pub async fn register_handler(&self, message_type: &str, handler: Box<dyn MessageHandler>) {
        self.message_handlers
            .write()
            .await
            .insert(message_type.to_string(), handler);
        debug!("Registered handler for message type: {}", message_type);
    }

    /// Handle incoming message
    pub async fn handle_message(
        &self,
        message: ProtocolMessage,
    ) -> ProtocolResult<Option<ProtocolMessage>> {
        debug!(
            "Handling message: {} (type: {})",
            message.id, message.message_type
        );

        {
            let handlers = self.message_handlers.read().await;
            if let Some(handler) = handlers.get(&*message.message_type) {
                let result = handler.handle_message(message.clone())?;

                if let Err(e) = self.event_bus.send(ProtocolEvent::MessageReceived {
                    message_id: message.id,
                    source: message.source.to_string(),
                }) {
                    tracing::debug!("Event bus send failed (no listeners): {e}");
                }

                return Ok(result);
            }
        }

        debug!(
            "No handler found for message type: {}",
            message.message_type
        );
        Ok(None)
    }

    // =========================================================================
    // Health checking
    // =========================================================================

    /// Get service health status
    pub async fn get_service_health(&self, service_id: &str) -> ProtocolResult<HealthStatus> {
        self.services
            .read()
            .await
            .get(service_id)
            .map_or(Ok(HealthStatus::Unknown), |service| {
                Ok(service.health_status.clone())
            })
    }

    /// Subscribe to protocol events
    pub fn subscribe_events(&self) -> broadcast::Receiver<ProtocolEvent> {
        self.event_bus.subscribe()
    }

    // =========================================================================
    // Background tasks
    // =========================================================================

    async fn start_background_tasks(&self) {
        if self.config.health_config.base.enabled {
            health::spawn_health_monitor(
                Arc::clone(&self.services),
                self.config.health_config.clone(),
                self.event_bus.clone(),
            );
        }

        if let Some(ref discovery_config) = self.config.discovery_config {
            let refresh_interval = discovery_config.refresh_interval;

            tokio::spawn(async move {
                let mut interval_timer = tokio::time::interval(refresh_interval);

                loop {
                    interval_timer.tick().await;
                    debug!("Refreshing service discovery");
                }
            });
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProtocolConfig, RoutingStrategy};
    use crate::types::{MessageFormat, ServiceEndpoint, TransportType};
    use std::time::Duration;
    use toadstool_config::defaults;
    use uuid::Uuid;

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

    fn create_test_service(id: &str, name: &str, status: HealthStatus) -> ServiceInfo {
        ServiceInfo {
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
            message_type: msg_type.to_string(),
            source: source.to_string(),
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
                message_type: "response".to_string(),
                source: "handler".to_string(),
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
        assert_eq!(response.message_type, "response");
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

        assert_eq!(msg.source, "test-source");
        assert_eq!(msg.message_type, "test-type");
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

        assert_eq!(msg.message_type, "test-type");
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
                message_type: "echo".to_string(),
                source: "handler".to_string(),
                destination: Some(msg.source.clone()),
                payload: msg.payload.clone(),
                headers: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
                format: msg.format.clone(),
                correlation_id: Some(msg.id),
                reply_to: None,
                ttl: None,
                priority: msg.priority.clone(),
            }))
        });

        let test_msg = create_test_message("test", "test-type");
        let msg_id = test_msg.id;
        let payload = test_msg.payload.clone();

        let response = handler.handle_message(test_msg).unwrap();
        assert!(response.is_some());

        let response = response.unwrap();
        assert_eq!(response.message_type, "echo");
        assert_eq!(response.payload, payload);
        assert_eq!(response.correlation_id, Some(msg_id));
    }
}
