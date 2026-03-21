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

pub enum SongbirdJobMessage {
    ExecuteJob {
        job: Box<UniversalJob>,
        reply_channel: mpsc::Sender<SongbirdJobResponse>,
    },
    CancelJob {
        job_id: Uuid,
    },
    StatusUpdate {
        job_id: Uuid,
        status: super::job_types::SubTaskStatus,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdBroadcastMessage {
    CapabilityUpdate {
        node_id: NodeId,
        capabilities: NodeCapabilities,
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
    HealthUpdate {
        node_id: NodeId,
        health_status: String,
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
    CustomMessage {
        message_type: String,
        payload: serde_json::Value,
        #[serde(with = "toadstool_common::system_time_serde")]
        timestamp: SystemTime,
    },
}

impl SongbirdBroadcastMessage {
    /// Derive a routing channel name from the message variant.
    ///
    /// Used by `SongbirdBroadcaster::broadcast()` to route to the correct channel.
    #[allow(clippy::missing_const_for_fn)] // CustomMessage uses message_type.as_str()
    pub fn channel_name(&self) -> &str {
        match self {
            Self::CapabilityUpdate { .. } => "capability-updates",
            Self::HealthUpdate { .. } => "health-updates",
            Self::CustomMessage { message_type, .. } => message_type.as_str(),
        }
    }
}
