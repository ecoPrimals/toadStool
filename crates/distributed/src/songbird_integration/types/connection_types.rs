//! Connection types for Songbird integration

#[cfg(feature = "channels")]
use std::sync::Arc;

#[cfg(feature = "channels")]
use super::job_types::SongbirdJobResponse;
use super::protocols::ProtocolConfig;

// ============================================================================
// Connection Types (required by discovery)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SongbirdConnection {
    pub endpoints: Vec<String>,
    pub active_endpoint: String,
    pub auth_token: Option<String>,
    pub health_status: ConnectionHealth,
    pub protocol_config: ProtocolConfig,
    #[cfg(feature = "channels")]
    pub reply_channel: Option<Arc<tokio::sync::mpsc::UnboundedSender<SongbirdJobResponse>>>,
}
