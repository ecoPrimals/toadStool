//! # Ecosystem Coordination
//!
//! This module handles integration with all primals in the ecosystem:
//! - 🎵 Songbird (Network Coordination)
//! - 🏠 `NestGate` (Storage)
//! - 🐕 `BearDog` (Security)
//! - 🐿️ Squirrel (AI)
//! - 🌱 biomeOS (Universal OS)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::env_config::EnvironmentConfig;
use toadstool_config::network;

/// Multicast discovery protocol identifier
#[cfg(feature = "networking")]
const DISCOVERY_PROTOCOL_ID: &[u8] = b"TOADSTOOL_DISCOVERY";

/// Ecosystem coordinator for primal integration
pub struct EcosystemCoordinator {
    /// Discovered primals
    primals: Arc<RwLock<HashMap<String, PrimalInstance>>>,
    /// Communication channels
    channels: Arc<RwLock<HashMap<String, PrimalChannel>>>,
    /// Integration config
    config: EcosystemConfig,
}

/// Configuration for ecosystem integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemConfig {
    /// Enable auto-discovery of primals
    pub auto_discovery: bool,
    /// Discovery timeout
    pub discovery_timeout: Duration,
    /// Primal endpoints (if not auto-discovered)
    pub primal_endpoints: HashMap<String, String>,
    /// Required primals for operation
    pub required_primals: Vec<String>,
    /// Optional primals for enhanced functionality
    pub optional_primals: Vec<String>,
}

impl Default for EcosystemConfig {
    fn default() -> Self {
        Self {
            auto_discovery: true,
            discovery_timeout: Duration::from_secs(30),
            primal_endpoints: HashMap::new(),
            required_primals: vec![],
            optional_primals: vec![
                "songbird".to_string(),
                "nestgate".to_string(),
                "beardog".to_string(),
                "squirrel".to_string(),
                "biomeos".to_string(),
            ],
        }
    }
}

/// Discovered primal instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalInstance {
    pub name: String,
    pub primal_type: PrimalType,
    pub endpoint: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub status: PrimalStatus,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
}

/// Types of primals in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalType {
    /// Songbird - Network coordination
    Songbird,
    /// `NestGate` - Storage
    NestGate,
    /// `BearDog` - Security
    BearDog,
    /// Squirrel - AI
    Squirrel,
    /// biomeOS - Universal OS
    BiomeOS,
    /// `ToadStool` - Compute (recursive)
    ToadStool,
    /// Custom primal
    Custom(String),
}

/// Status of a primal instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalStatus {
    /// Discovered but not connected
    Discovered,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed(String),
    /// Disconnected
    Disconnected,
}

/// Communication channel with a primal
pub struct PrimalChannel {
    pub primal_name: String,
    pub endpoint: String,
    pub client: PrimalClient,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Client for communicating with primals
pub enum PrimalClient {
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
    /// Create a new ecosystem coordinator
    pub fn new() -> ToadStoolResult<Self> {
        info!("🌐 Creating Ecosystem Coordinator");

        let primals = Arc::new(RwLock::new(HashMap::new()));
        let channels = Arc::new(RwLock::new(HashMap::new()));
        let config = EcosystemConfig::default();

        Ok(Self {
            primals,
            channels,
            config,
        })
    }

    /// Discover primals in the ecosystem
    pub async fn discover_primals(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
        info!("🔍 Discovering ecosystem primals");

        let mut discovered = Vec::new();

        if self.config.auto_discovery {
            // Auto-discover primals using various methods
            discovered.extend(self.discover_via_multicast().await?);
            discovered.extend(self.discover_via_dns().await?);
            discovered.extend(self.discover_via_local_scan().await?);
        }

        // Add configured endpoints
        for (name, endpoint) in &self.config.primal_endpoints {
            match self.discover_primal_at_endpoint(name, endpoint).await {
                Ok(primal) => discovered.push(primal),
                Err(e) => warn!("Failed to discover primal {} at {}: {}", name, endpoint, e),
            }
        }

        // Store discovered primals
        let mut primals = self.primals.write().await;
        for primal in &discovered {
            primals.insert(primal.name.clone(), primal.clone());
        }

        info!("✅ Discovered {} primals", discovered.len());
        Ok(discovered)
    }

    /// Discover primals via multicast
    async fn discover_via_multicast(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
        debug!("🔍 Attempting multicast discovery of primals");

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
                                    match self.discover_primal_at_endpoint(name, &endpoint).await {
                                        Ok(primal) => {
                                            discovered_primals.push(primal);
                                            debug!("✅ Discovered primal via multicast: {}", name);
                                        }
                                        Err(e) => {
                                            debug!(
                                                "❌ Failed to validate multicast primal {}: {}",
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

    /// Discover primals via DNS
    async fn discover_via_dns(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
        info!("🔍 Discovering primals via DNS");

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
            match self
                .discover_primal_at_endpoint(
                    name,
                    &format!("http://{dns_name}:{}", network::get_songbird_port()),
                )
                .await
            {
                Ok(primal) => discovered.push(primal),
                Err(e) => debug!("DNS discovery failed for {}: {}", name, e),
            }
        }

        info!("✅ DNS discovery found {} primals", discovered.len());
        Ok(discovered)
    }

    /// Discover primals via local network scan
    async fn discover_via_local_scan(&self) -> ToadStoolResult<Vec<PrimalInstance>> {
        info!("🔍 Discovering primals via local network scan");

        let mut discovered = Vec::new();

        // Scan common ports for primals
        let common_ports = vec![
            network::get_songbird_port(),
            network::get_toadstool_port(),
            network::get_beardog_port(),
            network::get_nestgate_port(),
            8084,
            8085,
        ];
        let config = EnvironmentConfig::from_env();
        let localhost = &config.network.bind_address;

        for port in common_ports {
            let endpoint = format!("http://{localhost}:{port}");
            if let Ok(primal) = self.discover_primal_at_endpoint("unknown", &endpoint).await {
                discovered.push(primal)
            } else {
                // Ignore errors for local scan
            }
        }

        info!("✅ Local scan found {} primals", discovered.len());
        Ok(discovered)
    }

    /// Discover a primal at a specific endpoint
    async fn discover_primal_at_endpoint(
        &self,
        name: &str,
        endpoint: &str,
    ) -> ToadStoolResult<PrimalInstance> {
        debug!("🔍 Discovering primal {} at {}", name, endpoint);

        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            // Try to get primal info
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

            // Parse primal information
            let primal_name = info
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(name)
                .to_string();

            let primal_type = info.get("type").and_then(|v| v.as_str()).map_or(
                PrimalType::Custom("unknown".to_string()),
                |t| match t {
                    "songbird" => PrimalType::Songbird,
                    "nestgate" => PrimalType::NestGate,
                    "beardog" => PrimalType::BearDog,
                    "squirrel" => PrimalType::Squirrel,
                    "biomeos" => PrimalType::BiomeOS,
                    "toadstool" => PrimalType::ToadStool,
                    other => PrimalType::Custom(other.to_string()),
                },
            );

            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let capabilities = info
                .get("capabilities")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(std::string::ToString::to_string)
                        .collect()
                })
                .unwrap_or_default();

            let primal = PrimalInstance {
                name: primal_name,
                primal_type,
                endpoint: endpoint.to_string(),
                version,
                capabilities,
                status: PrimalStatus::Discovered,
                discovered_at: chrono::Utc::now(),
            };

            debug!("✅ Discovered primal: {:?}", primal);
            Ok(primal)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Create a mock primal instance when networking is disabled
            let primal = PrimalInstance {
                name: name.to_string(),
                primal_type: PrimalType::Custom("mock".to_string()),
                endpoint: endpoint.to_string(),
                version: "mock".to_string(),
                capabilities: vec!["mock".to_string()],
                status: PrimalStatus::Discovered,
                discovered_at: chrono::Utc::now(),
            };

            debug!("✅ Mock primal created: {:?}", primal);
            Ok(primal)
        }
    }

    /// Integrate with discovered primals
    pub async fn integrate_primals(&self, primals: Vec<PrimalInstance>) -> ToadStoolResult<()> {
        info!("🔗 Integrating with {} primals", primals.len());

        for primal in primals {
            let primal_name = primal.name.clone();
            match self.integrate_primal(primal).await {
                Ok(()) => info!("✅ Integrated with {}", primal_name),
                Err(e) => error!("❌ Failed to integrate with {}: {}", primal_name, e),
            }
        }

        info!("✅ Primal integration complete");
        Ok(())
    }

    /// Integrate with a specific primal
    async fn integrate_primal(&self, mut primal: PrimalInstance) -> ToadStoolResult<()> {
        info!("🔗 Integrating with primal: {}", primal.name);

        // Create communication channel
        let channel = self.create_primal_channel(&primal)?;

        // Test connection
        match self.test_primal_connection(&channel).await {
            Ok(()) => {
                primal.status = PrimalStatus::Connected;
                info!("✅ Successfully connected to {}", primal.name);
            }
            Err(e) => {
                primal.status = PrimalStatus::Failed(e.to_string());
                warn!("❌ Failed to connect to {}: {}", primal.name, e);
            }
        }

        // Store channel
        let mut channels = self.channels.write().await;
        channels.insert(primal.name.clone(), channel);

        // Update primal status
        let mut primals = self.primals.write().await;
        primals.insert(primal.name.clone(), primal);

        Ok(())
    }

    /// Create communication channel with a primal
    fn create_primal_channel(&self, primal: &PrimalInstance) -> ToadStoolResult<PrimalChannel> {
        debug!("📡 Creating communication channel with {}", primal.name);

        #[cfg(feature = "networking")]
        let client = PrimalClient::Http(reqwest::Client::new());
        #[cfg(not(feature = "networking"))]
        let client = PrimalClient::Mock;

        let channel = PrimalChannel {
            primal_name: primal.name.clone(),
            endpoint: primal.endpoint.clone(),
            client,
            last_heartbeat: chrono::Utc::now(),
        };

        Ok(channel)
    }

    /// Test connection to a primal
    async fn test_primal_connection(&self, channel: &PrimalChannel) -> ToadStoolResult<()> {
        debug!("🔍 Testing connection to {}", channel.primal_name);

        match &channel.client {
            #[cfg(feature = "networking")]
            PrimalClient::Http(client) => {
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

                debug!("✅ Health check passed for {}", channel.primal_name);
                Ok(())
            }
            #[cfg(not(feature = "networking"))]
            PrimalClient::Mock => {
                debug!("✅ Mock health check passed for {}", channel.primal_name);
                Ok(())
            }
            #[cfg(feature = "websocket")]
            PrimalClient::WebSocket(_) => {
                // WebSocket health check - ping/pong
                debug!("🔍 WebSocket health check for {}", channel.primal_name);
                // For now, assume healthy if connection exists
                Ok(())
            }
            #[cfg(feature = "networking")]
            PrimalClient::TRpc(client) => {
                // tRPC health check using HTTP POST to /trpc/health
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

                debug!("✅ tRPC health check passed for {}", channel.primal_name);
                Ok(())
            }
        }
    }

    /// Send message to a primal
    pub async fn send_message(
        &self,
        primal_name: &str,
        message: EcosystemMessage,
    ) -> ToadStoolResult<EcosystemMessage> {
        debug!("📤 Sending message to {}", primal_name);

        let channels = self.channels.read().await;
        let channel = channels
            .get(primal_name)
            .ok_or_else(|| ToadStoolError::not_found(format!("Primal not found: {primal_name}")))?;

        match &channel.client {
            #[cfg(feature = "networking")]
            PrimalClient::Http(client) => {
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

                debug!("✅ Message sent to {} via HTTP", primal_name);
                Ok(response_message)
            }
            #[cfg(not(feature = "networking"))]
            PrimalClient::Mock => {
                debug!("📤 Mock message sent to {}", primal_name);
                // Return a mock response
                Ok(EcosystemMessage {
                    id: Uuid::new_v4(),
                    from: "mock_primal".to_string(),
                    to: message.from,
                    message_type: EcosystemMessageType::StatusUpdate,
                    payload: serde_json::json!({"status": "mock_response"}),
                    timestamp: chrono::Utc::now(),
                })
            }
            #[cfg(feature = "websocket")]
            PrimalClient::WebSocket(_) => {
                // WebSocket message sending
                debug!("📤 Sending message to {} via WebSocket", primal_name);
                // For now, return a placeholder response
                Ok(EcosystemMessage {
                    id: Uuid::new_v4(),
                    from: primal_name.to_string(),
                    to: message.from,
                    message_type: EcosystemMessageType::StatusUpdate,
                    payload: serde_json::json!({"status": "websocket_response"}),
                    timestamp: chrono::Utc::now(),
                })
            }
            #[cfg(feature = "networking")]
            PrimalClient::TRpc(client) => {
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

                debug!("✅ Message sent to {} via tRPC", primal_name);
                Ok(response_message)
            }
        }
    }

    /// Get status of all primals
    pub async fn get_primal_status(&self) -> ToadStoolResult<HashMap<String, PrimalStatus>> {
        let primals = self.primals.read().await;
        let status = primals
            .iter()
            .map(|(name, primal)| (name.clone(), primal.status.clone()))
            .collect();

        Ok(status)
    }

    /// Check if a primal is available
    pub async fn is_primal_available(&self, primal_name: &str) -> bool {
        let primals = self.primals.read().await;
        primals
            .get(primal_name)
            .is_some_and(|p| p.status == PrimalStatus::Connected)
    }

    /// Get primal capabilities
    pub async fn get_primal_capabilities(&self, primal_name: &str) -> ToadStoolResult<Vec<String>> {
        let primals = self.primals.read().await;
        let primal = primals
            .get(primal_name)
            .ok_or_else(|| ToadStoolError::not_found(format!("Primal not found: {primal_name}")))?;

        Ok(primal.capabilities.clone())
    }
}
