//! Protocol client implementation for service communication

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

use crate::config::{ProtocolConfig, RoutingStrategy, ServiceDiscoveryConfig};
use crate::transport::{Connection, TransportManager};
use crate::types::{
    HealthStatus, MessageHandler, MessagePriority, ProtocolError, ProtocolEvent, ProtocolMessage,
    ProtocolResult, ServiceEndpoint, ServiceInfo,
};

/// Main protocol client for service communication
pub struct ProtocolClient {
    config: ProtocolConfig,
    transport_manager: Arc<TransportManager>,
    services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    #[allow(dead_code)]
    connections: Arc<RwLock<HashMap<String, Connection>>>,
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
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
        };

        // Start background tasks
        client.start_background_tasks().await;

        Ok(client)
    }

    /// Register a service with the protocol client
    pub async fn register_service(&self, service_info: ServiceInfo) -> ProtocolResult<()> {
        info!("Registering service: {}", service_info.id);

        // Store service information
        {
            let mut services = self.services.write().await;
            services.insert(service_info.id.clone(), service_info.clone());
        }

        // Register with discovery service if configured
        if let Some(ref discovery_config) = self.config.discovery_config {
            if discovery_config.auto_register {
                self.register_with_discovery(&service_info, discovery_config)
                    .await?;
            }
        }

        // Emit event
        let _ = self.event_bus.send(ProtocolEvent::ServiceRegistered {
            service: service_info.clone(),
        });

        info!("Successfully registered service: {}", service_info.id);
        Ok(())
    }

    /// Discover services by name
    pub async fn discover_services(&self, service_name: &str) -> ProtocolResult<Vec<ServiceInfo>> {
        info!("Discovering services: {}", service_name);

        // Check local cache first
        {
            let services = self.services.read().await;
            let cached_services: Vec<ServiceInfo> = services
                .values()
                .filter(|s| s.name == service_name)
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

        // Query discovery service if configured
        if let Some(ref discovery_config) = self.config.discovery_config {
            let discovered_services = self
                .discover_from_registry(service_name, discovery_config)
                .await?;

            // Cache discovered services
            {
                let mut services = self.services.write().await;
                for service in &discovered_services {
                    services.insert(service.id.clone(), service.clone());
                }
            }

            return Ok(discovered_services);
        }

        // Return empty list if no discovery configured
        Ok(Vec::new())
    }

    /// Create a new protocol message
    pub fn create_message(
        &self,
        message_type: &str,
        payload: serde_json::Value,
    ) -> ProtocolMessage {
        ProtocolMessage {
            id: uuid::Uuid::new_v4(),
            message_type: message_type.to_string(),
            source: self.config.service_id.clone(),
            destination: None,
            payload,
            headers: HashMap::new(),
            timestamp: Utc::now(),
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

        // Set destination
        message.destination = Some(destination.to_string());

        // Select target service
        let services = self.discover_services(destination).await?;
        if services.is_empty() {
            return Err(ProtocolError::Discovery(format!(
                "No services found for: {destination}"
            )));
        }

        let service = self.select_service(&services)?;
        let endpoint = self.select_endpoint(service)?;

        // Send message through transport
        let response = self
            .transport_manager
            .send_message(&message, endpoint)
            .await?;

        // Emit events
        let _ = self.event_bus.send(ProtocolEvent::MessageSent {
            message_id: message.id,
            destination: destination.to_string(),
        });

        let _ = self.event_bus.send(ProtocolEvent::MessageReceived {
            message_id: response.id,
            source: response.source.clone(),
        });

        info!("Successfully sent message to: {}", destination);
        Ok(response)
    }

    /// Register message handler for a specific message type
    pub async fn register_handler(&self, message_type: &str, handler: Box<dyn MessageHandler>) {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(message_type.to_string(), handler);
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

        let handlers = self.message_handlers.read().await;
        if let Some(handler) = handlers.get(&message.message_type) {
            let result = handler.handle_message(message.clone())?;

            // Emit event
            let _ = self.event_bus.send(ProtocolEvent::MessageReceived {
                message_id: message.id,
                source: message.source,
            });

            return Ok(result);
        }

        // No handler found
        debug!(
            "No handler found for message type: {}",
            message.message_type
        );
        Ok(None)
    }

    /// Get service health status
    pub async fn get_service_health(&self, service_id: &str) -> ProtocolResult<HealthStatus> {
        let services = self.services.read().await;
        if let Some(service) = services.get(service_id) {
            Ok(service.health_status.clone())
        } else {
            Ok(HealthStatus::Unknown)
        }
    }

    /// Subscribe to protocol events
    pub fn subscribe_events(&self) -> broadcast::Receiver<ProtocolEvent> {
        self.event_bus.subscribe()
    }

    /// Start background tasks for health monitoring and discovery
    async fn start_background_tasks(&self) {
        // Health monitoring task
        if self.config.health_config.base.enabled {
            let services_for_health = Arc::clone(&self.services);
            let interval = self.config.health_config.base.interval;
            let _event_bus = self.event_bus.clone();

            tokio::spawn(async move {
                let mut interval_timer = tokio::time::interval(interval);

                loop {
                    interval_timer.tick().await;

                    // Check health of all registered services
                    let services_snapshot = services_for_health.read().await;
                    for (service_id, service_info) in services_snapshot.iter() {
                        // Check each service endpoint
                        for endpoint in &service_info.endpoints {
                            // Simple connectivity check via HTTP HEAD request
                            // Build URL from endpoint address (assume HTTP for health checks)
                            let url = if endpoint.address.starts_with("http") {
                                format!("{}/health", endpoint.address)
                            } else {
                                format!("http://{}/health", endpoint.address)
                            };

                            match reqwest::Client::new()
                                .head(&url)
                                .timeout(std::time::Duration::from_secs(5))
                                .send()
                                .await
                            {
                                Ok(response) if response.status().is_success() => {
                                    debug!("Service {} is healthy at {}", service_id, url);
                                }
                                Ok(response) => {
                                    debug!(
                                        "Service {} returned status {} at {}",
                                        service_id,
                                        response.status(),
                                        url
                                    );
                                }
                                Err(e) => {
                                    debug!("Service {} health check failed: {}", service_id, e);
                                }
                            }
                        }
                    }
                    debug!(
                        "Health check cycle completed for {} services",
                        services_snapshot.len()
                    );
                }
            });
        }

        // Service discovery refresh task
        if let Some(ref discovery_config) = self.config.discovery_config {
            let refresh_interval = discovery_config.refresh_interval;
            let _services = Arc::clone(&self.services);

            tokio::spawn(async move {
                let mut interval_timer = tokio::time::interval(refresh_interval);

                loop {
                    interval_timer.tick().await;

                    // Refresh service discovery
                    // This would query the discovery service for updates
                    debug!("Refreshing service discovery");
                }
            });
        }
    }

    /// Register service with discovery service
    async fn register_with_discovery(
        &self,
        service_info: &ServiceInfo,
        discovery_config: &ServiceDiscoveryConfig,
    ) -> ProtocolResult<()> {
        // Use registry endpoint if available
        if let Some(ref endpoint) = discovery_config.registry_endpoint {
            let url = format!("{}/api/v1/services", endpoint);

            match reqwest::Client::new()
                .post(&url)
                .json(&service_info)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    info!(
                        "Successfully registered service {} with discovery at {}",
                        service_info.id, endpoint
                    );
                }
                Ok(response) => {
                    debug!(
                        "Service registration returned status {}: {} at {}",
                        response.status(),
                        service_info.id,
                        endpoint
                    );
                }
                Err(e) => {
                    debug!(
                        "Failed to register service {} with discovery at {}: {}",
                        service_info.id, endpoint, e
                    );
                }
            }
        } else {
            debug!("No registry endpoint configured, skipping registration");
        }

        Ok(())
    }

    /// Discover services from registry
    async fn discover_from_registry(
        &self,
        service_name: &str,
        discovery_config: &ServiceDiscoveryConfig,
    ) -> ProtocolResult<Vec<ServiceInfo>> {
        let mut discovered_services = Vec::new();

        // Query registry endpoint if available
        if let Some(ref endpoint) = discovery_config.registry_endpoint {
            let url = format!("{}/api/v1/services/{}", endpoint, service_name);

            match reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    match response.json::<Vec<ServiceInfo>>().await {
                        Ok(services) => {
                            info!(
                                "Discovered {} instances of service {} from {}",
                                services.len(),
                                service_name,
                                endpoint
                            );
                            discovered_services.extend(services);
                        }
                        Err(e) => {
                            debug!(
                                "Failed to parse discovery response from {}: {}",
                                endpoint, e
                            );
                        }
                    }
                }
                Ok(response) => {
                    debug!(
                        "Discovery query returned status {}: {} at {}",
                        response.status(),
                        service_name,
                        endpoint
                    );
                }
                Err(e) => {
                    debug!("Failed to query discovery service at {}: {}", endpoint, e);
                }
            }
        } else {
            debug!("No registry endpoint configured, returning empty results");
        }

        Ok(discovered_services)
    }

    /// Select service based on routing strategy
    fn select_service<'a>(&self, services: &'a [ServiceInfo]) -> ProtocolResult<&'a ServiceInfo> {
        if services.is_empty() {
            return Err(ProtocolError::Routing("No services available".to_string()));
        }

        // Simple selection based on routing strategy
        match self.config.routing_config.default_strategy {
            RoutingStrategy::RoundRobin | RoutingStrategy::Random => {
                // For now, just return the first healthy service
                services
                    .iter()
                    .find(|s| s.health_status == HealthStatus::Healthy)
                    .or_else(|| services.first())
                    .ok_or_else(|| {
                        ProtocolError::Routing("No healthy services available".to_string())
                    })
            }
            _ => {
                // Default to first service
                services
                    .first()
                    .ok_or_else(|| ProtocolError::Routing("No services available".to_string()))
            }
        }
    }

    /// Select endpoint from service
    fn select_endpoint<'a>(&self, service: &'a ServiceInfo) -> ProtocolResult<&'a ServiceEndpoint> {
        // Find first healthy endpoint with supported transport
        let supported_transports = self.transport_manager.get_supported_transports();

        service
            .endpoints
            .iter()
            .find(|e| {
                e.health_status == HealthStatus::Healthy
                    && supported_transports.contains(&e.transport)
            })
            .or_else(|| {
                // Fallback to first endpoint with supported transport
                service
                    .endpoints
                    .iter()
                    .find(|e| supported_transports.contains(&e.transport))
            })
            .ok_or_else(|| ProtocolError::Routing("No suitable endpoints available".to_string()))
    }
}

/// Simple message handler implementation
pub struct SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    handler_fn: F,
}

impl<F> SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    pub fn new(handler_fn: F) -> Self {
        Self { handler_fn }
    }
}

impl<F> MessageHandler for SimpleMessageHandler<F>
where
    F: Fn(ProtocolMessage) -> Result<Option<ProtocolMessage>, ProtocolError> + Send + Sync,
{
    fn handle_message(
        &self,
        message: ProtocolMessage,
    ) -> Result<Option<ProtocolMessage>, ProtocolError> {
        (self.handler_fn)(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProtocolConfig;
    use crate::types::{MessageFormat, TransportType};
    use std::time::Duration;
    use toadstool_config::defaults;
    use uuid::Uuid;

    /// Helper function to create a test protocol config
    fn create_test_config() -> ProtocolConfig {
        ProtocolConfig::default()
    }

    /// Helper function to create a test service info
    ///
    /// **Infant Discovery**: Uses test port, not hardcoded primal ports.
    /// In production, use ServiceDiscovery to find services by capability.
    fn create_test_service(id: &str, name: &str, status: HealthStatus) -> ServiceInfo {
        ServiceInfo {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            endpoints: vec![ServiceEndpoint {
                id: format!("{}-endpoint", id),
                transport: TransportType::Http,
                address: defaults::network::LOCALHOST.to_string(),
                port: 9000, // Test port (not a hardcoded primal port)
                path: Some("/".to_string()),
                tls_enabled: false,
                health_status: status.clone(),
            }],
            metadata: HashMap::new(),
            health_status: status,
            last_seen: Utc::now(),
            capabilities: vec!["test".to_string()],
        }
    }

    /// Helper function to create a test protocol message
    fn create_test_message(source: &str, msg_type: &str) -> ProtocolMessage {
        ProtocolMessage {
            id: Uuid::new_v4(),
            message_type: msg_type.to_string(),
            source: source.to_string(),
            destination: None,
            payload: serde_json::json!({"test": "data"}),
            headers: HashMap::new(),
            timestamp: Utc::now(),
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

        // Verify service is registered
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

        // Verify both services are registered
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

        // Discover should find the cached service
        let discovered = client.discover_services("test-service").await.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].id, "service-1");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_discover_services_empty() {
        let config = create_test_config();
        let client = ProtocolClient::new(config).await.unwrap();

        // No services registered, should return empty
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

        // Non-existent service should return Unknown
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
                timestamp: Utc::now(),
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

        // Verify handler is registered by handling a message
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

        // No handler registered, should return None
        assert!(response.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_select_service_healthy_preferred() {
        let config = create_test_config();
        let client = ProtocolClient::new(config).await.unwrap();

        let services = vec![
            create_test_service("service-1", "test", HealthStatus::Degraded),
            create_test_service("service-2", "test", HealthStatus::Healthy),
            create_test_service("service-3", "test", HealthStatus::Unhealthy),
        ];

        let selected = client.select_service(&services).unwrap();
        assert_eq!(selected.id, "service-2", "Should select healthy service");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_select_service_no_healthy() {
        let config = create_test_config();
        let client = ProtocolClient::new(config).await.unwrap();

        let services = vec![
            create_test_service("service-1", "test", HealthStatus::Degraded),
            create_test_service("service-2", "test", HealthStatus::Unhealthy),
        ];

        let selected = client.select_service(&services).unwrap();
        // Should fallback to first service
        assert_eq!(selected.id, "service-1");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_select_service_empty() {
        let config = create_test_config();
        let client = ProtocolClient::new(config).await.unwrap();

        let services: Vec<ServiceInfo> = vec![];
        let result = client.select_service(&services);

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

        // Should receive ServiceRegistered event
        let event = event_receiver.try_recv();
        assert!(event.is_ok(), "Should receive event");

        match event.unwrap() {
            ProtocolEvent::ServiceRegistered { service: s } => {
                assert_eq!(s.id, "service-1");
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

        // Verify priority ordering
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
    async fn test_simple_message_handler() {
        let handler = SimpleMessageHandler::new(|msg| {
            Ok(Some(ProtocolMessage {
                id: Uuid::new_v4(),
                message_type: "echo".to_string(),
                source: "handler".to_string(),
                destination: Some(msg.source.clone()),
                payload: msg.payload.clone(),
                headers: HashMap::new(),
                timestamp: Utc::now(),
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
