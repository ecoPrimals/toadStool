// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC proxy for `ai.nautilus.*` namespace.
//!
//! ToadStool forwards `ai.nautilus.*` requests to the compute capability provider
//! (network/compute stack) via Unix JSON-RPC. ToadStool discovers the `"compute"`
//! socket and proxies the call; it does not hardcode a specific peer implementation.

use serde_json::Value;
use toadstool_common::primal_sockets::get_socket_path_for_capability;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

pub(super) struct NautilusRpcError {
    pub code: i32,
    pub message: String,
}

impl NautilusRpcError {
    fn proxy_error(msg: impl Into<String>) -> Self {
        Self {
            code: toadstool_common::constants::jsonrpc::error_codes::INTERNAL_ERROR,
            message: msg.into(),
        }
    }
}

/// Forward an `ai.nautilus.*` method to the compute capability provider via Unix JSON-RPC.
///
/// The peer is discovered at runtime via the `"compute"` capability socket.
/// Method name and params are forwarded verbatim.
pub async fn proxy_nautilus_rpc(method: &str, params: &Value) -> Result<Value, NautilusRpcError> {
    let socket_path = get_socket_path_for_capability("compute");

    let client = UnixJsonRpcClient::new(socket_path);

    client.call(method, params.clone()).await.map_err(|e| {
        NautilusRpcError::proxy_error(format!("compute capability provider unreachable: {e}"))
    })
}

/// Legacy name — prefer [`proxy_nautilus_rpc`].
#[allow(dead_code)] // Kept for API compatibility; default call path uses `proxy_nautilus_rpc`.
pub async fn proxy_to_barracuda(method: &str, params: &Value) -> Result<Value, NautilusRpcError> {
    proxy_nautilus_rpc(method, params).await
}

#[cfg(test)]
#[cfg(feature = "nautilus")]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxy_nautilus_rpc_unreachable() {
        let params = serde_json::json!({});
        let result = proxy_nautilus_rpc("ai.nautilus.test", &params).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("unreachable") || err.message.contains("compute"));
    }
}
