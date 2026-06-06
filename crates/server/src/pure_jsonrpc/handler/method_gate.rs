// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pre-dispatch capability gate (JH-0 / JH-2 ecosystem standard)
//!
//! Classifies every JSON-RPC method as [`MethodVisibility::Public`] or
//! [`MethodVisibility::Protected`] and gates dispatch based on the current
//! [`GateMode`]. Ships in [`GateMode::Permissive`] (all calls allowed)
//! per the primalSpring `METHOD_GATE_STANDARD.md` adoption guide.
//!
//! JH-2 adds [`ResourceEnvelope`] enforcement: ionic tokens carry resource
//! limits (`mem_mb`, `cpu_cores`, `method_allowlist`) that are checked at
//! dispatch time. When no token is present and the gate is permissive,
//! dispatch proceeds without limits (backward compatible).

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

/// Resource limits carried in an ionic token (JH-2).
///
/// When present, dispatch handlers enforce that the requested resources
/// fall within these bounds. Fields are optional — `None` means
/// "unlimited for this dimension".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEnvelope {
    /// Maximum memory in MB the token grants for a single dispatch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_mb: Option<u64>,
    /// Maximum CPU cores the token grants.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    /// Maximum timeout in milliseconds the token permits per dispatch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_timeout_ms: Option<u64>,
    /// Methods this token is allowed to call. Empty means "all methods allowed".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub method_allowlist: Vec<String>,
}

impl ResourceEnvelope {
    /// Check whether the envelope allows calling `method`.
    ///
    /// An empty allowlist means "all methods permitted".
    pub fn allows_method(&self, method: &str) -> bool {
        self.method_allowlist.is_empty() || self.method_allowlist.iter().any(|m| m == method)
    }
}

/// Trust level established for a dispatch request (Dark Forest Invariant 3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTrustLevel {
    #[default]
    Anonymous,
    LocalTransport,
    BtspVerified,
    MutuallyAuthenticated,
}

/// Per-connection transport hints for provenance extraction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectionTrustHints {
    pub transport: ConnectionTransport,
    pub btsp_verified: bool,
    /// Completed mutual BTSP handshake (BearDog JH-1).
    pub mutually_authenticated: bool,
}

/// Transport kind for the active JSON-RPC connection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectionTransport {
    #[default]
    Unknown,
    Unix,
    Tcp,
}

impl ConnectionTrustHints {
    pub const UNIX_LOCAL: Self = Self {
        transport: ConnectionTransport::Unix,
        btsp_verified: false,
        mutually_authenticated: false,
    };
    pub const UNIX_BTSP: Self = Self {
        transport: ConnectionTransport::Unix,
        btsp_verified: true,
        mutually_authenticated: false,
    };
    pub const UNIX_MUTUAL_BTSP: Self = Self {
        transport: ConnectionTransport::Unix,
        btsp_verified: true,
        mutually_authenticated: true,
    };
    pub const TCP: Self = Self {
        transport: ConnectionTransport::Tcp,
        btsp_verified: false,
        mutually_authenticated: false,
    };
}

/// Caller identity and resource context extracted from a request (JH-2).
///
/// Threaded through the dispatch path so that handlers can enforce
/// per-caller resource limits. `None` fields mean "no token / unknown".
#[derive(Debug, Clone, Default)]
pub struct CallerContext {
    /// Caller identity (e.g. DID from ionic token). `None` = anonymous.
    pub identity: Option<String>,
    /// Resource envelope from the ionic token. `None` = no token presented.
    pub envelope: Option<ResourceEnvelope>,
    /// Gate identity of the requesting node (from BTSP session or gate.update).
    pub gate_id: Option<String>,
    /// Trust level established during BTSP handshake.
    pub trust_level: DispatchTrustLevel,
}

impl CallerContext {
    /// Anonymous caller with no token (permissive-mode default).
    pub fn anonymous() -> Self {
        Self {
            gate_id: None,
            trust_level: DispatchTrustLevel::Anonymous,
            ..Self::default()
        }
    }

    /// Whether this caller presented a token with an envelope.
    pub fn has_envelope(&self) -> bool {
        self.envelope.is_some()
    }
}

/// Pre-dispatch capability gate.
///
/// Sits between request parsing and method routing. In `Permissive` mode
/// every call passes through. In `Enforcing` mode, protected methods
/// require valid credentials and the caller's [`ResourceEnvelope`]
/// constraints are checked.
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

    /// Check whether a method call should be allowed (JH-0 basic check).
    ///
    /// In `Permissive` mode this always returns `Ok(())`.
    /// In `Enforcing` mode, `Protected` methods are rejected with
    /// `PERMISSION_DENIED` (-32001, ecosystem standard).
    pub fn check(&self, method: &str) -> Result<(), JsonRpcError> {
        self.check_with_context(method, &CallerContext::anonymous())
    }

    /// Check method access with full caller context (JH-2).
    ///
    /// In `Enforcing` mode:
    /// - Anonymous callers are rejected on `Protected` methods with `UNAUTHORIZED`.
    /// - Callers with a token are checked against their envelope's `method_allowlist`.
    ///
    /// In `Permissive` mode: always allowed unless the token itself restricts the method.
    pub fn check_with_context(
        &self,
        method: &str,
        ctx: &CallerContext,
    ) -> Result<(), JsonRpcError> {
        let visibility = classify_method(method);

        trace!(
            method,
            ?visibility,
            mode = ?self.mode,
            has_identity = ctx.identity.is_some(),
            has_envelope = ctx.has_envelope(),
            "method gate check"
        );

        match self.mode {
            GateMode::Permissive => {
                if let Some(ref env) = ctx.envelope
                    && !env.allows_method(method)
                {
                    return Err(JsonRpcError::permission_denied(method));
                }
                Ok(())
            }
            GateMode::Enforcing => match visibility {
                MethodVisibility::Public => Ok(()),
                MethodVisibility::Protected => {
                    if ctx.identity.is_none() {
                        return Err(JsonRpcError::unauthorized(
                            "Authentication required for protected method",
                        ));
                    }
                    if let Some(ref env) = ctx.envelope
                        && !env.allows_method(method)
                    {
                        return Err(JsonRpcError::permission_denied(method));
                    }
                    Ok(())
                }
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
        "health.liveness" | "health.readiness" | "health.check" | "health.version"
        | "health.drain" | "toadstool.health" | "compute.health" => MethodVisibility::Public,

        // Identity and capabilities — introspection is always public
        "identity.get" | "primal.announce" | "toadstool.version" | "compute.version"
        | "capabilities.list" | "capability.list" | "primal.capabilities"
        | "compute.capabilities" | "compute.discover_capabilities"
        | "toadstool.query_capabilities" => {
            MethodVisibility::Public
        }

        // Provenance — read-only introspection
        "provenance.query" | "provenance.get" | "toadstool.provenance" => {
            MethodVisibility::Public
        }

        // Dispatch telemetry schema — public introspection for ml.mlp_train
        "dispatch.telemetry.schema" => MethodVisibility::Public,

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
        assert!(gate.check("dispatch.telemetry.schema").is_ok());
    }

    #[test]
    fn enforcing_mode_denies_anonymous_on_protected() {
        let gate = MethodGate::new(GateMode::Enforcing);

        let err = gate.check("compute.dispatch.submit").unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::UNAUTHORIZED
        );

        assert!(gate.check("shader.dispatch").is_err());
        assert!(gate.check("compute.execute").is_err());
        assert!(gate.check("compute.hardware.observe").is_err());
        assert!(gate.check("gate.update").is_err());
        assert!(gate.check("transport.open").is_err());
    }

    #[test]
    fn enforcing_mode_allows_authenticated_caller() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(ResourceEnvelope::default()),
            ..CallerContext::anonymous()
        };
        assert!(gate
            .check_with_context("compute.dispatch.submit", &ctx)
            .is_ok());
    }

    #[test]
    fn enforcing_mode_denies_method_not_in_allowlist() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(ResourceEnvelope {
                method_allowlist: vec!["shader.dispatch".into()],
                ..ResourceEnvelope::default()
            }),
            ..CallerContext::anonymous()
        };
        let err = gate
            .check_with_context("compute.dispatch.submit", &ctx)
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::PERMISSION_DENIED
        );
    }

    #[test]
    fn permissive_mode_still_enforces_allowlist_from_token() {
        let gate = MethodGate::permissive();
        let ctx = CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(ResourceEnvelope {
                method_allowlist: vec!["shader.dispatch".into()],
                ..ResourceEnvelope::default()
            }),
            ..CallerContext::anonymous()
        };
        let err = gate
            .check_with_context("compute.dispatch.submit", &ctx)
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::PERMISSION_DENIED
        );
        assert!(gate
            .check_with_context("shader.dispatch", &ctx)
            .is_ok());
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
            "dispatch.telemetry.schema",
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
            "dispatch.verify_trust",
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

    #[test]
    fn resource_envelope_allows_method_empty_allowlist() {
        let env = ResourceEnvelope::default();
        assert!(env.allows_method("anything"));
    }

    #[test]
    fn resource_envelope_allows_method_in_list() {
        let env = ResourceEnvelope {
            method_allowlist: vec![
                "compute.dispatch.submit".into(),
                "shader.dispatch".into(),
            ],
            ..ResourceEnvelope::default()
        };
        assert!(env.allows_method("compute.dispatch.submit"));
        assert!(env.allows_method("shader.dispatch"));
        assert!(!env.allows_method("compute.cancel"));
    }

    #[test]
    fn resource_envelope_serde_roundtrip() {
        let env = ResourceEnvelope {
            mem_mb: Some(4096),
            cpu_cores: Some(8),
            max_timeout_ms: Some(30_000),
            method_allowlist: vec!["compute.dispatch.submit".into()],
        };
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["mem_mb"], 4096);
        assert_eq!(json["cpu_cores"], 8);
        assert_eq!(json["max_timeout_ms"], 30_000);
        let back: ResourceEnvelope = serde_json::from_value(json).unwrap();
        assert_eq!(env, back);
    }

    #[test]
    fn resource_envelope_serde_skips_none_fields() {
        let env = ResourceEnvelope::default();
        let json = serde_json::to_value(&env).unwrap();
        assert!(json.get("mem_mb").is_none());
        assert!(json.get("cpu_cores").is_none());
        assert!(json.get("method_allowlist").is_none());
    }

    #[test]
    fn caller_context_anonymous_has_no_envelope() {
        let ctx = CallerContext::anonymous();
        assert!(ctx.identity.is_none());
        assert!(!ctx.has_envelope());
        assert!(ctx.gate_id.is_none());
        assert_eq!(ctx.trust_level, DispatchTrustLevel::Anonymous);
    }

    #[test]
    fn gate_mode_accessor_returns_constructor_mode() {
        let permissive = MethodGate::permissive();
        assert_eq!(permissive.mode(), GateMode::Permissive);
        let enforcing = MethodGate::new(GateMode::Enforcing);
        assert_eq!(enforcing.mode(), GateMode::Enforcing);
    }

    #[test]
    fn gate_mode_transition_permissive_to_enforcing() {
        let gate = MethodGate::permissive();
        assert!(gate.check("compute.submit").is_ok());

        let gate = MethodGate::new(GateMode::Enforcing);
        assert!(gate.check("compute.submit").is_err());
        assert!(gate.check("health.liveness").is_ok());
    }

    #[test]
    fn enforcing_public_method_ignores_missing_identity() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext::anonymous();
        assert!(gate
            .check_with_context("capabilities.list", &ctx)
            .is_ok());
    }

    #[test]
    fn enforcing_authenticated_empty_allowlist_allows_protected() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(ResourceEnvelope::default()),
            ..CallerContext::anonymous()
        };
        assert!(gate.check_with_context("compute.cancel", &ctx).is_ok());
        assert!(gate
            .check_with_context("compute.performance_surface.report", &ctx)
            .is_ok());
    }

    #[test]
    fn permissive_without_envelope_allows_restricted_methods() {
        let gate = MethodGate::permissive();
        let ctx = CallerContext::anonymous();
        assert!(gate
            .check_with_context("gate.update", &ctx)
            .is_ok());
        assert!(gate
            .check_with_context("transport.open", &ctx)
            .is_ok());
    }

    #[test]
    fn allowlist_exact_match_required() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext {
            identity: Some("did:key:z6Mk_test".into()),
            envelope: Some(ResourceEnvelope {
                method_allowlist: vec!["compute.submit".into()],
                ..ResourceEnvelope::default()
            }),
            ..CallerContext::anonymous()
        };
        assert!(gate.check_with_context("compute.submit", &ctx).is_ok());
        let err = gate
            .check_with_context("compute.submit.extra", &ctx)
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::PERMISSION_DENIED
        );
    }

    #[test]
    fn classify_auth_prefix_methods_are_public() {
        assert_eq!(classify_method("auth.check"), MethodVisibility::Public);
        assert_eq!(classify_method("auth.custom_probe"), MethodVisibility::Public);
    }

    #[test]
    fn caller_context_has_envelope_when_present() {
        let ctx = CallerContext {
            envelope: Some(ResourceEnvelope {
                mem_mb: Some(1024),
                ..ResourceEnvelope::default()
            }),
            ..CallerContext::anonymous()
        };
        assert!(ctx.has_envelope());
    }

    #[test]
    fn connection_trust_hints_constants() {
        assert_eq!(
            ConnectionTrustHints::UNIX_LOCAL.transport,
            ConnectionTransport::Unix
        );
        assert!(ConnectionTrustHints::UNIX_BTSP.btsp_verified);
        assert!(ConnectionTrustHints::UNIX_MUTUAL_BTSP.mutually_authenticated);
        assert_eq!(ConnectionTrustHints::TCP.transport, ConnectionTransport::Tcp);
    }

    #[test]
    fn dispatch_trust_level_serde_roundtrip() {
        for level in [
            DispatchTrustLevel::Anonymous,
            DispatchTrustLevel::LocalTransport,
            DispatchTrustLevel::BtspVerified,
            DispatchTrustLevel::MutuallyAuthenticated,
        ] {
            let json = serde_json::to_value(level).unwrap();
            let back: DispatchTrustLevel = serde_json::from_value(json).unwrap();
            assert_eq!(back, level);
        }
    }

    #[test]
    fn enforcing_rejects_anonymous_even_with_envelope_no_identity() {
        let gate = MethodGate::new(GateMode::Enforcing);
        let ctx = CallerContext {
            envelope: Some(ResourceEnvelope {
                method_allowlist: vec!["compute.submit".into()],
                ..ResourceEnvelope::default()
            }),
            ..CallerContext::anonymous()
        };
        let err = gate
            .check_with_context("compute.submit", &ctx)
            .unwrap_err();
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::UNAUTHORIZED
        );
    }
}
