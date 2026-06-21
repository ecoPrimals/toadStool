// SPDX-License-Identifier: AGPL-3.0-or-later
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
    assert!(
        gate.check_with_context("compute.dispatch.submit", &ctx)
            .is_ok()
    );
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
    assert!(gate.check_with_context("shader.dispatch", &ctx).is_ok());
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
        method_allowlist: vec!["compute.dispatch.submit".into(), "shader.dispatch".into()],
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
    assert!(gate.check_with_context("capabilities.list", &ctx).is_ok());
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
    assert!(
        gate.check_with_context("compute.performance_surface.report", &ctx)
            .is_ok()
    );
}

#[test]
fn permissive_without_envelope_allows_restricted_methods() {
    let gate = MethodGate::permissive();
    let ctx = CallerContext::anonymous();
    assert!(gate.check_with_context("gate.update", &ctx).is_ok());
    assert!(gate.check_with_context("transport.open", &ctx).is_ok());
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
    assert_eq!(
        classify_method("auth.custom_probe"),
        MethodVisibility::Public
    );
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
    const {
        assert!(ConnectionTrustHints::UNIX_BTSP.btsp_verified);
        assert!(ConnectionTrustHints::UNIX_MUTUAL_BTSP.mutually_authenticated);
    }
    assert_eq!(
        ConnectionTrustHints::TCP.transport,
        ConnectionTransport::Tcp
    );
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
    let err = gate.check_with_context("compute.submit", &ctx).unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::UNAUTHORIZED
    );
}
