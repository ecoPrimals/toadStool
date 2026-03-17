// SPDX-License-Identifier: AGPL-3.0-only
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
///
/// Uses `#[serde(borrow)]` and `Cow` for zero-copy deserialization when parsing
/// from network bytes via `serde_json::from_slice`. Method names are often
/// static literals, so borrowing avoids allocation on the hot path.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest<'a> {
    /// Protocol version (must be "2.0")
    #[serde(borrow)]
    pub jsonrpc: Cow<'a, str>,

    /// Method name (e.g., "toadstool.submit_workload")
    #[serde(borrow)]
    pub method: Cow<'a, str>,

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
/// Uses `Cow<'static, str>` for zero-copy static error messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code (standard or application-defined)
    pub code: i32,

    /// Human-readable error message (Cow for zero-copy static strings)
    pub message: Cow<'static, str>,

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

    pub fn parse_error(msg: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: Self::PARSE_ERROR,
            message: msg.into(),
            data: None,
        }
    }

    pub fn invalid_request(msg: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: Self::INVALID_REQUEST,
            message: msg.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: Self::METHOD_NOT_FOUND,
            message: Cow::Owned(format!("Method not found: {method}")),
            data: None,
        }
    }

    pub fn invalid_params(msg: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code: Self::INVALID_PARAMS,
            message: msg.into(),
            data: None,
        }
    }

    pub fn internal_error(msg: impl Into<Cow<'static, str>>) -> Self {
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
    ///
    /// # Errors
    ///
    /// Returns error if base64 data is invalid.
    pub fn into_tarpc(self) -> Result<TarpcWorkloadSubmission, String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let data = STANDARD
            .decode(&self.data)
            .map_err(|e| format!("Invalid base64 data: {e}"))?;

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

#[cfg(test)]
mod proptest_tests {
    use super::JsonRpcRequest;
    use proptest::prelude::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct JsonRpcRequestBuilder {
        jsonrpc: String,
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<serde_json::Value>,
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_jsonrpc_request_parse_method_params_id(
            method in "[a-zA-Z0-9_.]{1,60}",
            id_num in prop::option::of(0i64..10000i64),
        ) {
            let builder = JsonRpcRequestBuilder {
                jsonrpc: "2.0".to_string(),
                method: method.clone(),
                params: Some(serde_json::json!({"key": "value", "n": 42})),
                id: id_num.map(|n| serde_json::json!(n)),
            };
            let json = serde_json::to_string(&builder).unwrap();
            let parsed: JsonRpcRequest<'_> = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.jsonrpc.as_ref(), "2.0");
            prop_assert_eq!(parsed.method.as_ref(), method);
            prop_assert!(parsed.params.is_some());
        }

        #[test]
        fn prop_jsonrpc_request_parse_string_id(
            method in "[a-z_]+",
            id_str in "[a-zA-Z0-9_-]{1,30}",
        ) {
            let builder = JsonRpcRequestBuilder {
                jsonrpc: "2.0".to_string(),
                method: method.clone(),
                params: None,
                id: Some(serde_json::Value::String(id_str.clone())),
            };
            let json = serde_json::to_string(&builder).unwrap();
            let parsed: JsonRpcRequest<'_> = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.method.as_ref(), method);
            prop_assert!(parsed.id.is_some());
        }

        #[test]
        fn prop_jsonrpc_request_parse_array_params(
            method in "[a-z.]+",
        ) {
            let builder = JsonRpcRequestBuilder {
                jsonrpc: "2.0".to_string(),
                method: method.clone(),
                params: Some(serde_json::json!([1, 2, "three"])),
                id: Some(serde_json::json!(1)),
            };
            let json = serde_json::to_string(&builder).unwrap();
            let parsed: JsonRpcRequest<'_> = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(parsed.method.as_ref(), method);
            prop_assert!(parsed.params.is_some());
        }
    }
}
