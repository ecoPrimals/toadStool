// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection handling for Pure JSON-RPC server (Unix socket + TCP)
//!
//! Generic over JsonRpcHandler. Parses requests from owned bytes so that
//! JsonRpcRequest's Cow<'a, str> can borrow from the slice during deserialization.

mod tcp;
#[cfg(test)]
mod tests;
mod unix;

pub use tcp::serve_tcp;
pub use unix::{prebind_unix_listener, serve_unix, serve_unix_prebound, spawn_early_health_responder};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::types::JsonRpcError;
use crate::pure_jsonrpc::{JsonRpcHandler, JsonRpcRequest, JsonRpcResponse};

/// Parse request from body bytes, dispatch to handler, return serialized response.
///
/// Uses owned body so JsonRpcRequest can borrow from it via `serde_json::from_slice`.
#[cfg_attr(test, allow(dead_code))]
pub async fn process_request(handler: &JsonRpcHandler, body: &[u8]) -> ServerResult<Vec<u8>> {
    let request: JsonRpcRequest = match serde_json::from_slice(body) {
        Ok(r) => r,
        Err(e) => {
            let response = JsonRpcResponse {
                jsonrpc: std::borrow::Cow::Borrowed(toadstool_common::constants::jsonrpc::VERSION),
                result: None,
                error: Some(JsonRpcError::parse_error(format!("Parse error: {e}"))),
                id: serde_json::Value::Null,
            };
            return serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()));
        }
    };

    let response = handler.handle_request(&request).await;

    serde_json::to_vec(&response).map_err(|e| ServerError::Internal(e.to_string()))
}
