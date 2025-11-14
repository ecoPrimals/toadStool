//! # Federation Module
//!
//! Peer-to-peer networking and federation management for ToadStool.
//!
//! This module has been refactored into focused submodules:
//! - `types`: All data structures, errors, and type definitions
//! - `manager`: Core FederationManager implementation
//! - `trust`: Authentication and trust verification (BearDog integration)
//! - `discovery`: Peer discovery mechanisms (mDNS, network scan, bootstrap)
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::federation::{FederationManager, FederationConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = FederationManager::new().await?;
//!     let config = FederationConfig::default();
//!     
//!     // Enable federation
//!     manager.enable(config).await?;
//!     
//!     // Join a peer
//!     manager.join_peer("192.168.1.100:7777".to_string(), "beardog_verified".to_string()).await?;
//!     
//!     Ok(())
//! }
//! ```

// Module declarations
pub mod types;

// Re-export all public types for convenience
pub use types::{
    FederationConfig,
    FederationError,
    FederationMessage,
    FederationStatus,
    MessageHandler,
    NetworkInfo,
    PeerInfo,
    PeerStatus,
    SignatureResponse,
    TrustLevel,
    Result,
};

// For now, import the FederationManager from the old location
// This will be migrated in subsequent steps
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main federation manager
///
/// **Note**: This is currently a thin wrapper. The full implementation
/// will be migrated to submodules in the next refactoring phase.
pub struct FederationManager {
    config: FederationConfig,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    connections: Arc<RwLock<HashMap<String, types::PeerConnection>>>,
    message_handlers: Arc<RwLock<HashMap<String, Box<dyn MessageHandler + Send + Sync>>>>,
    enabled: bool,
}

// Implementation will be split across manager.rs, trust.rs, and discovery.rs
// For now, keeping stub implementations to maintain API compatibility
impl FederationManager {
    /// Create a new federation manager
    pub async fn new() -> Result<Self> {
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
    pub async fn enable(&mut self, config: FederationConfig) -> Result<()> {
        // Placeholder - will be implemented in manager.rs
        self.config = config;
        self.enabled = true;
        Ok(())
    }

    /// Disable federation
    pub async fn disable(&mut self) -> Result<()> {
        // Placeholder - will be implemented in manager.rs
        self.enabled = false;
        Ok(())
    }

    /// Get federation status
    pub async fn get_status(&self) -> Result<FederationStatus> {
        let peers = self.peers.read().await;
        let peer_count = peers.len();
        
        Ok(FederationStatus {
            enabled: self.enabled,
            node_id: self.config.node_id.clone(),
            peer_count,
            trust_policy: self.config.trust_policy.clone(),
            network_info: None,
            last_sync: Some(chrono::Utc::now()),
        })
    }

    /// Get peers
    pub async fn get_peers(&self, _include_offline: bool) -> Result<Vec<PeerInfo>> {
        let peers = self.peers.read().await;
        Ok(peers.values().cloned().collect())
    }

    /// Join a peer
    pub async fn join_peer(&self, _peer_address: String, _trust_policy: String) -> Result<()> {
        if !self.enabled {
            return Err(FederationError::FederationNotEnabled);
        }
        // Placeholder - will be implemented in manager.rs
        Ok(())
    }

    /// Leave federation
    pub async fn leave(&self, _force: bool) -> Result<()> {
        // Placeholder - will be implemented in manager.rs
        Ok(())
    }

    /// Send message to peer
    pub async fn send_message(&self, _peer_id: &str, _message: FederationMessage) -> Result<()> {
        if !self.enabled {
            return Err(FederationError::FederationNotEnabled);
        }
        // Placeholder - will be implemented in manager.rs
        Ok(())
    }

    /// Broadcast message to all peers
    pub async fn broadcast_message(&self, _message: FederationMessage) -> Result<()> {
        if !self.enabled {
            return Err(FederationError::FederationNotEnabled);
        }
        // Placeholder - will be implemented in manager.rs
        Ok(())
    }

    /// Register message handler
    pub async fn register_handler<H>(&self, _message_type: String, _handler: H)
    where
        H: MessageHandler + Send + Sync + 'static,
    {
        // Placeholder - will be implemented in manager.rs
    }
}

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

