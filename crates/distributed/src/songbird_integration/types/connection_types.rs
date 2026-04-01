// SPDX-License-Identifier: AGPL-3.0-only
//! Connection types for Songbird integration

#[cfg(feature = "channels")]
use std::sync::Arc;

#[cfg(feature = "channels")]
use super::job_types::SongbirdJobResponse;
use super::protocols::ProtocolConfig;

// ============================================================================
// Connection Types (required by discovery)
// ============================================================================

/// Health of the Songbird connection from the client's perspective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    /// Fully healthy.
    Healthy,
    /// Partially degraded but usable.
    Degraded,
    /// Unusable or failing health checks.
    Unhealthy,
    /// Health not yet determined.
    Unknown,
}

/// Active connection state for a Songbird peer (endpoints, auth, protocol).
#[derive(Debug, Clone)]
pub struct SongbirdConnection {
    /// Candidate endpoint URLs or addresses.
    pub endpoints: Vec<String>,
    /// Currently selected endpoint.
    pub active_endpoint: String,
    /// Optional bearer or session token for RPCs.
    pub auth_token: Option<String>,
    /// Latest observed connection health.
    pub health_status: ConnectionHealth,
    /// Wire protocol and framing configuration.
    pub protocol_config: ProtocolConfig,
    /// Optional channel for async job replies when `channels` is enabled.
    #[cfg(feature = "channels")]
    pub reply_channel: Option<Arc<tokio::sync::mpsc::UnboundedSender<SongbirdJobResponse>>>,
}
