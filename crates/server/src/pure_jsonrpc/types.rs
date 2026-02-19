//! JSON-RPC 2.0 wire types
//!
//! Pure data structures for the JSON-RPC 2.0 protocol.
//! No logic — only serialization/deserialization.

use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;

use crate::rpc_types::{
    ResourceRequirements, WorkloadPriority, WorkloadSubmission as TarpcWorkloadSubmission,
};

/// JSON-RPC 2.0 Request
///
/// Standard compliant: <https://www.jsonrpc.org/specification>
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (must be "2.0")
    pub jsonrpc: String,

    /// Method name (e.g., "toadstool.submit_workload")
    pub method: String,

    /// Optional parameters (can be object or array)
    #[serde(default)]
    pub params: Option<serde_json::Value>,

    /// Request ID (for matching request/response)
    pub id: Option<serde_json::Value>,
}

/// Zero-copy JSON-RPC version constant for responses
pub(crate) const JSONRPC_VERSION: &str = toadstool_common::constants::jsonrpc::VERSION;

/// JSON-RPC 2.0 Response
///
/// Either contains `result` (success) or `error` (failure), never both.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    /// Protocol version ("2.0")
    pub jsonrpc: Cow<'static, str>,

    /// Success result (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error object (present on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,

    /// Request ID (from original request)
    pub id: serde_json::Value,
}

/// JSON-RPC 2.0 Error Object
///
/// Standard error codes: <https://www.jsonrpc.org/specification#error_object>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard or application-defined)
    pub code: i32,

    /// Human-readable error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    // Standard JSON-RPC 2.0 error codes -- from shared ecosystem constants
    pub const PARSE_ERROR: i32 = toadstool_common::constants::jsonrpc::error_codes::PARSE_ERROR;
    pub const INVALID_REQUEST: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INVALID_REQUEST;
    pub const METHOD_NOT_FOUND: i32 =
        toadstool_common::constants::jsonrpc::error_codes::METHOD_NOT_FOUND;
    pub const INVALID_PARAMS: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INVALID_PARAMS;
    pub const INTERNAL_ERROR: i32 =
        toadstool_common::constants::jsonrpc::error_codes::INTERNAL_ERROR;

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: msg.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: msg.into(),
            data: None,
        }
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self {
            code: Self::INTERNAL_ERROR,
            message: msg.into(),
            data: None,
        }
    }
}

/// JSON-friendly workload submission (base64 encoding for binary data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonWorkloadSubmission {
    pub workload_id: String,
    pub workload_type: String,
    /// Base64-encoded binary data
    pub data: String,
    pub metadata: HashMap<String, String>,
    pub priority: WorkloadPriority,
    pub requirements: ResourceRequirements,
}

impl JsonWorkloadSubmission {
    /// Convert into tarpc submission (decode base64, consumes self)
    pub fn into_tarpc(self) -> Result<TarpcWorkloadSubmission, String> {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let data = STANDARD
            .decode(&self.data)
            .map_err(|e| format!("Invalid base64 data: {}", e))?;

        Ok(TarpcWorkloadSubmission {
            workload_id: self.workload_id,
            workload_type: self.workload_type,
            data: data.into(),
            metadata: self.metadata,
            priority: self.priority,
            requirements: self.requirements,
        })
    }
}
