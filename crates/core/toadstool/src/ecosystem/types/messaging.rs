// SPDX-License-Identifier: AGPL-3.0-only
//! Structured messages exchanged between ecosystem services.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Ecosystem message for primal communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Source service ID
    pub from: String,
    /// Destination service ID
    pub to: String,
    /// Message type
    pub message_type: EcosystemMessageType,
    /// Message payload (JSON)
    pub payload: serde_json::Value,
    /// Message timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

impl EcosystemMessage {
    /// Create a new message
    pub fn new(
        from: String,
        to: String,
        message_type: EcosystemMessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            message_type,
            payload,
            timestamp: SystemTime::now(),
        }
    }

    /// Create a heartbeat message
    pub fn heartbeat(from: String, to: String) -> Self {
        Self::new(
            from,
            to,
            EcosystemMessageType::Heartbeat,
            serde_json::json!({}),
        )
    }

    /// Create an error message
    pub fn error(from: String, to: String, error: String) -> Self {
        Self::new(
            from,
            to,
            EcosystemMessageType::Error,
            serde_json::json!({ "error": error }),
        )
    }
}

/// Types of ecosystem messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EcosystemMessageType {
    /// Heartbeat message
    Heartbeat,
    /// Capability announcement
    CapabilityAnnouncement,
    /// Resource request
    ResourceRequest,
    /// Resource response
    ResourceResponse,
    /// Workload request
    WorkloadRequest,
    /// Workload response
    WorkloadResponse,
    /// Status update
    StatusUpdate,
    /// Error message
    Error,
}

impl EcosystemMessageType {
    /// Check if this message requires a response
    pub const fn requires_response(&self) -> bool {
        matches!(self, Self::ResourceRequest | Self::WorkloadRequest)
    }

    /// Check if this is a response message
    pub const fn is_response(&self) -> bool {
        matches!(self, Self::ResourceResponse | Self::WorkloadResponse)
    }
}
