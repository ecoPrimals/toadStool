// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC 2.0 protocol constants
//!
//! Standard JSON-RPC 2.0 error codes per specification:
//! <https://www.jsonrpc.org/specification#error_object>

/// JSON-RPC protocol version string
pub const VERSION: &str = "2.0";

/// Standard JSON-RPC 2.0 error codes
///
/// Per specification: <https://www.jsonrpc.org/specification#error_object>
pub mod error_codes {
    /// Parse error: Invalid JSON was received by the server
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request: The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found: The method does not exist / is not available
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params: Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error: Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Server error range: Reserved for implementation-defined server-errors
    pub const SERVER_ERROR_RANGE_START: i32 = -32099;
    /// Server error range end
    pub const SERVER_ERROR_RANGE_END: i32 = -32000;

    // Application-specific error codes (within server error range)

    /// Workload not found
    pub const WORKLOAD_NOT_FOUND: i32 = -32000;

    /// Workload submission failed
    pub const WORKLOAD_SUBMIT_FAILED: i32 = -32001;

    /// Workload deletion failed
    pub const WORKLOAD_DELETE_FAILED: i32 = -32002;

    /// Capability not available
    pub const CAPABILITY_NOT_AVAILABLE: i32 = -32003;

    /// Resource exhausted
    pub const RESOURCE_EXHAUSTED: i32 = -32004;

    /// Authentication required (caller has no identity/token)
    pub const AUTH_REQUIRED: i32 = -32005;

    /// Permission denied (caller authenticated but lacks access to method/resource)
    pub const PERMISSION_DENIED: i32 = -32006;
}
