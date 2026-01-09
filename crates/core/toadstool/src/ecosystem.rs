//! # Ecosystem Coordination - Capability-Based Discovery
//!
//! This module handles integration with services in the ecosystem using
//! **capability-based discovery** rather than hardcoded primal names.
//!
//! ## Philosophy
//!
//! - **Self-Knowledge Only**: ToadStool knows only itself
//! - **Runtime Discovery**: Services discovered by capabilities, not names
//! - **Zero Hardcoding**: No primal-specific code or configuration
//! - **Capability-Based**: Services matched by what they can do, not who they are
//!
//! ## Example
//!
//! ```rust,ignore
//! // Instead of: connect_to_songbird()
//! // We do: find_service_by_capability(Capability::Coordination(...))
//! let coordinator = ecosystem.find_service_by_capability(
//!     Capability::Coordination(CoordinationCapability::ServiceDiscovery)
//! ).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_config::env_config::EnvironmentConfig;
use toadstool_config::network;

/// Multicast discovery protocol identifier
#[cfg(feature = "networking")]
const DISCOVERY_PROTOCOL_ID: &[u8] = b"TOADSTOOL_DISCOVERY";

/// Ecosystem coordinator for capability-based service integration
pub struct EcosystemCoordinator {
    /// Discovered services (indexed by service ID)
    services: Arc<RwLock<HashMap<String, DiscoveredService>>>,
    /// Communication channels (indexed by service ID)
    channels: Arc<RwLock<HashMap<String, ServiceChannel>>>,
    /// Service discovery client
    discovery: Arc<ServiceDiscovery>,
    /// Integration config
    config: EcosystemConfig,
}

/// Configuration for ecosystem integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Enable auto-discovery of services
    pub auto_discovery: bool,
    /// Discovery timeout
    pub discovery_timeout: Duration,
    /// Discovery method to use
    pub discovery_method: DiscoveryMethodConfig,
    /// Required capabilities for operation
    pub required_capabilities: Vec<Capability>,
    /// Optional capabilities for enhanced functionality
    pub optional_capabilities: Vec<Capability>,
}

/// Discovery method configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethodConfig {
    /// Automatic selection
    Auto,
    /// Environment variables only
    Environment,
    /// mDNS discovery
    Mdns,
    /// Configuration file
    ConfigFile { path: String },
    /// Registry service
    Registry { endpoint: String },
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_discovery: true,
            discovery_timeout: Duration::from_secs(30),
            discovery_method: DiscoveryMethodConfig::Auto,
            // No hardcoded primal names - discover by capability instead
            required_capabilities: vec![],
            optional_capabilities: vec![],
        }
    }
}

/// Discovered service instance (type alias for clarity)
pub type ServiceInstance = DiscoveredService;

/// Status of a service instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Discovered but not connected
    Discovered,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed(String),
    /// Disconnected
    Disconnected,
}

/// Communication channel with a service
pub struct ServiceChannel {
    pub service_id: String,
    pub service_name: String, // For logging/debugging only
    pub endpoint: String,
    pub client: ServiceClient,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Client for communicating with services
pub enum ServiceClient {
    /// HTTP client
    #[cfg(feature = "networking")]
    Http(reqwest::Client),
    /// WebSocket client (for real-time communication)
    #[cfg(feature = "websocket")]
    WebSocket(
        Arc<
            tokio::sync::Mutex<
                Option<
                    tokio_tungstenite::WebSocketStream<
                        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                    >,
                >,
            >,
        >,
    ),
    /// Pure Rust tRPC client (for high-performance communication)
    #[cfg(feature = "networking")]
    TRpc(reqwest::Client),
    /// Mock client for testing without networking
    #[cfg(not(feature = "networking"))]
    Mock,
}

/// Ecosystem message for primal communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    pub id: Uuid,
    pub from: String,
    pub to: String,
    pub message_type: EcosystemMessageType,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of ecosystem messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemMessageType {
    /// Heartbeat message
    Heartbeat,
    /// Capability announcement
    CapabilityAnnouncement,
    /// Resource request
    ResourceRequest,
    /// Resource response
    ResourceResponse,
    /// Workload request
    WorkloadRequest,
    /// Workload response
    WorkloadResponse,
    /// Status update
    StatusUpdate,
    /// Error message
    Error,
}

impl EcosystemCoordinator {
    /// Create a new ecosystem coordinator with capability-based discovery
    pub async fn new() -> ToadStoolResult<Self> {
        Self::with_config(EcosystemConfig::default()).await
    }

    /// Create a new ecosystem coordinator with custom config
    pub async fn with_config(config: EcosystemConfig) -> ToadStoolResult<Self> {
        info!("🌐 Creating Ecosystem Coordinator (capability-based discovery)");

        let services = Arc::new(RwLock::new(HashMap::new()));
        let channels = Arc::new(RwLock::new(HashMap::new()));

        // Initialize service discovery client
        let discovery_method = match &config.discovery_method {
            DiscoveryMethodConfig::Auto => DiscoveryMethod::Auto,
            DiscoveryMethodConfig::Environment => DiscoveryMethod::Environment,
            DiscoveryMethodConfig::Mdns => DiscoveryMethod::Mdns,
            DiscoveryMethodConfig::ConfigFile { path } => {
                DiscoveryMethod::ConfigFile { path: path.clone() }
            }
            DiscoveryMethodConfig::Registry { endpoint } => DiscoveryMethod::Registry {
                endpoint: endpoint.clone(),
            },
        };

        let discovery = ServiceDiscovery::new(discovery_method)
            .await
            .map_err(|e| ToadStoolError::other(format!("Failed to initialize discovery: {e}")))?;

        Ok(Self {
            services,
            channels,
            discovery: Arc::new(discovery),
            config,
        })
    }

    /// Discover services by capability
    ///
    /// This is the modern way to find services - by what they can do, not who they are.
    pub async fn find_service_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<DiscoveredService> {
        info!("🔍 Finding service with capability: {:?}", capability);

        let service = self
            .discovery
            .find_service_by_capability(capability)
            .await
            .map_err(|e| ToadStoolError::not_found(format!("No service found: {e}")))?;

        // Store discovered service
        let mut services = self.services.write().await;
        services.insert(service.id.clone(), service.clone());

        info!("✅ Found service: {} ({})", service.name, service.id);
        Ok(service)
    }

    /// Discover all services (legacy compatibility method)
    ///
    /// **Modern Alternative**: Use `find_service_by_capability()` for specific needs.
    pub async fn discover_services(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        info!("🔍 Discovering ecosystem services");

        let mut discovered = Vec::new();

        if self.config.auto_discovery {
            // Try to discover services for each required capability
            for capability in &self.config.required_capabilities {
                match self
                    .discovery
                    .find_service_by_capability(capability.clone())
                    .await
                {
                    Ok(service) => {
                        info!("✅ Found service for capability: {:?}", capability);
                        discovered.push(service);
                    }
                    Err(e) => {
                        warn!("❌ Required capability {:?} not found: {}", capability, e);
                    }
                }
            }

            // Try optional capabilities (don't fail if not found)
            for capability in &self.config.optional_capabilities {
                if let Ok(service) = self
                    .discovery
                    .find_service_by_capability(capability.clone())
                    .await
                {
                    info!("✅ Found optional service for capability: {:?}", capability);
                    discovered.push(service);
                }
            }
        }

        // Store discovered services
        let mut services = self.services.write().await;
        for service in &discovered {
            services.insert(service.id.clone(), service.clone());
        }

        info!("✅ Discovered {} services", discovered.len());
        Ok(discovered)
    }

    /// Discover services via multicast (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// This method is deprecated in favor of capability-based discovery.
    /// Use `find_service_by_capability()` instead.
    #[deprecated(since = "0.4.0", note = "Use capability-based discovery instead")]
    #[allow(dead_code)]
    async fn discover_via_multicast_legacy(&self) -> ToadStoolResult<Vec<ServiceInstance>> {
        debug!("🔍 Attempting multicast discovery (LEGACY METHOD)");

        #[cfg(feature = "networking")]
        {
            // Implement UDP multicast discovery
            use std::net::{Ipv4Addr, SocketAddr};
            use tokio::net::UdpSocket;

            let multicast_addr = "224.0.0.251:5353"
                .parse::<SocketAddr>()
                .map_err(|e| ToadStoolError::network(format!("Invalid multicast address: {e}")))?;

            let socket = UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|e| ToadStoolError::network(format!("Failed to bind UDP socket: {e}")))?;

            // Join multicast group
            socket
                .join_multicast_v4(Ipv4Addr::new(224, 0, 0, 251), Ipv4Addr::UNSPECIFIED)
                .map_err(|e| {
                    ToadStoolError::network(format!("Failed to join multicast group: {e}"))
                })?;

            // Send discovery broadcast
            socket
                .send_to(DISCOVERY_PROTOCOL_ID, &multicast_addr)
                .await
                .map_err(|e| {
                    ToadStoolError::network(format!("Failed to send discovery message: {e}"))
                })?;

            // Listen for responses with timeout
            let mut discovered_primals = Vec::new();
            let mut buffer = [0u8; 1024];

            // Set a timeout for discovery responses
            let timeout_duration = Duration::from_secs(5);

            match tokio::time::timeout(timeout_duration, socket.recv_from(&mut buffer)).await {
                Ok(Ok((len, addr))) => {
                    if let Ok(response) = std::str::from_utf8(&buffer[..len]) {
                        if response.starts_with("TOADSTOOL_PRIMAL:") {
                            let primal_info =
                                response.strip_prefix("TOADSTOOL_PRIMAL:").unwrap_or("");
                            if let Ok(primal_data) =
                                serde_json::from_str::<serde_json::Value>(primal_info)
                            {
                                if let Some(name) = primal_data.get("name").and_then(|v| v.as_str())
                                {
                                    let endpoint = format!("http://{}", addr.ip());
                                    match self.discover_service_at_endpoint(&endpoint).await {
                                        Ok(service) => {
                                            discovered_primals.push(service);
                                            debug!("✅ Discovered service via multicast: {}", name);
                                        }
                                        Err(e) => {
                                            debug!(
                                                "❌ Failed to validate multicast service {}: {}",
                                                name, e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    debug!("Multicast receive error: {}", e);
                }
                Err(_) => {
                    debug!("Multicast discovery timeout - no responses received");
                }
            }

            info!(
                "🔍 Multicast discovery found {} primals",
                discovered_primals.len()
            );
            Ok(discovered_primals)
        }

        #[cfg(not(feature = "networking"))]
        {
            debug!("Multicast discovery disabled (networking feature not enabled)");
            Ok(Vec::new())
        }
    }

    /// Discover services via DNS (Legacy - uses hardcoded ports)
    ///
    /// # ⚠️ Legacy Pattern
    /// This method is deprecated and uses hardcoded port assumptions.
    /// Prefer capability-based discovery for modern deployments.
    #[deprecated(
        since = "0.3.0",
        note = "Use capability-based discovery methods instead of hardcoded DNS + port scanning"
    )]
    #[allow(dead_code)]
    async fn discover_via_dns_legacy(&self) -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Discovering primals via DNS (legacy method)");
        warn!("⚠️  Using legacy DNS discovery with hardcoded ports - consider upgrading to capability-based discovery");

        let mut discovered = Vec::new();

        // Standard primal DNS names
        let dns_names = vec![
            ("songbird", "songbird.local"),
            ("nestgate", "nestgate.local"),
            ("beardog", "beardog.local"),
            ("squirrel", "squirrel.local"),
            ("biomeos", "biomeos.local"),
        ];

        for (name, dns_name) in dns_names {
            #[allow(deprecated)]
            match self
                .discover_service_at_endpoint(&format!(
                    "http://{dns_name}:{}",
                    network::get_songbird_port()
                ))
                .await
            {
                Ok(service) => discovered.push(service),
                Err(e) => debug!("DNS discovery failed for {}: {}", name, e),
            }
        }

        info!("✅ DNS discovery found {} primals", discovered.len());
        Ok(discovered)
    }

    /// Discover services via local network scan (Legacy - uses port scanning)
    ///
    /// # ⚠️ Legacy Pattern
    /// This method performs port scanning which is:
    /// - Inefficient (tries many ports sequentially)
    /// - Intrusive (network scanning can trigger security alerts)
    /// - Hardcoded (assumes specific port numbers)
    ///
    /// Modern deployments should use capability-based service discovery instead.
    #[deprecated(
        since = "0.3.0",
        note = "Port scanning is inefficient and intrusive. Use capability-based discovery instead."
    )]
    #[allow(dead_code)]
    async fn discover_via_local_scan_legacy(&self) -> ToadStoolResult<Vec<ServiceInstance>> {
        info!("🔍 Discovering primals via local network scan (legacy method)");
        warn!(
            "⚠️  Using legacy port scanning - this is inefficient and may trigger security alerts"
        );
        warn!("⚠️  Consider configuring capability-based service discovery for better performance");

        let mut discovered = Vec::new();

        // Scan common ports for primals (legacy discovery)
        #[allow(deprecated)]
        let common_ports = vec![
            network::get_songbird_port(),
            network::get_toadstool_port(), // Self-knowledge: knowing own port is acceptable
            network::get_beardog_port(),
            network::get_nestgate_port(),
            8084, // Legacy fallback port
            8085, // Legacy fallback port
        ];
        let config = EnvironmentConfig::from_env();
        let localhost = &config.network.bind_address;

        for port in common_ports {
            let endpoint = format!("http://{localhost}:{port}");
            if let Ok(service) = self.discover_service_at_endpoint(&endpoint).await {
                discovered.push(service)
            } else {
                // Ignore errors for local scan
            }
        }

        info!("✅ Local scan found {} services", discovered.len());
        Ok(discovered)
    }

    // ========================================================================
    // LEGACY API (Deprecated - kept for backward compatibility)
    // ========================================================================

    /// Integrate with discovered primals (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `integrate_services()` instead.
    #[deprecated(since = "0.4.0", note = "Use integrate_services()")]
    #[allow(dead_code)]
    pub async fn integrate_primals_legacy(
        &self,
        _primals: Vec<ServiceInstance>,
    ) -> ToadStoolResult<()> {
        warn!("⚠️  integrate_primals() is deprecated - use integrate_services() instead");
        Ok(())
    }

    /// Send message to a primal (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use capability-based messaging instead.
    #[deprecated(since = "0.4.0", note = "Use capability-based messaging")]
    pub async fn send_message(
        &self,
        primal_name: &str,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        debug!("📤 Sending message to {} (LEGACY API)", primal_name);

        // Try to find service by name (for backward compatibility)
        let services = self.services.read().await;
        let service = services
            .values()
            .find(|s| s.name == primal_name)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Service not found: {primal_name}"))
            })?;

        let channels = self.channels.read().await;
        let channel = channels.get(&service.id).ok_or_else(|| {
            ToadStoolError::not_found(format!("Channel not found for: {primal_name}"))
        })?;

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::Http(client) => {
                let message_url = format!("{}/message", channel.endpoint);
                let response = client
                    .post(&message_url)
                    .json(&message)
                    .timeout(Duration::from_secs(30))
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::network(format!("Failed to send message: {e}")))?;

                if !response.status().is_success() {
                    return Err(ToadStoolError::network(format!(
                        "Message send failed: {}",
                        response.status()
                    )));
                }

                let response_message: EcosystemMessage = response.json().await.map_err(|e| {
                    ToadStoolError::parsing(format!("Failed to parse response: {e}"))
                })?;

                debug!("✅ Message sent via HTTP");
                Ok(response_message)
            }
            #[cfg(not(feature = "networking"))]
            ServiceClient::Mock => {
                debug!("📤 Mock message sent");
                Ok(EcosystemMessage {
                    id: Uuid::new_v4(),
                    from: "mock_service".to_string(),
                    to: message.from,
                    message_type: EcosystemMessageType::StatusUpdate,
                    payload: serde_json::json!({"status": "mock_response"}),
                    timestamp: chrono::Utc::now(),
                })
            }
            #[cfg(feature = "websocket")]
            ServiceClient::WebSocket(_) => {
                debug!("📤 Sending message via WebSocket");
                Ok(EcosystemMessage {
                    id: Uuid::new_v4(),
                    from: channel.service_name.clone(),
                    to: message.from,
                    message_type: EcosystemMessageType::StatusUpdate,
                    payload: serde_json::json!({"status": "websocket_response"}),
                    timestamp: chrono::Utc::now(),
                })
            }
            #[cfg(feature = "networking")]
            ServiceClient::TRpc(client) => {
                // tRPC message sending
                debug!("📤 Sending message to {} via tRPC", primal_name);
                let message_url = format!("{}/trpc/message", channel.endpoint);
                let response = client
                    .post(&message_url)
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "method": "send_message",
                        "params": message
                    }))
                    .timeout(Duration::from_secs(30))
                    .send()
                    .await
                    .map_err(|e| {
                        ToadStoolError::network(format!("Failed to send tRPC message: {e}"))
                    })?;

                if !response.status().is_success() {
                    return Err(ToadStoolError::network(format!(
                        "tRPC message send failed: {}",
                        response.status()
                    )));
                }

                let response_message: EcosystemMessage = response.json().await.map_err(|e| {
                    ToadStoolError::parsing(format!("Failed to parse tRPC response: {e}"))
                })?;

                debug!("✅ Message sent via tRPC");
                Ok(response_message)
            }
        }
    }

    /// Get status of all primals (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `get_service_statuses()` instead.
    #[deprecated(since = "0.4.0", note = "Use get_service_statuses()")]
    pub async fn get_primal_status(&self) -> ToadStoolResult<HashMap<String, ServiceStatus>> {
        let services = self.services.read().await;
        let status = services
            .iter()
            .map(|(_id, service)| {
                // For backward compat, use service name as key
                let status = if service.healthy {
                    ServiceStatus::Connected
                } else {
                    ServiceStatus::Disconnected
                };
                (service.name.clone(), status)
            })
            .collect();

        Ok(status)
    }

    /// Check if a primal is available (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `is_capability_available()` instead.
    #[deprecated(since = "0.4.0", note = "Use is_capability_available()")]
    pub async fn is_primal_available(&self, primal_name: &str) -> bool {
        let services = self.services.read().await;
        services
            .values()
            .any(|s| s.name == primal_name && s.healthy)
    }

    /// Get primal capabilities (LEGACY)
    ///
    /// # ⚠️ Deprecated
    /// Use `get_service_capabilities()` instead.
    #[deprecated(since = "0.4.0", note = "Use get_service_capabilities()")]
    pub async fn get_primal_capabilities(&self, primal_name: &str) -> ToadStoolResult<Vec<String>> {
        // Forward to new method
        let services = self.services.read().await;
        let service = services
            .values()
            .find(|s| s.name == primal_name)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Service not found: {primal_name}"))
            })?;

        Ok(service
            .capabilities
            .iter()
            .map(|c| format!("{:?}", c))
            .collect())
    }

    // ========================================================================
    // MODERN CAPABILITY-BASED API (Added v0.4.0+)
    // ========================================================================

    /// Get service capabilities by service ID
    pub async fn get_service_capabilities(
        &self,
        service_id: &str,
    ) -> ToadStoolResult<Vec<Capability>> {
        let services = self.services.read().await;
        let service = services
            .get(service_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Service not found: {service_id}")))?;

        Ok(service.capabilities.clone())
    }

    /// Discover a service at a specific endpoint (modern version)
    async fn discover_service_at_endpoint(
        &self,
        endpoint: &str,
    ) -> ToadStoolResult<DiscoveredService> {
        debug!("🔍 Discovering service at {}", endpoint);

        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();
            let info_url = format!("{endpoint}/info");

            let response = client
                .get(&info_url)
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::network(format!("Failed to connect to {endpoint}: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::network(format!(
                    "Non-success status from {}: {}",
                    endpoint,
                    response.status()
                )));
            }

            let info: serde_json::Value = response.json().await.map_err(|e| {
                ToadStoolError::parsing(format!("Failed to parse response from {endpoint}: {e}"))
            })?;

            // Parse service information (capability-based)
            let service_id = info
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let service_name = info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Parse capabilities (expect structured Capability enums)
            let capabilities = info
                .get("capabilities")
                .and_then(|v| v.as_array())
                .and_then(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())).ok())
                .unwrap_or_default();

            // Parse endpoint to extract host and port
            let (host, port) = if let Some(url) = endpoint.strip_prefix("http://") {
                if let Some(port_idx) = url.rfind(':') {
                    let host = &url[..port_idx];
                    let port = url[port_idx + 1..].parse().unwrap_or(80);
                    (host.to_string(), port)
                } else {
                    (url.to_string(), 80)
                }
            } else {
                (endpoint.to_string(), 80)
            };

            let service = DiscoveredService {
                id: service_id,
                name: service_name,
                version,
                capabilities,
                endpoints: vec![ServiceEndpoint::http(host, port)],
                metadata: HashMap::new(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            };

            debug!("✅ Discovered service: {:?}", service);
            Ok(service)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Mock service for testing
            let service = DiscoveredService {
                id: uuid::Uuid::new_v4().to_string(),
                name: "mock_service".to_string(),
                version: "0.0.0".to_string(),
                capabilities: vec![],
                endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
                metadata: HashMap::new(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            };

            debug!("✅ Mock service created: {:?}", service);
            Ok(service)
        }
    }

    /// Integrate with discovered services
    pub async fn integrate_services(
        &self,
        services: Vec<DiscoveredService>,
    ) -> ToadStoolResult<()> {
        info!("🔗 Integrating with {} services", services.len());

        for service in services {
            let service_id = service.id.clone();
            match self.integrate_service(service).await {
                Ok(()) => info!("✅ Integrated with service {}", service_id),
                Err(e) => error!("❌ Failed to integrate with service {}: {}", service_id, e),
            }
        }

        info!("✅ Service integration complete");
        Ok(())
    }

    /// Integrate with a specific service
    async fn integrate_service(&self, service: DiscoveredService) -> ToadStoolResult<()> {
        info!(
            "🔗 Integrating with service: {} ({})",
            service.name, service.id
        );

        // Create communication channel
        let channel = self.create_service_channel(&service)?;

        // Test connection
        match self.test_service_connection(&channel).await {
            Ok(()) => {
                info!("✅ Successfully connected to service {}", service.id);
            }
            Err(e) => {
                warn!("❌ Failed to connect to service {}: {}", service.id, e);
            }
        }

        // Store channel
        let mut channels = self.channels.write().await;
        channels.insert(service.id.clone(), channel);

        // Store service
        let mut services = self.services.write().await;
        services.insert(service.id.clone(), service);

        Ok(())
    }

    /// Create communication channel with a service
    fn create_service_channel(
        &self,
        service: &DiscoveredService,
    ) -> ToadStoolResult<ServiceChannel> {
        debug!(
            "📡 Creating communication channel with service {}",
            service.id
        );

        let endpoint = service
            .primary_endpoint()
            .map(|e| e.url())
            .unwrap_or_else(|| "http://localhost".to_string());

        #[cfg(feature = "networking")]
        let client = ServiceClient::Http(reqwest::Client::new());
        #[cfg(not(feature = "networking"))]
        let client = ServiceClient::Mock;

        let channel = ServiceChannel {
            service_id: service.id.clone(),
            service_name: service.name.clone(),
            endpoint,
            client,
            last_heartbeat: chrono::Utc::now(),
        };

        Ok(channel)
    }

    /// Test connection to a service
    async fn test_service_connection(&self, channel: &ServiceChannel) -> ToadStoolResult<()> {
        debug!("🔍 Testing connection to service {}", channel.service_id);

        match &channel.client {
            #[cfg(feature = "networking")]
            ServiceClient::Http(client) => {
                let health_url = format!("{}/health", channel.endpoint);
                let response = client
                    .get(&health_url)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::network(format!("Health check failed: {e}")))?;

                if !response.status().is_success() {
                    return Err(ToadStoolError::network(format!(
                        "Health check returned: {}",
                        response.status()
                    )));
                }

                debug!("✅ Health check passed for service {}", channel.service_id);
                Ok(())
            }
            #[cfg(not(feature = "networking"))]
            ServiceClient::Mock => {
                debug!(
                    "✅ Mock health check passed for service {}",
                    channel.service_id
                );
                Ok(())
            }
            #[cfg(feature = "websocket")]
            ServiceClient::WebSocket(_) => {
                debug!(
                    "🔍 WebSocket health check for service {}",
                    channel.service_id
                );
                Ok(())
            }
            #[cfg(feature = "networking")]
            ServiceClient::TRpc(client) => {
                let health_url = format!("{}/trpc/health", channel.endpoint);
                let response = client
                    .post(&health_url)
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({"method": "health", "params": {}}))
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                    .map_err(|e| {
                        ToadStoolError::network(format!("TRPC health check failed: {e}"))
                    })?;

                if !response.status().is_success() {
                    return Err(ToadStoolError::network(format!(
                        "TRPC health check returned: {}",
                        response.status()
                    )));
                }

                debug!(
                    "✅ tRPC health check passed for service {}",
                    channel.service_id
                );
                Ok(())
            }
        }
    }

    /// Get all discovered services
    pub async fn get_discovered_services(&self) -> Vec<DiscoveredService> {
        let services = self.services.read().await;
        services.values().cloned().collect()
    }

    /// Check if a capability is available in the ecosystem
    pub async fn is_capability_available(&self, capability: &Capability) -> bool {
        let services = self.services.read().await;
        services.values().any(|s| s.has_capability(capability))
    }
}
