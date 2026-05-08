// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pre-dispatch capability gate (JH-0 ecosystem standard)
//!
//! Classifies every JSON-RPC method as [`MethodVisibility::Public`] or
//! [`MethodVisibility::Protected`] and gates dispatch based on the current
//! [`GateMode`]. Ships in [`GateMode::Permissive`] (all calls allowed)
//! per the primalSpring `METHOD_GATE_STANDARD.md` adoption guide.
//!
//! When [`GateMode::Enforcing`] is activated (future, requires BearDog
//! ionic tokens — JH-1/JH-2), protected methods will require a valid
//! caller token with sufficient resource envelope.

use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::pure_jsonrpc::types::JsonRpcError;

/// Whether a method is freely callable or requires authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodVisibility {
    /// Callable by any peer without credentials.
    Public,
    /// Requires a valid token / caller identity when the gate is enforcing.
    Protected,
}

/// Gate operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateMode {
    /// All calls allowed regardless of caller identity (JH-0 default).
    Permissive,
    /// Protected methods require valid authentication (JH-2 future).
    Enforcing,
}

/// Pre-dispatch capability gate.
///
/// Sits between request parsing and method routing. In `Permissive` mode
/// every call passes through. In `Enforcing` mode, protected methods are
/// rejected unless the caller provides valid credentials (not yet wired —
/// blocked on BearDog JH-1 ionic token infrastructure).
pub struct MethodGate {
    mode: GateMode,
}

impl MethodGate {
    /// Create a new gate in the given mode.
    pub fn new(mode: GateMode) -> Self {
        Self { mode }
    }

    /// Create a gate in permissive mode (JH-0 default).
    pub fn permissive() -> Self {
        Self::new(GateMode::Permissive)
    }

    /// Current operating mode.
    pub fn mode(&self) -> GateMode {
        self.mode
    }

    /// Check whether a method call should be allowed.
    ///
    /// In `Permissive` mode this always returns `Ok(())`.
    /// In `Enforcing` mode, `Protected` methods are rejected with
    /// `PERMISSION_DENIED` (-32006). Future: caller context / token
    /// verification will refine this to per-caller decisions.
    pub fn check(&self, method: &str) -> Result<(), JsonRpcError> {
        let visibility = classify_method(method);

        trace!(
            method,
            ?visibility,
            mode = ?self.mode,
            "method gate check"
        );

        match self.mode {
            GateMode::Permissive => Ok(()),
            GateMode::Enforcing => match visibility {
                MethodVisibility::Public => Ok(()),
                MethodVisibility::Protected => Err(JsonRpcError::permission_denied(method)),
            },
        }
    }
}

/// Classify a method name into its visibility tier.
///
/// Public methods are introspection / health / identity / auth — always
/// callable. Everything else (dispatch, workloads, hardware, transport,
/// gate routing) is protected.
pub fn classify_method(method: &str) -> MethodVisibility {
    match method {
        // Health probes — always public (PG-62 fast-path must not be gated)
        "health.liveness" | "health.readiness" | "health.check" | "toadstool.health"
        | "compute.health" => MethodVisibility::Public,

        // Identity and capabilities — introspection is always public
        "identity.get" | "toadstool.version" | "compute.version" | "capabilities.list"
        | "capability.list" | "primal.capabilities" | "compute.capabilities"
        | "compute.discover_capabilities" | "toadstool.query_capabilities" => {
            MethodVisibility::Public
        }

        // Provenance — read-only introspection
        "provenance.query" | "provenance.get" | "toadstool.provenance" => {
            MethodVisibility::Public
        }

        // Auth methods — must be public so callers can check their own status
        m if m.starts_with("auth.") => MethodVisibility::Public,

        // Everything else is protected
        _ => MethodVisibility::Protected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_mode_allows_all() {
        let gate = MethodGate::permissive();
        assert!(gate.check("compute.dispatch.submit").is_ok());
        assert!(gate.check("shader.dispatch").is_ok());
        assert!(gate.check("health.liveness").is_ok());
        assert!(gate.check("some.unknown.method").is_ok());
    }

    #[test]
    fn enforcing_mode_allows_public() {
        let gate = MethodGate::new(GateMode::Enforcing);
        assert!(gate.check("health.liveness").is_ok());
        assert!(gate.check("health.readiness").is_ok());
        assert!(gate.check("health.check").is_ok());
        assert!(gate.check("identity.get").is_ok());
        assert!(gate.check("capabilities.list").is_ok());
        assert!(gate.check("toadstool.version").is_ok());
        assert!(gate.check("toadstool.health").is_ok());
        assert!(gate.check("provenance.query").is_ok());
        assert!(gate.check("auth.check").is_ok());
        assert!(gate.check("auth.mode").is_ok());
        assert!(gate.check("auth.peer_info").is_ok());
    }

    #[test]
    fn enforcing_mode_denies_protected() {
        let gate = MethodGate::new(GateMode::Enforcing);

        let err = gate.check("compute.dispatch.submit").unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::PERMISSION_DENIED
        );

        assert!(gate.check("shader.dispatch").is_err());
        assert!(gate.check("compute.execute").is_err());
        assert!(gate.check("compute.hardware.observe").is_err());
        assert!(gate.check("gate.update").is_err());
        assert!(gate.check("transport.open").is_err());
    }

    #[test]
    fn classify_covers_all_public_methods() {
        let public_methods = [
            "health.liveness",
            "health.readiness",
            "health.check",
            "toadstool.health",
            "compute.health",
            "identity.get",
            "toadstool.version",
            "compute.version",
            "capabilities.list",
            "capability.list",
            "primal.capabilities",
            "compute.capabilities",
            "compute.discover_capabilities",
            "toadstool.query_capabilities",
            "provenance.query",
            "provenance.get",
            "toadstool.provenance",
            "auth.check",
            "auth.mode",
            "auth.peer_info",
        ];
        for m in &public_methods {
            assert_eq!(
                classify_method(m),
                MethodVisibility::Public,
                "{m} should be Public"
            );
        }
    }

    #[test]
    fn classify_protected_methods() {
        let protected_methods = [
            "compute.dispatch.submit",
            "compute.dispatch.status",
            "compute.dispatch.result",
            "shader.dispatch",
            "compute.execute",
            "compute.submit",
            "compute.cancel",
            "toadstool.submit_workload",
            "toadstool.cancel_workload",
            "compute.hardware.observe",
            "compute.hardware.apply",
            "gate.update",
            "gate.remove",
            "transport.open",
            "transport.stream",
            "ember.list",
            "compute.performance_surface.report",
        ];
        for m in &protected_methods {
            assert_eq!(
                classify_method(m),
                MethodVisibility::Protected,
                "{m} should be Protected"
            );
        }
    }

    #[test]
    fn gate_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&GateMode::Permissive).unwrap(),
            "\"permissive\""
        );
        assert_eq!(
            serde_json::to_string(&GateMode::Enforcing).unwrap(),
            "\"enforcing\""
        );
    }
}
