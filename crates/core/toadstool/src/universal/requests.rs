//! # Request and Response Types
//!
//! Types for inter-primal communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::types::PrimalContext;

/// Primal API endpoints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimalEndpoints {
    /// Primary API endpoint
    pub primary: String,
    /// Health check endpoint
    pub health: String,
    /// Metrics endpoint
    pub metrics: Option<String>,
    /// Admin endpoint
    pub admin: Option<String>,
    /// WebSocket endpoint
    pub websocket: Option<String>,
    /// Custom endpoints
    pub custom: HashMap<String, String>,
}

/// Inter-primal request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalRequest {
    /// Request ID
    pub id: Uuid,
    /// Source primal
    pub source: String,
    /// Target primal
    pub target: String,
    /// Request type
    pub request_type: String,
    /// Request payload
    pub payload: serde_json::Value,
    /// Request context
    pub context: PrimalContext,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Success
    Success,
    /// Error with details
    Error { code: String, message: String },
    /// Timeout
    Timeout,
    /// Service unavailable
    ServiceUnavailable,
}

/// Inter-primal response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResponse {
    /// Request ID this response is for
    pub request_id: Uuid,
    /// Response status
    pub status: ResponseStatus,
    /// Response payload
    pub payload: serde_json::Value,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Response timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
