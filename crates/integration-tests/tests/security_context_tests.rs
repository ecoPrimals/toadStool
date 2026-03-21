// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unnecessary_wraps)]
//! Security context validation tests
//!
//! Tests use the real `SecurityContext` API: `isolation_level`, capabilities
//! (Capability enum), `user_context`, `network_security`, `filesystem_security`.

use toadstool::{Capability, IsolationLevel, SecurityContext, SecuritySettings, ToadStoolResult};

#[test]
fn test_security_context_default() {
    let context = SecurityContext::default();
    assert!(
        context.validate().is_ok(),
        "Default security context should be valid"
    );
}

#[test]
fn test_security_context_creation() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![Capability::Read, Capability::Write],
        ..SecurityContext::default()
    };
    assert!(context.validate().is_ok());
    assert_eq!(context.capabilities.len(), 2);
    assert!(context.has_permission("read"));
    assert!(context.has_permission("write"));
}

#[test]
fn test_security_settings_default() {
    let settings = SecuritySettings::default();
    assert!(!settings.default_capabilities.is_empty());
}

#[test]
fn test_isolation_levels() {
    let levels = vec![
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
        IsolationLevel::Maximum,
    ];
    for level in levels {
        let context = SecurityContext::for_isolation_level(level);
        assert!(context.validate().is_ok());
    }
}

#[test]
fn test_permission_checking() {
    let context = SecurityContext {
        capabilities: vec![Capability::Read, Capability::Execute],
        ..SecurityContext::default()
    };
    assert!(context.has_permission("read"));
    assert!(context.has_permission("execute"));
    assert!(!context.has_permission("write"));
}

#[test]
fn test_security_context_cloning() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Enhanced,
        capabilities: vec![Capability::Read],
        ..SecurityContext::default()
    };
    let cloned = context.clone();
    assert_eq!(context.isolation_level, cloned.isolation_level);
    assert_eq!(context.capabilities.len(), cloned.capabilities.len());
}

#[test]
fn test_security_context_serialization() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![Capability::Execute, Capability::Read],
        ..SecurityContext::default()
    };
    let json = serde_json::to_string(&context);
    assert!(json.is_ok(), "Security context should serialize");
    if let Ok(json_str) = json {
        let deserialized: Result<SecurityContext, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "Should deserialize security context");
    }
}

#[test]
fn test_strict_security_context() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Maximum,
        capabilities: vec![Capability::Execute], // Minimal capabilities
        ..SecurityContext::default()
    };
    assert!(context.validate().is_ok());
    assert!(
        !context.has_permission("write"),
        "Strict context should not allow write"
    );
}

#[test]
fn test_permissive_security_context() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::None,
        capabilities: vec![
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::NetworkClient,
            Capability::NetworkServer,
        ],
        ..SecurityContext::default()
    };
    // Permissive context should still be valid
    assert!(context.validate().is_ok());
    assert!(
        context.has_permission("*"),
        "Wildcard should match any non-empty context"
    );
}

#[test]
fn test_capability_builder_api() {
    let context = SecurityContext::for_isolation_level(IsolationLevel::Standard)
        .with_capability(Capability::NetworkClient);
    assert!(context.has_capability(&Capability::Execute)); // default from for_isolation_level
    assert!(context.has_capability(&Capability::NetworkClient));
}

fn returns_security_result() -> ToadStoolResult<SecurityContext> {
    Ok(SecurityContext::default())
}

#[test]
fn test_security_result_type() {
    let result = returns_security_result();
    assert!(result.is_ok());
}
