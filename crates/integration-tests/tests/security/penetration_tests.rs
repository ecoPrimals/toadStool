//! Security penetration tests
//!
//! Exercises the security model's boundary enforcement: capability escalation
//! attempts, invalid context construction, and permission edge cases.

use toadstool::{Capability, IsolationLevel, SecurityContext, SecuritySettings};

// ── Capability boundary tests ────────────────────────────────────────────────

#[test]
fn test_default_context_is_valid() {
    let ctx = SecurityContext::default();
    assert!(ctx.validate().is_ok(), "Default context must be valid");
}

#[test]
fn test_empty_capabilities_fails_validation() {
    // Empty capability set is rejected by validate(): every context must declare
    // at least one capability so intent is explicit (no accidental deny-all grants).
    let ctx = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        capabilities: vec![],
        ..SecurityContext::default()
    };
    assert!(
        ctx.validate().is_err(),
        "A context with no capabilities must fail validation"
    );
}

#[test]
fn test_has_permission_read() {
    let ctx = SecurityContext {
        capabilities: vec![Capability::Read],
        ..SecurityContext::default()
    };
    assert!(ctx.has_permission("read"));
    assert!(!ctx.has_permission("write"));
    assert!(!ctx.has_permission("execute"));
}

#[test]
fn test_has_permission_wildcard() {
    let ctx = SecurityContext {
        capabilities: vec![Capability::Read, Capability::Execute],
        ..SecurityContext::default()
    };
    // Wildcard should match any capability that is present
    assert!(ctx.has_permission("*"), "Wildcard should match any non-empty capability set");
}

#[test]
fn test_wildcard_fails_on_empty_capabilities() {
    let ctx = SecurityContext {
        capabilities: vec![],
        ..SecurityContext::default()
    };
    assert!(
        !ctx.has_permission("*"),
        "Wildcard on empty capability set should return false"
    );
}

#[test]
fn test_unknown_permission_name_custom_capability() {
    let ctx = SecurityContext {
        capabilities: vec![Capability::Custom("audit_logs".to_string())],
        ..SecurityContext::default()
    };
    assert!(ctx.has_permission("audit_logs"), "Custom capability should match by name");
    assert!(!ctx.has_permission("admin"), "Unrelated custom capability should not match");
}

// ── Privilege escalation resistance ──────────────────────────────────────────

#[test]
fn test_cannot_escalate_read_to_write() {
    let ctx = SecurityContext {
        capabilities: vec![Capability::Read],
        ..SecurityContext::default()
    };
    assert!(!ctx.has_permission("write"), "Read-only context must not grant write");
    assert!(!ctx.has_permission("execute"), "Read-only context must not grant execute");
}

#[test]
fn test_process_management_requires_explicit_grant() {
    let ctx = SecurityContext::default();
    // Default context (Execute + Read) must not include ProcessManagement
    assert!(
        !ctx.has_capability(&Capability::ProcessManagement),
        "Default context must not grant ProcessManagement"
    );
}

#[test]
fn test_network_server_requires_explicit_grant() {
    let ctx = SecurityContext::default();
    assert!(
        !ctx.has_capability(&Capability::NetworkServer),
        "Default context must not grant NetworkServer"
    );
}

// ── Isolation level checks ────────────────────────────────────────────────────

#[test]
fn test_isolation_levels_are_distinct() {
    let levels = [
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
        IsolationLevel::Maximum,
    ];
    // Each level is a distinct variant — compare ordinals via debug string
    let names: Vec<String> = levels.iter().map(|l| format!("{l:?}")).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(unique.len(), levels.len(), "All isolation levels must be distinct");
}

#[test]
fn test_for_isolation_level_maximum() {
    let ctx = SecurityContext::for_isolation_level(IsolationLevel::Maximum);
    assert_eq!(ctx.isolation_level, IsolationLevel::Maximum);
    // Maximum isolation provides only Execute, no write/network
    assert!(!ctx.has_capability(&Capability::Write));
    assert!(!ctx.has_capability(&Capability::NetworkClient));
}

#[test]
fn test_validate_read_write_on_non_strict() {
    // Read + Write is valid on non-strict isolation levels
    let ctx = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![Capability::Read, Capability::Write],
        ..SecurityContext::default()
    };
    assert!(ctx.validate().is_ok());
}

// ── SecuritySettings ──────────────────────────────────────────────────────────

#[test]
fn test_security_settings_has_defaults() {
    let settings = SecuritySettings::default();
    assert!(
        !settings.default_capabilities.is_empty(),
        "Default security settings must include at least one capability"
    );
}
