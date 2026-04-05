// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

pub(super) fn serialize_arc_str<S>(v: &Arc<str>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(v)
}

pub(super) fn deserialize_arc_str<'de, D>(d: D) -> Result<Arc<str>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    Ok(Arc::from(s))
}

/// GPU capabilities for a single gate
///
/// Uses `Arc<str>` for gate_id to avoid allocations on hot-path route decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateGpuInfo {
    /// Gate identifier (e.g., "tower", "gate2")
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub gate_id: Arc<str>,
    /// GPU model name (e.g., "RTX 4070", "RTX 3090")
    pub gpu_model: String,
    /// Total VRAM in MB
    pub vram_total_mb: u64,
    /// Available VRAM in MB
    pub vram_available_mb: u64,
    /// Models currently loaded in VRAM
    pub loaded_models: Vec<String>,
    /// Current queue depth (pending jobs)
    pub queue_depth: usize,
    /// Whether this gate is reachable via mesh
    pub reachable: bool,
    /// Remote endpoint for this gate (Unix socket path or host:port).
    /// Only present for remote gates — local gate has `None`.
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// Routing decision for a compute job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected gate ID (`Arc<str>` avoids allocation on hot path)
    #[serde(
        serialize_with = "serialize_arc_str",
        deserialize_with = "deserialize_arc_str"
    )]
    pub gate_id: Arc<str>,
    /// Reason for selection
    pub reason: RoutingReason,
    /// Estimated wait time in milliseconds
    pub estimated_wait_ms: u64,
}

/// Why a particular gate was selected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingReason {
    /// Model already loaded in VRAM (fastest)
    ModelLoaded,
    /// Most VRAM available for new model loading
    MostVramAvailable,
    /// Shortest queue (lowest wait time)
    ShortestQueue,
    /// Only gate available
    OnlyOption,
    /// Local execution (no mesh hop needed)
    Local,
}

/// Error from remote dispatch.
#[derive(Debug, thiserror::Error)]
pub enum RemoteDispatchError {
    /// Transport/connection error.
    #[error("transport error: {0}")]
    Transport(String),
    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialize(String),
    /// Remote gate returned an error.
    #[error("remote error: {0}")]
    Remote(String),
}
