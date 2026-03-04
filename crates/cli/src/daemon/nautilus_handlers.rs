// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC proxy for `ai.nautilus.*` namespace.
//!
//! ToadStool forwards nautilus requests to barraCuda via capability-based IPC.
//! Nautilus (evolutionary reservoir computing) is math — it belongs to barraCuda.
//! ToadStool is a thin proxy: discover barraCuda by "compute" capability, forward
//! the JSON-RPC call, return the response.

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

/// Forward a `ai.nautilus.*` method to barraCuda via capability-based IPC.
///
/// barraCuda is discovered at runtime via the "compute" capability socket.
/// The method name and params are forwarded verbatim.
pub async fn proxy_to_barracuda(method: &str, params: &Value) -> Result<Value, NautilusRpcError> {
    let socket_path = get_socket_path_for_capability("compute");

    let client = UnixJsonRpcClient::new(socket_path);

    client
        .call(method, params.clone())
        .await
        .map_err(|e| NautilusRpcError::proxy_error(format!("barraCuda unreachable: {e}")))
}
