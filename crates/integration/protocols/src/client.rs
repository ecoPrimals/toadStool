//! Protocol client implementation for service communication

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

use crate::config::{DiscoveryConfig, ProtocolConfig, RoutingStrategy};
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
        if self.config.health_config.enabled {
            let _services = Arc::clone(&self.services);
            let interval = self.config.health_config.interval;
            let _event_bus = self.event_bus.clone();

            tokio::spawn(async move {
                let mut interval_timer = tokio::time::interval(interval);

                loop {
                    interval_timer.tick().await;

                    // Check health of all registered services
                    // Mock implementation for now
                    debug!("Health check cycle completed");
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
        _discovery_config: &DiscoveryConfig,
    ) -> ProtocolResult<()> {
        // Mock implementation - would register with actual discovery service
        debug!("Registering service with discovery: {}", service_info.id);
        Ok(())
    }

    /// Discover services from registry
    async fn discover_from_registry(
        &self,
        service_name: &str,
        _discovery_config: &DiscoveryConfig,
    ) -> ProtocolResult<Vec<ServiceInfo>> {
        // Mock implementation - would query actual discovery service
        debug!("Discovering services from registry: {}", service_name);
        Ok(Vec::new())
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
