//! Federation types

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// Information about a federation peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub peer_id: Uuid,
    pub endpoint: SocketAddr,
    pub capabilities: Vec<String>,
    pub shared_resources: Vec<String>,
    pub status: FederationStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub trust_level: TrustLevel,
}

/// Federation connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationStatus {
    Connecting,
    Connected,
    Syncing,
    Ready,
    Disconnected,
    Error(String),
}

/// Trust level for federation peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Untrusted,
    Verified,
    Sovereign,
}

/// Federation protocol request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FederationRequest {
    pub peer_id: Uuid,
    pub mode: String,
    pub capabilities: Vec<String>,
    pub shared_resources: Vec<String>,
    pub protocol_version: String,
}

/// Federation protocol response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct FederationResponse {
    pub peer_id: Uuid,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
    pub accepted_resources: Vec<String>,
}
