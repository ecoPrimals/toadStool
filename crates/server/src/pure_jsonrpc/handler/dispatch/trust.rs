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
#[must_use]
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
        "verified": matches!(
            caller_ctx.trust_level,
            DispatchTrustLevel::BtspVerified | DispatchTrustLevel::MutuallyAuthenticated
        ),
        "local_gate_id": crate::pure_jsonrpc::handler::resolve_local_gate_id()
            .unwrap_or_else(|| PRIMAL_NAME.to_owned()),
        "btsp_required": is_btsp_required_for_dispatch(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_trust_anonymous_caller() {
        let ctx = CallerContext::anonymous();
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "anonymous");
        assert!(result["gate_id"].is_null());
        assert_eq!(result["verified"], false);
        assert!(result.get("btsp_required").is_some());
        assert!(result.get("local_gate_id").is_some());
    }

    #[test]
    fn verify_trust_local_transport() {
        let ctx = CallerContext {
            gate_id: Some("strandGate".into()),
            trust_level: DispatchTrustLevel::LocalTransport,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "local_transport");
        assert_eq!(result["gate_id"], "strandGate");
        assert_eq!(result["verified"], false);
    }

    #[test]
    fn verify_trust_btsp_verified() {
        let ctx = CallerContext {
            gate_id: Some("eastGate".into()),
            trust_level: DispatchTrustLevel::BtspVerified,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "btsp_verified");
        assert_eq!(result["gate_id"], "eastGate");
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn verify_trust_mutually_authenticated() {
        let ctx = CallerContext {
            gate_id: Some("biomeGate".into()),
            trust_level: DispatchTrustLevel::MutuallyAuthenticated,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "mutually_authenticated");
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn verify_trust_with_requested_gate_id() {
        let ctx = CallerContext::anonymous();
        let params = serde_json::json!({"gate_id": "target-gate-42"});
        let result = verify_trust(&ctx, Some(&params));
        assert_eq!(result["requested_gate_id"], "target-gate-42");
        assert_eq!(result["verified"], false);
    }

    #[test]
    fn verify_trust_no_params_requested_gate_is_null() {
        let ctx = CallerContext::anonymous();
        let result = verify_trust(&ctx, None);
        assert!(result["requested_gate_id"].is_null());
    }
}
