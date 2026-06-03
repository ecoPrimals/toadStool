// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_common::constants::primal_identity::PRIMAL_NAME;
use toadstool_common::primal_sockets::{SocketPathEnv, is_btsp_required};

use crate::pure_jsonrpc::handler::method_gate::{CallerContext, DispatchTrustLevel};

/// Whether production dispatch requires a completed BTSP handshake.
#[must_use]
pub fn is_btsp_required_for_dispatch() -> bool {
    is_btsp_required(&SocketPathEnv::from_env())
}

/// Verify trust level for a dispatch request.
///
/// Returns the trust assessment without executing any dispatch.
/// Used by remote gates to pre-validate before forwarding workloads.
pub fn verify_trust(
    caller_ctx: &CallerContext,
    params: Option<&serde_json::Value>,
) -> serde_json::Value {
    let requested_gate_id = params
        .and_then(|p| p.get("gate_id"))
        .and_then(serde_json::Value::as_str);

    serde_json::json!({
        "trust_level": caller_ctx.trust_level,
        "gate_id": caller_ctx.gate_id,
        "requested_gate_id": requested_gate_id,
        "verified": caller_ctx.trust_level != DispatchTrustLevel::Anonymous,
        "local_gate_id": PRIMAL_NAME,
        "btsp_required": is_btsp_required_for_dispatch(),
    })
}
