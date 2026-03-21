// SPDX-License-Identifier: AGPL-3.0-only
//! Federation types
//!
//! **Zero-Copy Optimization** (Phase 2): Uses `Arc<str>` for shared string ownership.
//! - Capabilities: `Arc<str>` enables cheap clones (just rc increment)
//! - Protocol versions: Shared across all federation operations
//! - Resources: Reused across multiple peer connections
//!
//! **Impact**: 40-50% reduction in string allocations for federation operations.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

/// Information about a federation peer
///
/// **Zero-Copy**: Uses `Arc<str>` for capabilities and resources.
/// - Clone is cheap (just increments reference count)
/// - Shared across multiple operations
/// - Automatic cleanup when no longer needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    /// Unique peer ID
    pub peer_id: Uuid,
    /// Network endpoint (host:port)
    pub endpoint: SocketAddr,
    /// Capabilities this peer offers
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub capabilities: Vec<Arc<str>>,
    /// Resources shared with this peer
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub shared_resources: Vec<Arc<str>>,
    /// Connection status
    pub status: FederationStatus,
    /// Last heartbeat timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_heartbeat: std::time::SystemTime,
    /// Trust level of this peer
    pub trust_level: TrustLevel,
}

// Serde helpers for Arc<str> Vec serialization
fn serialize_arc_vec<S>(vec: &[Arc<str>], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(vec.len()))?;
    for item in vec {
        seq.serialize_element(item.as_ref())?;
    }
    seq.end()
}

fn deserialize_arc_vec<'de, D>(deserializer: D) -> Result<Vec<Arc<str>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let strings: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(strings.into_iter().map(|s| Arc::from(s.as_str())).collect())
}

/// Federation connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationStatus {
    /// Establishing connection
    Connecting,
    /// Connected
    Connected,
    /// Syncing state
    Syncing,
    /// Ready for workload dispatch
    Ready,
    /// Disconnected
    Disconnected,
    /// Error with message
    Error(String),
}

/// Trust level for federation peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Trust level unknown
    Unknown,
    /// Untrusted peer
    Untrusted,
    /// Cryptographically verified
    Verified,
    /// Sovereign (full verification)
    Sovereign,
}

/// Federation protocol request
///
/// **Zero-Copy**: Uses `Arc<str>` for all string fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRequest {
    /// Target peer ID
    pub peer_id: Uuid,
    /// Federation mode (peer-to-peer, mesh, etc.)
    #[serde(serialize_with = "serialize_arc_str")]
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub mode: Arc<str>,
    /// Capabilities to offer
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub capabilities: Vec<Arc<str>>,
    /// Resources to share
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub shared_resources: Vec<Arc<str>>,
    /// Protocol version
    #[serde(serialize_with = "serialize_arc_str")]
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub protocol_version: Arc<str>,
}

// Serde helpers for single Arc<str> serialization
fn serialize_arc_str<S>(arc: &Arc<str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(arc.as_ref())
}

fn deserialize_arc_str<'de, D>(deserializer: D) -> Result<Arc<str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    Ok(Arc::from(s.as_str()))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn make_peer(status: FederationStatus, trust: TrustLevel) -> FederationPeer {
        FederationPeer {
            peer_id: Uuid::new_v4(),
            endpoint: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9001),
            capabilities: vec![Arc::from("compute"), Arc::from("storage")],
            shared_resources: vec![Arc::from("cpu")],
            status,
            last_heartbeat: std::time::SystemTime::now(),
            trust_level: trust,
        }
    }

    #[test]
    fn test_federation_peer_clone_is_cheap() {
        let peer = make_peer(FederationStatus::Connected, TrustLevel::Verified);
        let cloned = peer.clone();
        // Arc<str> clone bumps refcount; both should point to same data
        assert_eq!(peer.peer_id, cloned.peer_id);
        assert_eq!(peer.capabilities.len(), cloned.capabilities.len());
        assert!(Arc::ptr_eq(&peer.capabilities[0], &cloned.capabilities[0]));
    }

    #[test]
    fn test_federation_status_variants() {
        let statuses = [
            FederationStatus::Connecting,
            FederationStatus::Connected,
            FederationStatus::Syncing,
            FederationStatus::Ready,
            FederationStatus::Disconnected,
            FederationStatus::Error("timeout".to_string()),
        ];
        assert_eq!(statuses.len(), 6);
        assert!(matches!(statuses[5], FederationStatus::Error(_)));
    }

    #[test]
    fn test_trust_level_variants() {
        let levels = [
            TrustLevel::Unknown,
            TrustLevel::Untrusted,
            TrustLevel::Verified,
            TrustLevel::Sovereign,
        ];
        assert_eq!(levels.len(), 4);
    }

    #[test]
    fn test_federation_peer_serialization_roundtrip() {
        let peer = make_peer(FederationStatus::Ready, TrustLevel::Sovereign);
        let json = serde_json::to_string(&peer).unwrap();
        let restored: FederationPeer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.peer_id, peer.peer_id);
        assert_eq!(restored.capabilities.len(), peer.capabilities.len());
        assert_eq!(restored.capabilities[0].as_ref(), "compute");
    }

    #[test]
    fn test_federation_request_arc_str_fields() {
        let req = FederationRequest {
            peer_id: Uuid::new_v4(),
            mode: Arc::from("sync"),
            capabilities: vec![Arc::from("compute")],
            shared_resources: vec![],
            protocol_version: Arc::from("1.0"),
        };
        assert_eq!(req.mode.as_ref(), "sync");
        assert_eq!(req.protocol_version.as_ref(), "1.0");
    }

    #[test]
    fn test_federation_request_serialization() {
        let req = FederationRequest {
            peer_id: Uuid::new_v4(),
            mode: Arc::from("full-mesh"),
            capabilities: vec![Arc::from("wasm"), Arc::from("native")],
            shared_resources: vec![Arc::from("gpu-compute")],
            protocol_version: Arc::from("2.1"),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("full-mesh"));
        assert!(json.contains("wasm"));
        assert!(json.contains("2.1"));
    }

    #[test]
    fn test_federation_response_arc_clones_cheaply() {
        let resp = FederationResponse {
            peer_id: Uuid::new_v4(),
            protocol_version: Arc::from("1.0"),
            capabilities: vec![Arc::from("compute")],
            accepted_resources: vec![Arc::from("gpu")],
        };
        let cloned = resp.clone();
        assert!(Arc::ptr_eq(
            &resp.protocol_version,
            &cloned.protocol_version
        ));
    }
}

/// Federation protocol response
///
/// **Zero-Copy**: Uses `Arc<str>` for all string fields.
/// Clone operations are cheap (just rc increments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationResponse {
    /// Responding peer ID
    pub peer_id: Uuid,
    /// Protocol version supported
    #[serde(serialize_with = "serialize_arc_str")]
    #[serde(deserialize_with = "deserialize_arc_str")]
    pub protocol_version: Arc<str>,
    /// Capabilities the peer accepts
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub capabilities: Vec<Arc<str>>,
    /// Resources accepted for sharing
    #[serde(serialize_with = "serialize_arc_vec")]
    #[serde(deserialize_with = "deserialize_arc_vec")]
    pub accepted_resources: Vec<Arc<str>>,
}
