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

    #[test]
    fn verify_trust_forged_btsp_no_gate_id() {
        // A caller claims BtspVerified but provides no gate_id — suspicious.
        // The handler should still report verified=true (trust is from transport
        // layer, not from gate_id presence), but gate_id should be null.
        let ctx = CallerContext {
            gate_id: None,
            trust_level: DispatchTrustLevel::BtspVerified,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "btsp_verified");
        assert!(result["gate_id"].is_null());
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn verify_trust_forged_mutual_auth_no_gate_id() {
        // MutuallyAuthenticated without gate_id is even more suspicious.
        let ctx = CallerContext {
            gate_id: None,
            trust_level: DispatchTrustLevel::MutuallyAuthenticated,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "mutually_authenticated");
        assert!(result["gate_id"].is_null());
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn verify_trust_gate_id_mismatch_with_requested() {
        // Caller has gate_id "eastGate" but requests dispatch to "biomeGate".
        // verify_trust should report both and let the caller decide.
        let ctx = CallerContext {
            gate_id: Some("eastGate".into()),
            trust_level: DispatchTrustLevel::BtspVerified,
            ..CallerContext::anonymous()
        };
        let params = serde_json::json!({"gate_id": "biomeGate"});
        let result = verify_trust(&ctx, Some(&params));
        assert_eq!(result["gate_id"], "eastGate");
        assert_eq!(result["requested_gate_id"], "biomeGate");
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn verify_trust_anonymous_with_gate_id() {
        // Caller is Anonymous trust level but still has a gate_id
        // (e.g., extracted from connection metadata without BTSP).
        // Should NOT be verified.
        let ctx = CallerContext {
            gate_id: Some("rogue-gate".into()),
            trust_level: DispatchTrustLevel::Anonymous,
            ..CallerContext::anonymous()
        };
        let result = verify_trust(&ctx, None);
        assert_eq!(result["trust_level"], "anonymous");
        assert_eq!(result["gate_id"], "rogue-gate");
        assert_eq!(result["verified"], false);
    }

    #[test]
    fn verify_trust_malformed_params_non_string_gate_id() {
        // Caller passes gate_id as a number instead of string.
        let ctx = CallerContext::anonymous();
        let params = serde_json::json!({"gate_id": 42});
        let result = verify_trust(&ctx, Some(&params));
        assert!(result["requested_gate_id"].is_null());
    }

    #[test]
    fn verify_trust_empty_params_object() {
        let ctx = CallerContext::anonymous();
        let params = serde_json::json!({});
        let result = verify_trust(&ctx, Some(&params));
        assert!(result["requested_gate_id"].is_null());
        assert_eq!(result["verified"], false);
    }

    #[test]
    fn verify_trust_extra_unknown_params_ignored() {
        // Unknown fields in params should not cause errors.
        let ctx = CallerContext {
            gate_id: Some("legit-gate".into()),
            trust_level: DispatchTrustLevel::BtspVerified,
            ..CallerContext::anonymous()
        };
        let params = serde_json::json!({
            "gate_id": "target",
            "unknown_field": "should be ignored",
            "another_junk": [1, 2, 3]
        });
        let result = verify_trust(&ctx, Some(&params));
        assert_eq!(result["requested_gate_id"], "target");
        assert_eq!(result["verified"], true);
    }

    #[test]
    fn all_trust_levels_serialize_to_snake_case() {
        let levels = [
            (DispatchTrustLevel::Anonymous, "anonymous"),
            (DispatchTrustLevel::LocalTransport, "local_transport"),
            (DispatchTrustLevel::BtspVerified, "btsp_verified"),
            (
                DispatchTrustLevel::MutuallyAuthenticated,
                "mutually_authenticated",
            ),
        ];
        for (level, expected) in levels {
            let ctx = CallerContext {
                trust_level: level,
                ..CallerContext::anonymous()
            };
            let result = verify_trust(&ctx, None);
            assert_eq!(result["trust_level"], expected, "trust level {expected}");
        }
    }
}
