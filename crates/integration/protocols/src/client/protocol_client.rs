// SPDX-License-Identifier: AGPL-3.0-only
//! Core [`ProtocolClient`] type and connection lifecycle.

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

use super::discovery;
use super::health;
use super::routing;

type HandlerMap = HashMap<Arc<str>, Box<dyn MessageHandler>>;

/// Main protocol client for service communication
pub struct ProtocolClient {
    config: ProtocolConfig,
    transport_manager: Arc<TransportManager>,
    /// Keyed by service id (`Arc<str>` = zero-copy clone)
    services: Arc<RwLock<HashMap<Arc<str>, ServiceInfo>>>,
    message_handlers: Arc<RwLock<HandlerMap>>,
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
            .ok_or_else(|| {
                ProtocolError::Internal(format!("Service {service_id} missing after insert"))
            })?;

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
            .insert(Arc::from(message_type), handler);
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
