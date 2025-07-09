//! # Federation Manager
//!
//! Handles peer-to-peer networking and federation management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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
            listen_address: "0.0.0.0:7777".parse().unwrap(),
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
            last_sync: None, // TODO: Track last sync time
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
        // TODO: Implement connection handling
        // This would involve:
        // 1. Authentication
        // 2. Trust verification
        // 3. Message handling
        // 4. Connection management
        
        info!("Handling connection from {}", addr);
        
        Ok(())
    }

    /// Connect to a peer
    async fn connect_to_peer(&self, addr: SocketAddr, trust_policy: String) -> Result<(), FederationError> {
        info!("Connecting to peer at {}", addr);
        
        // TODO: Implement peer connection
        // This would involve:
        // 1. TCP connection
        // 2. Authentication handshake
        // 3. Trust verification
        // 4. Message channel setup
        
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
        
        // TODO: Implement peer discovery
        // This could involve:
        // 1. mDNS discovery
        // 2. DHT lookup
        // 3. Bootstrap nodes
        // 4. Peer exchange
        
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
} 