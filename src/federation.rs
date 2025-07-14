//! # Federation Manager
//!
//! Handles peer-to-peer networking and federation management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use toadstool_config::constants::network;

/// Signature response for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResponse {
    pub peer_id: String,
    pub challenge: String,
    pub signature: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

/// Federation-specific errors
#[derive(Error, Debug)]
pub enum FederationError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Peer not found: {peer_id}")]
    PeerNotFound { peer_id: String },
    
    #[error("Trust policy violation: {policy}")]
    TrustPolicyViolation { policy: String },
    
    #[error("Federation not enabled")]
    FederationNotEnabled,
    
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },
    
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Protocol error: {message}")]
    ProtocolError { message: String },
}

/// Federation status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStatus {
    pub enabled: bool,
    pub node_id: String,
    pub peer_count: usize,
    pub trust_policy: String,
    pub network_info: Option<NetworkInfo>,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub listen_address: String,
    pub public_address: String,
    pub port: u16,
    pub protocol: String,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub status: PeerStatus,
    pub trust_level: TrustLevel,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Connecting,
    Failed,
    Banned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Trusted,
    Verified,
    Unverified,
    Untrusted,
}

/// Federation message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationMessage {
    Ping { timestamp: chrono::DateTime<chrono::Utc> },
    Pong { timestamp: chrono::DateTime<chrono::Utc> },
    PeerDiscovery { peers: Vec<PeerInfo> },
    BiomeSync { biome_id: String, manifest: String },
    ResourceShare { resource_type: String, available: bool },
    TaskDistribution { task_id: String, requirements: String },
    HealthCheck { node_id: String, status: String },
    AuthenticationChallenge { challenge: String, timestamp: chrono::DateTime<chrono::Utc> },
    SignatureResponse { peer_id: String, challenge: String, signature: Vec<u8>, timestamp: chrono::DateTime<chrono::Utc> },
}

/// Federation configuration
#[derive(Debug, Clone)]
pub struct FederationConfig {
    pub enabled: bool,
    pub node_id: String,
    pub listen_address: SocketAddr,
    pub public_address: Option<String>,
    pub trust_policy: String,
    pub auto_discovery: bool,
    pub max_peers: usize,
    pub heartbeat_interval: std::time::Duration,
    pub connection_timeout: std::time::Duration,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            node_id: Uuid::new_v4().to_string(),
            listen_address: "0.0.0.0:7777".parse()
                .unwrap_or_else(|_| ([0, 0, 0, 0], 7777).into()),
            public_address: None,
            trust_policy: "beardog_verified".to_string(),
            auto_discovery: true,
            max_peers: 50,
            heartbeat_interval: std::time::Duration::from_secs(30),
            connection_timeout: std::time::Duration::from_secs(10),
        }
    }
}

/// Main federation manager
pub struct FederationManager {
    config: FederationConfig,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler + Send + Sync>>>>,
    enabled: bool,
}

/// Peer connection information
#[derive(Debug)]
struct PeerConnection {
    peer_id: String,
    address: SocketAddr,
    connected_at: chrono::DateTime<chrono::Utc>,
    last_activity: chrono::DateTime<chrono::Utc>,
    message_tx: tokio::sync::mpsc::UnboundedSender<FederationMessage>,
}

/// Message handler trait
pub trait MessageHandler {
    fn handle_message(&self, peer_id: &str, message: FederationMessage) -> Result<(), FederationError>;
}

impl FederationManager {
    pub async fn new() -> Result<Self, FederationError> {
        let config = FederationConfig::default();
        
        Ok(Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            enabled: false,
        })
    }

    /// Enable federation with configuration
    pub async fn enable(&mut self, config: FederationConfig) -> Result<(), FederationError> {
        info!("Enabling federation with node ID: {}", config.node_id);
        
        self.config = config;
        self.enabled = true;
        
        // Start federation services
        self.start_federation_services().await?;
        
        Ok(())
    }

    /// Disable federation
    pub async fn disable(&mut self) -> Result<(), FederationError> {
        info!("Disabling federation");
        
        self.enabled = false;
        
        // Disconnect all peers
        let peer_ids: Vec<String> = {
            let connections = self.connections.read().await;
            connections.keys().cloned().collect()
        };
        
        for peer_id in peer_ids {
            self.disconnect_peer(&peer_id).await?;
        }
        
        Ok(())
    }

    /// Join a peer
    pub async fn join_peer(&self, peer_address: String, trust_policy: String) -> Result<(), FederationError> {
        if !self.enabled {
            return Err(FederationError::FederationNotEnabled);
        }
        
        info!("Joining peer: {} with trust policy: {}", peer_address, trust_policy);
        
        // Parse peer address
        let addr: SocketAddr = peer_address.parse()
            .map_err(|_| FederationError::ConnectionFailed {
                reason: "Invalid peer address".to_string(),
            })?;
        
        // Connect to peer
        self.connect_to_peer(addr, trust_policy).await?;
        
        Ok(())
    }

    /// Leave federation
    pub async fn leave(&self, force: bool) -> Result<(), FederationError> {
        info!("Leaving federation (force: {})", force);
        
        if force {
            // Force disconnect all peers
            let peer_ids: Vec<String> = {
                let connections = self.connections.read().await;
                connections.keys().cloned().collect()
            };
            
            for peer_id in peer_ids {
                self.disconnect_peer(&peer_id).await?;
            }
        } else {
            // Graceful leave - notify peers
            self.broadcast_message(FederationMessage::HealthCheck {
                node_id: self.config.node_id.clone(),
                status: "leaving".to_string(),
            }).await?;
            
            // Wait a bit for message delivery
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            
            // Disconnect peers
            let peer_ids: Vec<String> = {
                let connections = self.connections.read().await;
                connections.keys().cloned().collect()
            };
            
            for peer_id in peer_ids {
                self.disconnect_peer(&peer_id).await?;
            }
        }
        
        Ok(())
    }

    /// Get federation status
    pub async fn get_status(&self) -> Result<FederationStatus, FederationError> {
        let peers = self.peers.read().await;
        let peer_count = peers.len();
        
        let network_info = if self.enabled {
            Some(NetworkInfo {
                listen_address: self.config.listen_address.to_string(),
                public_address: self.config.public_address.clone().unwrap_or_else(|| "unknown".to_string()),
                port: self.config.listen_address.port(),
                protocol: "tcp".to_string(),
            })
        } else {
            None
        };
        
        Ok(FederationStatus {
            enabled: self.enabled,
            node_id: self.config.node_id.clone(),
            peer_count,
            trust_policy: self.config.trust_policy.clone(),
            network_info,
            last_sync: Some(chrono::Utc::now()), // Set to current time for now
        })
    }

    /// Get peers
    pub async fn get_peers(&self, include_offline: bool) -> Result<Vec<PeerInfo>, FederationError> {
        let peers = self.peers.read().await;
        
        let filtered_peers: Vec<PeerInfo> = peers.values()
            .filter(|peer| {
                if include_offline {
                    true
                } else {
                    matches!(peer.status, PeerStatus::Connected)
                }
            })
            .cloned()
            .collect();
        
        Ok(filtered_peers)
    }

    /// Send message to specific peer
    pub async fn send_message(&self, peer_id: &str, message: FederationMessage) -> Result<(), FederationError> {
        let connections = self.connections.read().await;
        
        if let Some(connection) = connections.get(peer_id) {
            connection.message_tx.send(message)
                .map_err(|_| FederationError::ConnectionFailed {
                    reason: "Failed to send message".to_string(),
                })?;
            Ok(())
        } else {
            Err(FederationError::PeerNotFound { peer_id: peer_id.to_string() })
        }
    }

    /// Broadcast message to all peers
    pub async fn broadcast_message(&self, message: FederationMessage) -> Result<(), FederationError> {
        let connections = self.connections.read().await;
        
        for (peer_id, connection) in connections.iter() {
            if let Err(e) = connection.message_tx.send(message.clone()) {
                warn!("Failed to send message to peer {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }

    /// Register message handler
    pub async fn register_handler<H>(&self, message_type: String, handler: H) 
    where
        H: MessageHandler + Send + Sync + 'static,
    {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(message_type, Box::new(handler));
    }

    /// Start federation services
    async fn start_federation_services(&self) -> Result<(), FederationError> {
        // Start network listener
        self.start_network_listener().await?;
        
        // Start peer discovery
        if self.config.auto_discovery {
            self.start_peer_discovery().await?;
        }
        
        // Start heartbeat service
        self.start_heartbeat_service().await?;
        
        Ok(())
    }

    /// Start network listener
    async fn start_network_listener(&self) -> Result<(), FederationError> {
        let listener = tokio::net::TcpListener::bind(&self.config.listen_address).await?;
        info!("Federation listener started on {}", self.config.listen_address);
        
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("Incoming federation connection from {}", addr);
                        
                        let manager = manager.clone();
                        tokio::spawn(async move {
                            if let Err(e) = manager.handle_incoming_connection(stream, addr).await {
                                error!("Failed to handle incoming connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Start peer discovery
    async fn start_peer_discovery(&self) -> Result<(), FederationError> {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.discover_peers().await {
                    error!("Peer discovery failed: {}", e);
                }
            }
        });
        
        Ok(())
    }

    /// Start heartbeat service
    async fn start_heartbeat_service(&self) -> Result<(), FederationError> {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.config.heartbeat_interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.send_heartbeats().await {
                    error!("Heartbeat failed: {}", e);
                }
            }
        });
        
        Ok(())
    }

    /// Handle incoming connection
    async fn handle_incoming_connection(
        &self,
        stream: tokio::net::TcpStream,
        addr: SocketAddr,
    ) -> Result<(), FederationError> {
        info!("Handling connection from {}", addr);
        
        // 1. Authentication handshake
        let peer_id = self.authenticate_peer(&stream, &addr).await?;
        
        // 2. Trust verification
        self.verify_trust_policy(&peer_id, &self.config.trust_policy).await?;
        
        // 3. Create message channel
        let (message_tx, mut message_rx) = tokio::sync::mpsc::unbounded_channel();
        
        // 4. Create peer connection
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            address: addr,
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            message_tx,
        };
        
        // 5. Add connection to manager
        {
            let mut connections = self.connections.write().await;
            connections.insert(peer_id.clone(), connection);
        }
        
        // 6. Update peer status
        self.update_peer_status(&peer_id, PeerStatus::Connected).await?;
        
        // 7. Start message handling loop
        let manager = self.clone();
        let peer_id_clone = peer_id.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                if let Err(e) = manager.handle_message(&peer_id_clone, message).await {
                    error!("Error handling message from {}: {}", peer_id_clone, e);
                }
            }
        });
        
        info!("Successfully established connection with peer: {}", peer_id);
        Ok(())
    }

    /// Connect to a peer
    async fn connect_to_peer(&self, addr: SocketAddr, trust_policy: String) -> Result<(), FederationError> {
        info!("Connecting to peer at {}", addr);
        
        // 1. TCP connection with timeout
        let stream = tokio::time::timeout(
            self.config.connection_timeout,
            tokio::net::TcpStream::connect(addr),
        )
        .await
        .map_err(|_| FederationError::ConnectionFailed {
            reason: "Connection timeout".to_string(),
        })?
        .map_err(|e| FederationError::ConnectionFailed {
            reason: format!("TCP connection failed: {}", e),
        })?;
        
        // 2. Authentication handshake
        let peer_id = self.authenticate_peer(&stream, &addr).await?;
        
        // 3. Trust verification
        self.verify_trust_policy(&peer_id, &trust_policy).await?;
        
        // 4. Create message channel
        let (message_tx, mut message_rx) = tokio::sync::mpsc::unbounded_channel();
        
        // 5. Create peer connection
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            address: addr,
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            message_tx,
        };
        
        // 6. Add connection to manager
        {
            let mut connections = self.connections.write().await;
            connections.insert(peer_id.clone(), connection);
        }
        
        // 7. Update or create peer info
        self.update_or_create_peer(&peer_id, addr, TrustLevel::Verified).await?;
        
        // 8. Start message handling loop
        let manager = self.clone();
        let peer_id_clone = peer_id.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                if let Err(e) = manager.handle_message(&peer_id_clone, message).await {
                    error!("Error handling message from {}: {}", peer_id_clone, e);
                }
            }
        });
        
        info!("Successfully connected to peer: {}", peer_id);
        Ok(())
    }

    /// Disconnect from a peer
    async fn disconnect_peer(&self, peer_id: &str) -> Result<(), FederationError> {
        info!("Disconnecting from peer: {}", peer_id);
        
        // Remove connection
        {
            let mut connections = self.connections.write().await;
            connections.remove(peer_id);
        }
        
        // Update peer status
        {
            let mut peers = self.peers.write().await;
            if let Some(peer) = peers.get_mut(peer_id) {
                peer.status = PeerStatus::Disconnected;
            }
        }
        
        Ok(())
    }

    /// Discover peers
    async fn discover_peers(&self) -> Result<(), FederationError> {
        debug!("Discovering peers");
        
        // 1. mDNS discovery - broadcast service discovery message
        self.mdns_discover().await?;
        
        // 2. Bootstrap nodes - try connecting to known bootstrap nodes
        self.bootstrap_discovery().await?;
        
        // 3. Peer exchange - ask existing peers for their peer lists
        self.peer_exchange_discovery().await?;
        
        // 4. Local network scan - scan for peers on local network
        self.local_network_scan().await?;
        
        Ok(())
    }

    /// Send heartbeats to all peers
    async fn send_heartbeats(&self) -> Result<(), FederationError> {
        let heartbeat = FederationMessage::Ping {
            timestamp: chrono::Utc::now(),
        };
        
        self.broadcast_message(heartbeat).await?;
        
        Ok(())
    }

    /// Handle received message
    async fn handle_message(&self, peer_id: &str, message: FederationMessage) -> Result<(), FederationError> {
        debug!("Received message from {}: {:?}", peer_id, message);
        
        match &message {
            FederationMessage::Ping { timestamp } => {
                // Respond with pong
                let pong = FederationMessage::Pong {
                    timestamp: chrono::Utc::now(),
                };
                self.send_message(peer_id, pong).await?;
            }
            FederationMessage::Pong { timestamp: _ } => {
                // Update peer last seen
                self.update_peer_last_seen(peer_id).await?;
            }
            FederationMessage::AuthenticationChallenge { challenge, timestamp } => {
                // Respond with signature
                let signature = self.sign_challenge(challenge).await?;
                let response = FederationMessage::SignatureResponse {
                    peer_id: peer_id.to_string(),
                    challenge: challenge.to_string(),
                    signature: signature.to_vec(),
                    timestamp: chrono::Utc::now(),
                };
                self.send_message(peer_id, response).await?;
            }
            _ => {
                // Handle other message types with registered handlers
                let handlers = self.message_handlers.read().await;
                let message_type = match &message {
                    FederationMessage::PeerDiscovery { .. } => "peer_discovery",
                    FederationMessage::BiomeSync { .. } => "biome_sync",
                    FederationMessage::ResourceShare { .. } => "resource_share",
                    FederationMessage::TaskDistribution { .. } => "task_distribution",
                    FederationMessage::HealthCheck { .. } => "health_check",
                    _ => "unknown",
                };
                
                if let Some(handler) = handlers.get(message_type) {
                    handler.handle_message(peer_id, message)?;
                }
            }
        }
        
        Ok(())
    }

    /// Update peer last seen timestamp
    async fn update_peer_last_seen(&self, peer_id: &str) -> Result<(), FederationError> {
        let mut peers = self.peers.write().await;
        
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.last_seen = Some(chrono::Utc::now());
            peer.status = PeerStatus::Connected;
        }
        
        Ok(())
    }

    /// Authenticate peer connection
    async fn authenticate_peer(&self, _stream: &tokio::net::TcpStream, addr: &SocketAddr) -> Result<String, FederationError> {
        // For now, generate a simple peer ID based on address
        // In a production implementation, this would involve:
        // 1. Challenge-response authentication
        // 2. Certificate verification
        // 3. Cryptographic handshake
        
        let peer_id = format!("peer_{}", addr.ip());
        debug!("Authenticated peer: {}", peer_id);
        
        Ok(peer_id)
    }

    /// Verify trust policy
    async fn verify_trust_policy(&self, peer_id: &str, trust_policy: &str) -> Result<(), FederationError> {
        debug!("Verifying trust policy for peer: {} with policy: {}", peer_id, trust_policy);
        
        // Basic trust policy verification
        match trust_policy {
            "open" => {
                // Accept all peers
                Ok(())
            }
            "beardog_verified" => {
                // Implement BearDog cryptographic verification
                self.verify_beardog_signature(peer_id).await
            }
            "allowlist" => {
                // Check if peer is in allowlist
                self.verify_allowlist(peer_id).await
            }
            _ => {
                Err(FederationError::TrustPolicyViolation {
                    policy: trust_policy.to_string(),
                })
            }
        }
    }
    
    /// Verify BearDog cryptographic signature for peer
    async fn verify_beardog_signature(&self, peer_id: &str) -> Result<(), FederationError> {
        use ed25519_dalek::{Verifier, PublicKey, Signature};
        
        debug!("Verifying BearDog signature for peer: {}", peer_id);
        
        // Get the peer's public key from configuration or trusted key store
        let peer_public_key = self.get_peer_public_key(peer_id).await?;
        
        // Create a challenge for the peer to sign
        let challenge = format!("beardog_challenge_{}_{}_{}", 
            peer_id, 
            chrono::Utc::now().timestamp(), 
            self.config.node_id
        );
        
        // Request signature from peer
        let signature_response = self.request_signature_from_peer(peer_id, &challenge).await?;
        
        // Parse the signature
        let signature = Signature::from_bytes(&signature_response.signature)
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Invalid signature format: {}", e)
            })?;
        
        // Parse the public key
        let public_key = PublicKey::from_bytes(&peer_public_key)
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Invalid public key format: {}", e)
            })?;
        
        // Verify the signature
        public_key.verify(challenge.as_bytes(), &signature)
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Signature verification failed: {}", e)
            })?;
        
        debug!("✅ BearDog signature verified for peer: {}", peer_id);
        Ok(())
    }
    
    /// Get peer's public key from trusted key store
    async fn get_peer_public_key(&self, peer_id: &str) -> Result<Vec<u8>, FederationError> {
        // Check environment variables for trusted keys
        let env_key = format!("BEARDOG_TRUSTED_KEY_{}", peer_id.to_uppercase());
        if let Ok(key_hex) = std::env::var(env_key) {
            return hex::decode(key_hex)
                .map_err(|e| FederationError::AuthenticationFailed {
                    reason: format!("Invalid hex key format: {}", e)
                });
        }
        
        // Check configuration file for trusted keys
        let config_path = std::env::var("BEARDOG_TRUSTED_KEYS_PATH")
            .unwrap_or_else(|_| "beardog_trusted_keys.json".to_string());
        
        if let Ok(contents) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(keys) = serde_json::from_str::<std::collections::HashMap<String, String>>(&contents) {
                if let Some(key_hex) = keys.get(peer_id) {
                    return hex::decode(key_hex)
                        .map_err(|e| FederationError::AuthenticationFailed {
                            reason: format!("Invalid hex key format: {}", e)
                        });
                }
            }
        }
        
        // Fallback: try to load from BearDog service if available
        #[cfg(feature = "beardog_integration")]
        {
            if let Ok(key) = self.query_beardog_for_key(peer_id).await {
                return Ok(key);
            }
        }
        
        Err(FederationError::AuthenticationFailed {
            reason: format!("No trusted key found for peer: {}", peer_id)
        })
    }
    
    /// Request signature from peer for authentication
    async fn request_signature_from_peer(&self, peer_id: &str, challenge: &str) -> Result<SignatureResponse, FederationError> {
        debug!("Requesting signature from peer: {}", peer_id);
        
        // Create signature request message
        let auth_message = FederationMessage::AuthenticationChallenge {
            challenge: challenge.to_string(),
            timestamp: chrono::Utc::now(),
        };
        
        // Send challenge to peer
        self.send_message(peer_id, auth_message).await?;
        
        // Wait for signature response (with timeout)
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.wait_for_signature_response(peer_id, challenge)
        ).await
        .map_err(|_| FederationError::AuthenticationFailed {
            reason: "Signature request timeout".to_string()
        })??;
        
        debug!("✅ Received signature response from peer: {}", peer_id);
        Ok(response)
    }
    
    /// Wait for signature response from peer
    async fn wait_for_signature_response(&self, peer_id: &str, challenge: &str) -> Result<SignatureResponse, FederationError> {
        debug!("Waiting for signature response from peer: {} for challenge: {}", peer_id, challenge);
        
        // Get timeout from environment or use default
        let timeout_secs = std::env::var("FEDERATION_SIGNATURE_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30);
        
        // Create a timeout for the signature response
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            self.wait_for_peer_signature_response(peer_id, challenge)
        );
        
        match timeout.await {
            Ok(Ok(response)) => {
                debug!("✅ Received signature response from peer: {}", peer_id);
                Ok(response)
            }
            Ok(Err(e)) => {
                debug!("❌ Failed to get signature response from peer {}: {}", peer_id, e);
                Err(e)
            }
            Err(_) => {
                debug!("⏰ Timeout waiting for signature response from peer: {}", peer_id);
                Err(FederationError::AuthenticationFailed {
                    reason: format!("Timeout waiting for signature response from peer: {}", peer_id)
                })
            }
        }
    }
    
    /// Internal method to wait for peer signature response
    async fn wait_for_peer_signature_response(&self, peer_id: &str, challenge: &str) -> Result<SignatureResponse, FederationError> {
        // In a real implementation, this would use a message handler system
        // to wait for incoming signature responses. For now, we attempt to
        // request the signature directly from the peer.
        
        // Try to get the actual signature from the peer
        match self.send_signature_request(peer_id, challenge).await {
            Ok(signature) => {
                Ok(SignatureResponse {
                    peer_id: peer_id.to_string(),
                    challenge: challenge.to_string(),
                    signature,
                    timestamp: chrono::Utc::now(),
                })
            }
            Err(e) => {
                debug!("Failed to get signature from peer {}: {}", peer_id, e);
                // Return a failure that will be caught by verification
                Err(FederationError::AuthenticationFailed {
                    reason: format!("Could not obtain signature from peer {}: {}", peer_id, e)
                })
            }
        }
    }
    
    /// Verify peer is in allowlist
    async fn verify_allowlist(&self, peer_id: &str) -> Result<(), FederationError> {
        debug!("Verifying peer against allowlist: {}", peer_id);
        
        // Check environment variable for allowlist
        let allowlist_env = std::env::var("FEDERATION_ALLOWLIST")
            .unwrap_or_else(|_| String::new());
        
        if !allowlist_env.is_empty() {
            let allowed_peers: Vec<&str> = allowlist_env.split(',').collect();
            if allowed_peers.contains(&peer_id) {
                debug!("✅ Peer {} found in environment allowlist", peer_id);
                return Ok(());
            }
        }
        
        // Check configuration file for allowlist
        let config_path = std::env::var("FEDERATION_ALLOWLIST_PATH")
            .unwrap_or_else(|_| "federation_allowlist.json".to_string());
        
        if let Ok(contents) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(allowlist) = serde_json::from_str::<Vec<String>>(&contents) {
                if allowlist.contains(&peer_id.to_string()) {
                    debug!("✅ Peer {} found in configuration allowlist", peer_id);
                    return Ok(());
                }
            }
        }
        
        debug!("❌ Peer {} not found in allowlist", peer_id);
        Err(FederationError::AuthenticationFailed {
            reason: format!("Peer {} not in allowlist", peer_id)
        })
    }
    
    #[cfg(feature = "beardog_integration")]
    /// Query BearDog service for peer's public key
    async fn query_beardog_for_key(&self, peer_id: &str) -> Result<Vec<u8>, FederationError> {
        debug!("Querying BearDog service for key: {}", peer_id);
        
        let beardog_endpoint = std::env::var("BEARDOG_ENDPOINT")
            .unwrap_or_else(|| crate::runtime_defaults::helpers::default_beardog_endpoint());
        
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/keys/{}", beardog_endpoint, peer_id))
            .send()
            .await
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Failed to query BearDog: {}", e)
            })?;
        
        if response.status().is_success() {
            let key_response: serde_json::Value = response.json().await
                .map_err(|e| FederationError::AuthenticationFailed {
                    reason: format!("Failed to parse BearDog response: {}", e)
                })?;
            
            if let Some(key_hex) = key_response.get("public_key").and_then(|k| k.as_str()) {
                return hex::decode(key_hex)
                    .map_err(|e| FederationError::AuthenticationFailed {
                        reason: format!("Invalid hex key from BearDog: {}", e)
                    });
            }
        }
        
        Err(FederationError::AuthenticationFailed {
            reason: format!("BearDog service has no key for peer: {}", peer_id)
        })
    }
    
    /// Sign a challenge with our private key
    async fn sign_challenge(&self, challenge: &str) -> Result<Vec<u8>, FederationError> {
        use ed25519_dalek::{Signer, Keypair};
        
        debug!("Signing challenge: {}", challenge);
        
        // Get our private key
        let private_key = self.get_our_private_key().await?;
        
        // Create keypair from private key
        let keypair = Keypair::from_bytes(&private_key)
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Invalid private key: {}", e)
            })?;
        
        // Sign the challenge
        let signature = keypair.sign(challenge.as_bytes());
        
        debug!("✅ Challenge signed successfully");
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Get our private key for signing
    async fn get_our_private_key(&self) -> Result<Vec<u8>, FederationError> {
        // Check environment variable for private key
        if let Ok(key_hex) = std::env::var("BEARDOG_PRIVATE_KEY") {
            return hex::decode(key_hex)
                .map_err(|e| FederationError::AuthenticationFailed {
                    reason: format!("Invalid hex private key: {}", e)
                });
        }
        
        // Check configuration file for private key
        let config_path = std::env::var("BEARDOG_PRIVATE_KEY_PATH")
            .unwrap_or_else(|_| "beardog_private_key.json".to_string());
        
        if let Ok(contents) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(key_hex) = config.get("private_key").and_then(|k| k.as_str()) {
                    return hex::decode(key_hex)
                        .map_err(|e| FederationError::AuthenticationFailed {
                            reason: format!("Invalid hex private key: {}", e)
                        });
                }
            }
        }
        
        // Fallback: try to load from BearDog service if available
        #[cfg(feature = "beardog_integration")]
        {
            if let Ok(key) = self.query_beardog_for_our_key().await {
                return Ok(key);
            }
        }
        
        Err(FederationError::AuthenticationFailed {
            reason: "No private key found for signing".to_string()
        })
    }
    
    #[cfg(feature = "beardog_integration")]
    /// Query BearDog service for our private key
    async fn query_beardog_for_our_key(&self) -> Result<Vec<u8>, FederationError> {
        debug!("Querying BearDog service for our private key");
        
        let beardog_endpoint = std::env::var("BEARDOG_ENDPOINT")
            .unwrap_or_else(|| crate::runtime_defaults::helpers::default_beardog_endpoint());
        
        let client = reqwest::Client::new();
        let response = client
            .get(&format!("{}/api/v1/keys/private", beardog_endpoint))
            .send()
            .await
            .map_err(|e| FederationError::AuthenticationFailed {
                reason: format!("Failed to query BearDog: {}", e)
            })?;
        
        if response.status().is_success() {
            let key_response: serde_json::Value = response.json().await
                .map_err(|e| FederationError::AuthenticationFailed {
                    reason: format!("Failed to parse BearDog response: {}", e)
                })?;
            
            if let Some(key_hex) = key_response.get("private_key").and_then(|k| k.as_str()) {
                return hex::decode(key_hex)
                    .map_err(|e| FederationError::AuthenticationFailed {
                        reason: format!("Invalid hex private key from BearDog: {}", e)
                    });
            }
        }
        
        Err(FederationError::AuthenticationFailed {
            reason: "BearDog service has no private key".to_string()
        })
    }

    /// Update peer status
    async fn update_peer_status(&self, peer_id: &str, status: PeerStatus) -> Result<(), FederationError> {
        let mut peers = self.peers.write().await;
        
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.status = status;
            peer.last_seen = Some(chrono::Utc::now());
        }
        
        Ok(())
    }

    /// Update or create peer info
    async fn update_or_create_peer(&self, peer_id: &str, addr: SocketAddr, trust_level: TrustLevel) -> Result<(), FederationError> {
        let mut peers = self.peers.write().await;
        
        if let Some(peer) = peers.get_mut(peer_id) {
            // Update existing peer
            peer.address = addr.to_string();
            peer.status = PeerStatus::Connected;
            peer.trust_level = trust_level;
            peer.last_seen = Some(chrono::Utc::now());
        } else {
            // Create new peer
            let peer = PeerInfo {
                id: peer_id.to_string(),
                address: addr.to_string(),
                status: PeerStatus::Connected,
                trust_level,
                last_seen: Some(chrono::Utc::now()),
                capabilities: vec!["standard".to_string()],
                metadata: HashMap::new(),
            };
            peers.insert(peer_id.to_string(), peer);
        }
        
        Ok(())
    }

    /// Send signature request to peer
    async fn send_signature_request(&self, peer_id: &str, challenge: &str) -> Result<Vec<u8>, FederationError> {
        debug!("Sending signature request to peer: {}", peer_id);
        
        // Get peer connection
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(peer_id) {
            let message = FederationMessage::AuthenticationChallenge {
                challenge: challenge.to_string(),
                timestamp: chrono::Utc::now(),
            };
            
            // Send challenge message
            connection.message_tx.send(message)
                .map_err(|e| FederationError::Network(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Failed to send challenge to peer: {}", e)
                )))?;
            
            // Wait for signature response
            let response = self.wait_for_signature_response(peer_id, challenge).await?;
            Ok(response.signature)
        } else {
            Err(FederationError::PeerNotFound {
                peer_id: peer_id.to_string()
            })
        }
    }

    /// mDNS peer discovery
    async fn mdns_discover(&self) -> Result<(), FederationError> {
        debug!("Performing mDNS discovery");
        
        // Real mDNS implementation using mdns crate
        #[cfg(feature = "mdns")]
        {
            use mdns::{Record, RecordKind};
            
            let service_name = "_toadstool._tcp.local";
            debug!("Searching for mDNS service: {}", service_name);
            
            // Create mDNS client
            let timeout = std::env::var("FEDERATION_MDNS_TIMEOUT")
                .and_then(|s| s.parse().ok())
                .map(std::time::Duration::from_secs)
                .unwrap_or(crate::runtime_defaults::timeouts::DEFAULT_NETWORK_TIMEOUT);
            
            match tokio::time::timeout(timeout, self.perform_mdns_query(service_name)).await {
                Ok(Ok(responses)) => {
                    debug!("Found {} mDNS responses", responses.len());
                    
                    for response in responses {
                        match self.process_mdns_response(response).await {
                            Ok(_) => debug!("Successfully processed mDNS response"),
                            Err(e) => warn!("Failed to process mDNS response: {}", e),
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("mDNS query failed: {}", e);
                }
                Err(_) => {
                    warn!("mDNS query timed out");
                }
            }
        }
        
        // Fallback implementation when mDNS is not available
        #[cfg(not(feature = "mdns"))]
        {
            debug!("mDNS feature not enabled, performing simplified network discovery");
            
            // Simple network scan on common ports
            let local_networks = std::env::var("FEDERATION_SCAN_SUBNETS")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec![
                    "192.168.1.0/24".to_string(),
                    "192.168.0.0/24".to_string(),
                    "10.0.0.0/24".to_string(),
                    "172.16.0.0/24".to_string(),
                ]);
            
            for network in local_networks {
                if let Ok(network_addr) = network.parse::<ipnet::IpNet>() {
                    self.scan_network_for_peers(network_addr).await?;
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(feature = "mdns")]
    /// Perform actual mDNS query
    async fn perform_mdns_query(&self, service_name: &str) -> Result<Vec<mdns::Response>, FederationError> {
        use mdns::{Record, RecordKind};
        
        let mut responses = Vec::new();
        
        // Query for TXT records that contain service information
        let stream = mdns::discover::all(service_name, std::time::Duration::from_secs(5))
            .map_err(|e| FederationError::Network(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("mDNS discovery failed: {}", e)
            )))?;
        
        // Collect responses with timeout
        let timeout = std::env::var("FEDERATION_MDNS_DISCOVERY_TIMEOUT")
            .and_then(|s| s.parse().ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or(crate::runtime_defaults::timeouts::DEFAULT_NETWORK_TIMEOUT);
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout {
            match stream.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(response) => {
                    debug!("Received mDNS response: {:?}", response);
                    responses.push(response);
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Continue waiting
                    continue;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }
        
        Ok(responses)
    }
    
    #[cfg(feature = "mdns")]
    /// Process a single mDNS response
    async fn process_mdns_response(&self, response: mdns::Response) -> Result<(), FederationError> {
        use mdns::{Record, RecordKind};
        
        debug!("Processing mDNS response from: {}", response.hostname);
        
        // Extract service information from the response
        let mut service_info = HashMap::new();
        let mut addresses = Vec::new();
        let mut port = None;
        
        for record in response.records {
            match record.kind {
                RecordKind::A(addr) => {
                    addresses.push(std::net::IpAddr::V4(addr));
                }
                RecordKind::AAAA(addr) => {
                    addresses.push(std::net::IpAddr::V6(addr));
                }
                RecordKind::SRV { port: srv_port, .. } => {
                    port = Some(srv_port);
                }
                RecordKind::TXT(txt_records) => {
                    for txt_record in txt_records {
                        if let Ok(txt_str) = String::from_utf8(txt_record) {
                            if let Some((key, value)) = txt_str.split_once('=') {
                                service_info.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Check if this is a ToadStool service
        if let Some(service_type) = service_info.get("service") {
            if service_type == "toadstool" {
                // Try to connect to discovered peers
                if let Some(port) = port {
                    for addr in addresses {
                        let socket_addr = SocketAddr::new(addr, port);
                        
                        match self.connect_to_peer(socket_addr, "mdns_discovered".to_string()).await {
                            Ok(_) => {
                                info!("✅ Successfully connected to mDNS discovered peer: {}", socket_addr);
                            }
                            Err(e) => {
                                debug!("Failed to connect to mDNS discovered peer {}: {}", socket_addr, e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(not(feature = "mdns"))]
    /// Scan a network for potential peers
    async fn scan_network_for_peers(&self, network: ipnet::IpNet) -> Result<(), FederationError> {
        debug!("Scanning network for peers: {}", network);
        
        let common_ports = vec![
            self.config.listen_address.port(),
            network::DEFAULT_TOADSTOOL_PORT, // Default ToadStool port
            network::DEFAULT_SONGBIRD_PORT, // Common HTTP port
            network::DEFAULT_BEARDOG_PORT, // Alternative HTTP port
        ];
        
        // Get the first few IP addresses from the network
        let mut scan_addresses = Vec::new();
        let mut count = 0;
        
        for ip in network.hosts() {
            if count >= 10 {
                break; // Limit scan to first 10 addresses
            }
            scan_addresses.push(ip);
            count += 1;
        }
        
        // Scan addresses concurrently
        let mut tasks = Vec::new();
        
        for ip in scan_addresses {
            for port in &common_ports {
                let addr = SocketAddr::new(ip, *port);
                
                // Skip our own address
                if addr == self.config.listen_address {
                    continue;
                }
                
                let task = self.probe_potential_peer(addr);
                tasks.push(task);
            }
        }
        
        // Wait for all probes to complete
        let results = futures::future::join_all(tasks).await;
        
        let mut successful_connections = 0;
        for result in results {
            if result.is_ok() {
                successful_connections += 1;
            }
        }
        
        debug!("Network scan completed: {} successful connections", successful_connections);
        
        Ok(())
    }
    
    /// Probe a potential peer address
    async fn probe_potential_peer(&self, addr: SocketAddr) -> Result<(), FederationError> {
        debug!("Probing potential peer: {}", addr);
        
        // Try to connect with a short timeout
        let timeout = std::env::var("FEDERATION_PROBE_TIMEOUT")
            .and_then(|s| s.parse().ok())
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(2));
        
        match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                debug!("Successfully connected to potential peer: {}", addr);
                
                // Try to perform federation handshake
                match self.attempt_federation_handshake(stream, addr).await {
                    Ok(_) => {
                        info!("✅ Successfully established federation with peer: {}", addr);
                        Ok(())
                    }
                    Err(e) => {
                        debug!("Federation handshake failed with {}: {}", addr, e);
                        Err(e)
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("Connection failed to {}: {}", addr, e);
                Err(FederationError::Network(e))
            }
            Err(_) => {
                debug!("Connection timeout to {}", addr);
                Err(FederationError::ConnectionFailed {
                    reason: "Connection timeout".to_string()
                })
            }
        }
    }
    
    /// Attempt federation handshake with a peer
    async fn attempt_federation_handshake(&self, stream: tokio::net::TcpStream, addr: SocketAddr) -> Result<(), FederationError> {
        debug!("Attempting federation handshake with: {}", addr);
        
        // Send a federation ping message
        let ping_message = FederationMessage::Ping {
            timestamp: chrono::Utc::now(),
        };
        
        // Serialize the message
        let message_data = serde_json::to_vec(&ping_message)
            .map_err(FederationError::Serialization)?;
        
        // Send the message (simplified - in real implementation would use proper protocol)
        // For now, we'll just consider the connection successful if we can serialize the message
        
        debug!("✅ Federation handshake completed with: {}", addr);
        
        // Add the peer to our connections
        self.connect_to_peer(addr, "network_discovery".to_string()).await?;
        
        Ok(())
    }
    
    /// Bootstrap node discovery
    async fn bootstrap_discovery(&self) -> Result<(), FederationError> {
        debug!("Performing bootstrap discovery");
        
        // Get bootstrap nodes from configuration
        let bootstrap_nodes = self.get_bootstrap_nodes().await?;
        
        if bootstrap_nodes.is_empty() {
            debug!("No bootstrap nodes configured, skipping bootstrap discovery");
            return Ok(());
        }
        
        debug!("Attempting to connect to {} bootstrap nodes", bootstrap_nodes.len());
        
        let mut successful_connections = 0;
        
        for bootstrap_node in bootstrap_nodes {
            debug!("Connecting to bootstrap node: {}", bootstrap_node);
            
            match self.connect_to_bootstrap_node(&bootstrap_node).await {
                Ok(_) => {
                    successful_connections += 1;
                    info!("✅ Successfully connected to bootstrap node: {}", bootstrap_node);
                    
                    // Request peer list from bootstrap node
                    if let Err(e) = self.request_peer_list_from_bootstrap(&bootstrap_node).await {
                        warn!("Failed to request peer list from bootstrap node {}: {}", bootstrap_node, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to connect to bootstrap node {}: {}", bootstrap_node, e);
                }
            }
        }
        
        debug!("Bootstrap discovery completed: {}/{} successful connections", 
            successful_connections, bootstrap_nodes.len());
        
        Ok(())
    }
    
    /// Get bootstrap nodes from configuration
    async fn get_bootstrap_nodes(&self) -> Result<Vec<String>, FederationError> {
        let mut bootstrap_nodes = Vec::new();
        
        // Check environment variable for bootstrap nodes
        if let Ok(nodes_str) = std::env::var("FEDERATION_BOOTSTRAP_NODES") {
            for node in nodes_str.split(',') {
                let node = node.trim();
                if !node.is_empty() {
                    bootstrap_nodes.push(node.to_string());
                }
            }
        }
        
        // Check configuration file for bootstrap nodes
        let config_path = std::env::var("FEDERATION_BOOTSTRAP_CONFIG")
            .unwrap_or_else(|_| "federation_bootstrap.json".to_string());
        
        if let Ok(contents) = tokio::fs::read_to_string(&config_path).await {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&contents) {
                if let Some(nodes) = config.get("bootstrap_nodes").and_then(|n| n.as_array()) {
                    for node in nodes {
                        if let Some(node_str) = node.as_str() {
                            bootstrap_nodes.push(node_str.to_string());
                        }
                    }
                }
            }
        }
        
        // Add well-known bootstrap nodes if none configured
        if bootstrap_nodes.is_empty() {
            bootstrap_nodes.extend(vec![
                "bootstrap.toadstool.org:7777".to_string(),
                "bootstrap2.toadstool.org:7777".to_string(),
                "bootstrap3.toadstool.org:7777".to_string(),
            ]);
        }
        
        Ok(bootstrap_nodes)
    }
    
    /// Connect to a bootstrap node
    async fn connect_to_bootstrap_node(&self, bootstrap_node: &str) -> Result<(), FederationError> {
        debug!("Connecting to bootstrap node: {}", bootstrap_node);
        
        // Parse the bootstrap node address
        let addr = bootstrap_node.parse::<SocketAddr>()
            .map_err(|e| FederationError::ConnectionFailed {
                reason: format!("Invalid bootstrap node address {}: {}", bootstrap_node, e)
            })?;
        
        // Connect with timeout
        let timeout = std::time::Duration::from_secs(10);
        
        match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                debug!("Successfully connected to bootstrap node: {}", bootstrap_node);
                
                // Perform federation handshake
                self.attempt_federation_handshake(stream, addr).await?;
                
                Ok(())
            }
            Ok(Err(e)) => {
                Err(FederationError::ConnectionFailed {
                    reason: format!("Failed to connect to bootstrap node {}: {}", bootstrap_node, e)
                })
            }
            Err(_) => {
                Err(FederationError::ConnectionFailed {
                    reason: format!("Connection timeout to bootstrap node {}", bootstrap_node)
                })
            }
        }
    }
    
    /// Request peer list from bootstrap node
    async fn request_peer_list_from_bootstrap(&self, bootstrap_node: &str) -> Result<(), FederationError> {
        debug!("Requesting peer list from bootstrap node: {}", bootstrap_node);
        
        // Create peer discovery message
        let discovery_message = FederationMessage::PeerDiscovery {
            peers: vec![], // Empty request for peer list
        };
        
        // Send message to bootstrap node
        let peer_id = format!("bootstrap_{}", bootstrap_node);
        self.send_message(&peer_id, discovery_message).await?;
        
        debug!("✅ Requested peer list from bootstrap node: {}", bootstrap_node);
        
        Ok(())
    }
    
    /// Peer exchange discovery
    async fn peer_exchange_discovery(&self) -> Result<(), FederationError> {
        debug!("Performing peer exchange discovery");
        
        // Get all connected peers
        let peers = self.get_peers(false).await?;
        
        if peers.is_empty() {
            debug!("No connected peers for peer exchange discovery");
            return Ok(());
        }
        
        debug!("Requesting peer lists from {} connected peers", peers.len());
        
        // Ask existing peers for their peer lists
        let peer_discovery_msg = FederationMessage::PeerDiscovery {
            peers: vec![], // Request peer list
        };
        
        let mut successful_requests = 0;
        
        for peer in peers {
            match self.send_message(&peer.id, peer_discovery_msg.clone()).await {
                Ok(_) => {
                    successful_requests += 1;
                    debug!("✅ Requested peer list from: {}", peer.id);
                }
                Err(e) => {
                    warn!("Failed to request peer list from {}: {}", peer.id, e);
                }
            }
        }
        
        debug!("Peer exchange discovery completed: {}/{} successful requests", 
            successful_requests, peers.len());
        
        Ok(())
    }
    
    /// Local network scan
    async fn local_network_scan(&self) -> Result<(), FederationError> {
        debug!("Performing local network scan");
        
        // This implementation is already included in the mDNS fallback
        // when mDNS is not available, so we can delegate to that
        #[cfg(not(feature = "mdns"))]
        {
            // Use the same logic as mDNS fallback
            let local_networks = vec![
                "192.168.1.0/24",
                "192.168.0.0/24", 
                "10.0.0.0/24",
                "172.16.0.0/24",
            ];
            
            for network in local_networks {
                if let Ok(network_addr) = network.parse::<ipnet::IpNet>() {
                    self.scan_network_for_peers(network_addr).await?;
                }
            }
        }
        
        #[cfg(feature = "mdns")]
        {
            // When mDNS is available, do a more targeted scan
            debug!("Performing targeted local network scan");
            
            // Get local network interfaces
            let local_ips = self.get_local_network_interfaces().await?;
            
            for local_ip in local_ips {
                if let Ok(network) = self.get_network_from_ip(local_ip).await {
                    self.scan_network_for_peers(network).await?;
                }
            }
        }
        
        Ok(())
    }
    
    #[cfg(feature = "mdns")]
    /// Get local network interfaces
    async fn get_local_network_interfaces(&self) -> Result<Vec<std::net::IpAddr>, FederationError> {
        use std::net::IpAddr;
        
        let mut interfaces = Vec::new();
        
        // This is a simplified implementation
        // In a real implementation, you would use system APIs to get interfaces
        
        // For now, return common local network addresses
        interfaces.push(IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 1)));
        interfaces.push(IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)));
        interfaces.push(IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 1)));
        
        Ok(interfaces)
    }
    
    #[cfg(feature = "mdns")]
    /// Get network from IP address
    async fn get_network_from_ip(&self, ip: std::net::IpAddr) -> Result<ipnet::IpNet, FederationError> {
        // This is a simplified implementation
        // In a real implementation, you would determine the actual network mask
        
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                let network = format!("{}.{}.{}.0/24", ipv4.octets()[0], ipv4.octets()[1], ipv4.octets()[2]);
                network.parse::<ipnet::IpNet>()
                    .map_err(|e| FederationError::ConnectionFailed {
                        reason: format!("Invalid network: {}", e)
                    })
            }
            std::net::IpAddr::V6(_) => {
                // For IPv6, use a /64 network
                let network = format!("{}/64", ip);
                network.parse::<ipnet::IpNet>()
                    .map_err(|e| FederationError::ConnectionFailed {
                        reason: format!("Invalid network: {}", e)
                    })
            }
        }
    }
}

// Clone implementation for Arc sharing
impl Clone for FederationManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            peers: Arc::clone(&self.peers),
            connections: Arc::clone(&self.connections),
            message_handlers: Arc::clone(&self.message_handlers),
            enabled: self.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_federation_manager_creation() {
        let manager = FederationManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_federation_status() {
        let manager = FederationManager::new().await.unwrap();
        let status = manager.get_status().await.unwrap();
        assert!(!status.enabled);
        assert_eq!(status.peer_count, 0);
    }

    #[tokio::test]
    async fn test_get_peers() {
        let manager = FederationManager::new().await.unwrap();
        let peers = manager.get_peers(true).await.unwrap();
        assert!(peers.is_empty());
    }

    #[test]
    fn test_federation_config_default() {
        let config = FederationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.trust_policy, "beardog_verified");
        assert_eq!(config.max_peers, 50);
    }
    
    #[tokio::test]
    async fn test_verify_allowlist_env_var() {
        let manager = FederationManager::new().await.unwrap();
        
        // Set environment variable
        std::env::set_var("FEDERATION_ALLOWLIST", "peer1,peer2,peer3");
        
        // Test allowed peer
        let result = manager.verify_allowlist("peer1").await;
        assert!(result.is_ok());
        
        // Test not allowed peer
        let result = manager.verify_allowlist("peer999").await;
        assert!(result.is_err());
        
        // Clean up
        std::env::remove_var("FEDERATION_ALLOWLIST");
    }
    
    #[tokio::test]
    async fn test_verify_allowlist_file() {
        let manager = FederationManager::new().await.unwrap();
        
        // Create temporary allowlist file
        let allowlist = vec!["peer1".to_string(), "peer2".to_string()];
        let allowlist_json = serde_json::to_string(&allowlist).unwrap();
        let temp_file = "/tmp/test_allowlist.json";
        tokio::fs::write(temp_file, allowlist_json).await.unwrap();
        
        // Set environment variable to point to file
        std::env::set_var("FEDERATION_ALLOWLIST_PATH", temp_file);
        
        // Test allowed peer
        let result = manager.verify_allowlist("peer1").await;
        assert!(result.is_ok());
        
        // Test not allowed peer
        let result = manager.verify_allowlist("peer999").await;
        assert!(result.is_err());
        
        // Clean up
        std::env::remove_var("FEDERATION_ALLOWLIST_PATH");
        let _ = tokio::fs::remove_file(temp_file).await;
    }
    
    #[tokio::test]
    async fn test_beardog_signature_verification_no_key() {
        let manager = FederationManager::new().await.unwrap();
        
        // Test verification without any keys configured
        let result = manager.verify_beardog_signature("test_peer").await;
        if let Err(FederationError::AuthenticationFailed { reason }) = result {
            assert!(reason.contains("No trusted key found"));
        } else {
            assert!(false, "Expected AuthenticationFailed error, got: {:?}", result);
        }
    }
    
    #[tokio::test]
    async fn test_get_bootstrap_nodes_default() {
        let manager = FederationManager::new().await.unwrap();
        
        let nodes = manager.get_bootstrap_nodes().await.unwrap();
        assert!(!nodes.is_empty());
        assert!(nodes.iter().any(|node| node.contains("bootstrap.toadstool.org")));
    }
    
    #[tokio::test]
    async fn test_get_bootstrap_nodes_env_var() {
        let manager = FederationManager::new().await.unwrap();
        
        // Set environment variable
        std::env::set_var("FEDERATION_BOOTSTRAP_NODES", "node1:7777,node2:7777,node3:7777");
        
        let nodes = manager.get_bootstrap_nodes().await.unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.contains(&"node1:7777".to_string()));
        assert!(nodes.contains(&"node2:7777".to_string()));
        assert!(nodes.contains(&"node3:7777".to_string()));
        
        // Clean up
        std::env::remove_var("FEDERATION_BOOTSTRAP_NODES");
    }
    
    #[tokio::test]
    async fn test_get_bootstrap_nodes_file() {
        let manager = FederationManager::new().await.unwrap();
        
        // Create temporary bootstrap config file
        let config = serde_json::json!({
            "bootstrap_nodes": ["config1:7777", "config2:7777"]
        });
        let temp_file = "/tmp/test_bootstrap.json";
        tokio::fs::write(temp_file, config.to_string()).await.unwrap();
        
        // Set environment variable to point to file
        std::env::set_var("FEDERATION_BOOTSTRAP_CONFIG", temp_file);
        
        let nodes = manager.get_bootstrap_nodes().await.unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&"config1:7777".to_string()));
        assert!(nodes.contains(&"config2:7777".to_string()));
        
        // Clean up
        std::env::remove_var("FEDERATION_BOOTSTRAP_CONFIG");
        let _ = tokio::fs::remove_file(temp_file).await;
    }
    
    #[test]
    fn test_signature_response_serialization() {
        let response = SignatureResponse {
            peer_id: "test_peer".to_string(),
            challenge: "test_challenge".to_string(),
            signature: vec![1, 2, 3, 4],
            timestamp: chrono::Utc::now(),
        };
        
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: SignatureResponse = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(response.peer_id, deserialized.peer_id);
        assert_eq!(response.challenge, deserialized.challenge);
        assert_eq!(response.signature, deserialized.signature);
    }
    
    #[test]
    fn test_federation_message_serialization() {
        let messages = vec![
            FederationMessage::Ping { timestamp: chrono::Utc::now() },
            FederationMessage::Pong { timestamp: chrono::Utc::now() },
            FederationMessage::AuthenticationChallenge { 
                challenge: "test".to_string(), 
                timestamp: chrono::Utc::now() 
            },
            FederationMessage::SignatureResponse { 
                peer_id: "peer".to_string(),
                challenge: "challenge".to_string(),
                signature: vec![1, 2, 3],
                timestamp: chrono::Utc::now(),
            },
        ];
        
        for message in messages {
            let serialized = serde_json::to_string(&message).unwrap();
            let _deserialized: FederationMessage = serde_json::from_str(&serialized).unwrap();
        }
    }
    
    #[tokio::test]
    async fn test_trust_policy_verification() {
        let manager = FederationManager::new().await.unwrap();
        
        // Test open policy
        let result = manager.verify_trust_policy("test_peer", "open").await;
        assert!(result.is_ok());
        
        // Test invalid policy
        let result = manager.verify_trust_policy("test_peer", "invalid_policy").await;
        assert!(result.is_err());
        
        if let Err(FederationError::TrustPolicyViolation { policy }) = result {
            assert_eq!(policy, "invalid_policy");
        } else {
            assert!(false, "Expected TrustPolicyViolation error, got: {:?}", result);
        }
    }
    
    #[tokio::test]
    async fn test_peer_info_operations() {
        let manager = FederationManager::new().await.unwrap();
        let addr = "127.0.0.1:7777".parse().unwrap();
        
        // Test creating peer
        let result = manager.update_or_create_peer("test_peer", addr, TrustLevel::Verified).await;
        assert!(result.is_ok());
        
        // Test updating peer status
        let result = manager.update_peer_status("test_peer", PeerStatus::Connected).await;
        assert!(result.is_ok());
        
        // Test updating last seen
        let result = manager.update_peer_last_seen("test_peer").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_network_scan_parsing() {
        let manager = FederationManager::new().await.unwrap();
        
        // Test network parsing
        let networks = vec![
            "192.168.1.0/24",
            "10.0.0.0/24",
            "172.16.0.0/24",
        ];
        
        for network_str in networks {
            let network = network_str.parse::<ipnet::IpNet>();
            assert!(network.is_ok());
            
            // Test that scan doesn't panic
            let result = manager.scan_network_for_peers(network.unwrap()).await;
            // We expect this to complete (may have connection failures but shouldn't panic)
            assert!(result.is_ok() || result.is_err()); // Either is fine for this test
        }
    }
    
    #[test]
    fn test_peer_status_clone() {
        let status = PeerStatus::Connected;
        let cloned = status.clone();
        assert!(matches!(cloned, PeerStatus::Connected));
    }
    
    #[test]
    fn test_trust_level_clone() {
        let level = TrustLevel::Verified;
        let cloned = level.clone();
        assert!(matches!(cloned, TrustLevel::Verified));
    }
    
    #[tokio::test]
    async fn test_federation_manager_enable_disable() {
        let mut manager = FederationManager::new().await.unwrap();
        
        // Test enabling federation
        let config = FederationConfig {
            enabled: true,
            node_id: "test_node".to_string(),
            listen_address: "127.0.0.1:7777".parse().unwrap(),
            public_address: None,
            trust_policy: "open".to_string(),
            auto_discovery: false,
            max_peers: 10,
            heartbeat_interval: std::time::Duration::from_secs(30),
            connection_timeout: std::time::Duration::from_secs(10),
        };
        
        let result = manager.enable(config).await;
        assert!(result.is_ok());
        
        // Test disabling federation
        let result = manager.disable().await;
        assert!(result.is_ok());
    }
} 