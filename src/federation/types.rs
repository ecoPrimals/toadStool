//! Federation types, errors, and data structures
//!
//! This module contains all type definitions for the federation system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use thiserror::Error;
use uuid::Uuid;

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
    pub last_sync: Option<DateTime<Utc>>,
}

/// Network information
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
    pub last_seen: Option<DateTime<Utc>>,
    pub capabilities: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Peer connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    Connecting,
    Failed,
    Banned,
}

/// Trust level for peers
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
    Ping { timestamp: DateTime<Utc> },
    Pong { timestamp: DateTime<Utc> },
    PeerDiscovery { peers: Vec<PeerInfo> },
    BiomeSync { biome_id: String, manifest: String },
    ResourceShare { resource_type: String, available: bool },
    TaskDistribution { task_id: String, requirements: String },
    HealthCheck { node_id: String, status: String },
    AuthenticationChallenge { challenge: String, timestamp: DateTime<Utc> },
    SignatureResponse { 
        peer_id: String, 
        challenge: String, 
        signature: Vec<u8>, 
        timestamp: DateTime<Utc> 
    },
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
        use toadstool_config::defaults::network;
        
        Self {
            enabled: false,
            node_id: Uuid::new_v4().to_string(),
            listen_address: format!("{}:{}", network::LOCALHOST, network::FEDERATION_PORT)
                .parse()
                .unwrap_or_else(|_| ([0, 0, 0, 0], network::FEDERATION_PORT).into()),
            public_address: None,
            trust_policy: "beardog_verified".to_string(),
            auto_discovery: true,
            max_peers: 50,
            heartbeat_interval: std::time::Duration::from_secs(30),
            connection_timeout: std::time::Duration::from_secs(10),
        }
    }
}

/// Peer connection information (internal)
#[derive(Debug)]
pub(crate) struct PeerConnection {
    pub peer_id: String,
    pub address: SocketAddr,
    pub connected_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub message_tx: tokio::sync::mpsc::UnboundedSender<FederationMessage>,
}

/// Message handler trait
pub trait MessageHandler {
    fn handle_message(&self, peer_id: &str, message: FederationMessage) 
        -> Result<(), FederationError>;
}

/// Type alias for federation results
pub type Result<T> = std::result::Result<T, FederationError>;

