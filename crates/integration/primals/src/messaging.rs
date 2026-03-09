// SPDX-License-Identifier: AGPL-3.0-only
//! Inter-Primal messaging and metrics types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Primal metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in GB
    pub memory_usage: f64,
    /// Storage usage in GB
    pub storage_usage: f64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, serde_json::Value>,
    /// Metrics timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Inter-Primal communication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalMessage {
    /// Message ID
    pub id: Uuid,
    /// Source Primal
    pub from: String,
    /// Destination Primal
    pub to: String,
    /// Message type
    pub message_type: PrimalMessageType,
    /// Message payload
    pub payload: serde_json::Value,
    /// Message timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Message headers
    pub headers: HashMap<String, String>,
}

/// Types of inter-Primal messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalMessageType {
    /// Configuration update
    ConfigUpdate,
    /// Resource request
    ResourceRequest,
    /// Resource response
    ResourceResponse,
    /// Health check
    HealthCheck,
    /// Metrics request
    MetricsRequest,
    /// Metrics response
    MetricsResponse,
    /// Service discovery
    ServiceDiscovery,
    /// Authentication token
    AuthToken,
    /// Custom message
    Custom(String),
}
