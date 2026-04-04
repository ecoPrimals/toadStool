// SPDX-License-Identifier: AGPL-3.0-only
//! Message types for Songbird integration

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::UniversalJob;

use super::job_types::SongbirdJobResponse;
use super::node::NodeCapabilities;

/// Node identifier re-export for message types
pub type NodeId = super::node::NodeId;

// ============================================================================
// Message Types
// ============================================================================

/// In-process job control messages for the Songbird integration path.
pub enum SongbirdJobMessage {
    /// Run a job and send the result on `reply_channel`.
    ExecuteJob {
        /// Work to execute.
        job: Box<UniversalJob>,
        /// Channel for the final [`SongbirdJobResponse`].
        reply_channel: mpsc::Sender<SongbirdJobResponse>,
    },
    /// Request cancellation of a job by id.
    CancelJob {
        /// Target job identifier.
        job_id: Uuid,
    },
    /// Subtask or job status changed.
    StatusUpdate {
        /// Job or subtask identifier.
        job_id: Uuid,
        /// New status.
        status: super::job_types::SubTaskStatus,
    },
}

/// Broadcast payloads for capability, health, or custom fan-out channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdBroadcastMessage {
    /// Node capabilities changed.
    CapabilityUpdate {
        /// Originating node.
        node_id: NodeId,
        /// Advertised capabilities.
        capabilities: NodeCapabilities,
        /// When the update was produced.
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
    /// Node health signal.
    HealthUpdate {
        /// Originating node.
        node_id: NodeId,
        /// Opaque health summary (e.g. `"ok"`, `"degraded"`).
        health_status: String,
        /// When the update was produced.
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
    /// Application-defined broadcast.
    CustomMessage {
        /// Routing label for subscribers.
        message_type: String,
        /// JSON payload.
        payload: serde_json::Value,
        /// When the message was produced.
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
}

impl SongbirdBroadcastMessage {
    /// Derive a routing channel name from the message variant.
    ///
    /// Used by `SongbirdBroadcaster::broadcast()` to route to the correct channel.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "not const due to future evolution"
    )] // CustomMessage uses message_type.as_str()
    pub fn channel_name(&self) -> &str {
        match self {
            Self::CapabilityUpdate { .. } => "capability-updates",
            Self::HealthUpdate { .. } => "health-updates",
            Self::CustomMessage { message_type, .. } => message_type.as_str(),
        }
    }
}
