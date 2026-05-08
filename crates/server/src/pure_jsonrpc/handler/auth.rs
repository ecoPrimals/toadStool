// SPDX-License-Identifier: AGPL-3.0-or-later
//! Auth introspection methods (JH-0 ecosystem standard)
//!
//! Three public methods for callers to inspect the gate state:
//! - `auth.check` — would the caller be allowed to call a given method?
//! - `auth.mode` — what gate mode is active?
//! - `auth.peer_info` — what does the server know about the caller?

use super::method_gate::{MethodGate, classify_method};
#[cfg(test)]
use super::method_gate::GateMode;
use crate::pure_jsonrpc::types::JsonRpcError;

/// `auth.check` — test whether a method would be allowed for the current caller.
///
/// Params: `{"method": "compute.dispatch.submit"}`
/// Returns: `{"allowed": true, "visibility": "protected", "mode": "permissive"}`
#[expect(
    clippy::unnecessary_wraps,
    reason = "Result return required for JSON-RPC handler dispatch consistency"
)]
pub fn auth_check(
    gate: &MethodGate,
    params: Option<&serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let method = params
        .and_then(|p| p.get("method"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let visibility = classify_method(method);
    let allowed = gate.check(method).is_ok();

    Ok(serde_json::json!({
        "allowed": allowed,
        "visibility": visibility,
        "mode": gate.mode(),
        "method": method,
    }))
}

/// `auth.mode` — return the current gate operating mode.
///
/// Returns: `{"mode": "permissive"}` or `{"mode": "enforcing"}`
#[expect(
    clippy::unnecessary_wraps,
    reason = "Result return required for JSON-RPC handler dispatch consistency"
)]
pub fn auth_mode(gate: &MethodGate) -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({
        "mode": gate.mode(),
    }))
}

/// `auth.peer_info` — return what the server knows about the caller.
///
/// Currently returns minimal info (no caller context is threaded yet).
/// JH-2 will add token/identity fields once BearDog ships ionic tokens.
///
/// Returns: `{"transport": "unknown", "authenticated": false}`
#[expect(
    clippy::unnecessary_wraps,
    reason = "Result return required for JSON-RPC handler dispatch consistency"
)]
pub fn auth_peer_info() -> Result<serde_json::Value, JsonRpcError> {
    Ok(serde_json::json!({
        "transport": "unknown",
        "authenticated": false,
        "token": null,
        "identity": null,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_check_permissive_allows_protected() {
        let gate = MethodGate::permissive();
        let params = serde_json::json!({"method": "compute.dispatch.submit"});
        let result = auth_check(&gate, Some(&params)).unwrap();
        assert_eq!(result["allowed"], true);
        assert_eq!(result["visibility"], "protected");
        assert_eq!(result["mode"], "permissive");
    }

    #[test]
    fn auth_check_enforcing_denies_protected() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let params = serde_json::json!({"method": "compute.dispatch.submit"});
        let result = auth_check(&gate, Some(&params)).unwrap();
        assert_eq!(result["allowed"], false);
        assert_eq!(result["visibility"], "protected");
        assert_eq!(result["mode"], "enforcing");
    }

    #[test]
    fn auth_check_enforcing_allows_public() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let params = serde_json::json!({"method": "health.liveness"});
        let result = auth_check(&gate, Some(&params)).unwrap();
        assert_eq!(result["allowed"], true);
        assert_eq!(result["visibility"], "public");
    }

    #[test]
    fn auth_check_no_method_param() {
        let gate = MethodGate::permissive();
        let result = auth_check(&gate, None).unwrap();
        assert_eq!(result["allowed"], true);
        assert_eq!(result["method"], "");
    }

    #[test]
    fn auth_mode_returns_current() {
        let gate = MethodGate::permissive();
        let result = auth_mode(&gate).unwrap();
        assert_eq!(result["mode"], "permissive");

        let gate = MethodGate::new(GateMode::Enforcing);
        let result = auth_mode(&gate).unwrap();
        assert_eq!(result["mode"], "enforcing");
    }

    #[test]
    fn auth_peer_info_returns_unknown() {
        let result = auth_peer_info().unwrap();
        assert_eq!(result["transport"], "unknown");
        assert_eq!(result["authenticated"], false);
        assert!(result["token"].is_null());
        assert!(result["identity"].is_null());
    }
}
