// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Security Policy and Audit
//!
//! Week 17 Sprint: Security Policy, `AuditSettings`, `AuditEvent` tests

use std::collections::HashMap;
use toadstool::security::*;

// ============================================================================
// SecurityPolicy Tests (12 tests)
// ============================================================================

#[test]
fn test_security_policy_creation() {
    let policy = SecurityPolicy {
        name: "default".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Standard,
        allowed_capabilities: vec![Capability::Read, Capability::Execute],
        denied_capabilities: vec![Capability::ProcessManagement],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert_eq!(policy.name, "default");
    assert_eq!(policy.version, "1.0");
}

#[test]
fn test_security_policy_with_allowed_capabilities() {
    let policy = SecurityPolicy {
        name: "permissive".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Basic,
        allowed_capabilities: vec![
            Capability::Read,
            Capability::Write,
            Capability::Execute,
            Capability::NetworkClient,
        ],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert_eq!(policy.allowed_capabilities.len(), 4);
    assert!(policy.denied_capabilities.is_empty());
}

#[test]
fn test_security_policy_with_denied_capabilities() {
    let policy = SecurityPolicy {
        name: "restrictive".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Enhanced,
        allowed_capabilities: vec![Capability::Read],
        denied_capabilities: vec![
            Capability::Write,
            Capability::NetworkClient,
            Capability::ProcessManagement,
        ],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert_eq!(policy.denied_capabilities.len(), 3);
}

#[test]
fn test_security_policy_clone() {
    let policy1 = SecurityPolicy {
        name: "test".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Standard,
        allowed_capabilities: vec![Capability::Read],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    let policy2 = policy1.clone();
    assert_eq!(policy1.name, policy2.name);
}

#[test]
fn test_security_policy_debug() {
    let policy = SecurityPolicy {
        name: "debug-policy".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Standard,
        allowed_capabilities: vec![],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    let debug_str = format!("{policy:?}");
    assert!(debug_str.contains("SecurityPolicy"));
}

#[test]
fn test_security_policy_serialization() {
    let policy = SecurityPolicy {
        name: "serialize".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Standard,
        allowed_capabilities: vec![Capability::Read],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    let json = serde_json::to_string(&policy).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_security_policy_deserialization() {
    let policy = SecurityPolicy {
        name: "deserialize".to_string(),
        version: "2.0".to_string(),
        isolation_level: IsolationLevel::Enhanced,
        allowed_capabilities: vec![Capability::Execute],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    let json = serde_json::to_string(&policy).unwrap();
    let deserialized: SecurityPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(policy.name, deserialized.name);
    assert_eq!(policy.version, deserialized.version);
}

#[test]
fn test_security_policy_with_network_restrictions() {
    let net = NetworkSecurity {
        allow_outbound: true,
        allow_inbound: false,
        allowed_domains: vec!["example.com".to_string()],
        blocked_domains: vec!["malicious.com".to_string()],
        allowed_ports: vec![443],
        blocked_ports: vec![22],
    };

    let policy = SecurityPolicy {
        name: "network-restricted".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Enhanced,
        allowed_capabilities: vec![Capability::NetworkClient],
        denied_capabilities: vec![],
        network_restrictions: net.clone(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert!(policy.network_restrictions.allow_outbound);
    assert!(!policy.network_restrictions.allow_inbound);
}

#[test]
fn test_security_policy_with_filesystem_restrictions() {
    let fs = FilesystemSecurity {
        read_only: true,
        allowed_read_paths: vec!["/home".to_string()],
        allowed_write_paths: vec![],
        blocked_paths: vec!["/etc".to_string(), "/root".to_string()],
    };

    let policy = SecurityPolicy {
        name: "fs-restricted".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Enhanced,
        allowed_capabilities: vec![Capability::Read],
        denied_capabilities: vec![Capability::Write],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: fs.clone(),
    };

    assert!(policy.filesystem_restrictions.read_only);
    assert_eq!(policy.filesystem_restrictions.blocked_paths.len(), 2);
}

#[test]
fn test_security_policy_version_comparison() {
    let policy_v1 = SecurityPolicy {
        name: "app".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Standard,
        allowed_capabilities: vec![],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    let policy_v2 = SecurityPolicy {
        name: "app".to_string(),
        version: "2.0".to_string(),
        isolation_level: IsolationLevel::Enhanced,
        allowed_capabilities: vec![],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert_ne!(policy_v1.version, policy_v2.version);
}

#[test]
fn test_security_policy_comprehensive() {
    let policy = SecurityPolicy {
        name: "comprehensive".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::Maximum,
        allowed_capabilities: vec![Capability::Read, Capability::Execute],
        denied_capabilities: vec![Capability::Write, Capability::NetworkServer],
        network_restrictions: NetworkSecurity {
            allow_outbound: true,
            allow_inbound: false,
            allowed_domains: vec!["api.example.com".to_string()],
            blocked_domains: vec![],
            allowed_ports: vec![443],
            blocked_ports: vec![],
        },
        filesystem_restrictions: FilesystemSecurity {
            read_only: false,
            allowed_read_paths: vec!["/app".to_string()],
            allowed_write_paths: vec!["/tmp".to_string()],
            blocked_paths: vec!["/etc".to_string()],
        },
    };

    assert_eq!(policy.allowed_capabilities.len(), 2);
    assert_eq!(policy.denied_capabilities.len(), 2);
    assert!(!policy.filesystem_restrictions.read_only);
}

#[test]
fn test_security_policy_empty_restrictions() {
    let policy = SecurityPolicy {
        name: "minimal".to_string(),
        version: "1.0".to_string(),
        isolation_level: IsolationLevel::None,
        allowed_capabilities: vec![],
        denied_capabilities: vec![],
        network_restrictions: NetworkSecurity::default(),
        filesystem_restrictions: FilesystemSecurity::default(),
    };

    assert!(policy.allowed_capabilities.is_empty());
    assert!(policy.denied_capabilities.is_empty());
}

// ============================================================================
// AuditSettings Tests (10 tests)
// ============================================================================

#[test]
fn test_audit_settings_default() {
    let settings = AuditSettings::default();

    assert!(settings.enabled);
    assert_eq!(settings.log_level, "info");
    assert!(!settings.events.is_empty());
}

#[test]
fn test_audit_settings_custom() {
    let settings = AuditSettings {
        enabled: false,
        log_level: "debug".to_string(),
        events: vec![AuditEvent::SecurityViolation],
    };

    assert!(!settings.enabled);
    assert_eq!(settings.log_level, "debug");
}

#[test]
fn test_audit_settings_all_events() {
    let settings = AuditSettings {
        enabled: true,
        log_level: "info".to_string(),
        events: vec![
            AuditEvent::ExecutionStart,
            AuditEvent::ExecutionEnd,
            AuditEvent::SecurityViolation,
            AuditEvent::CapabilityUsed,
            AuditEvent::NetworkAccess,
            AuditEvent::FilesystemAccess,
        ],
    };

    assert_eq!(settings.events.len(), 6);
}

#[test]
fn test_audit_settings_clone() {
    let settings1 = AuditSettings::default();
    let settings2 = settings1.clone();

    assert_eq!(settings1.enabled, settings2.enabled);
    assert_eq!(settings1.log_level, settings2.log_level);
}

#[test]
fn test_audit_settings_debug() {
    let settings = AuditSettings::default();
    let debug_str = format!("{settings:?}");
    assert!(debug_str.contains("AuditSettings"));
}

#[test]
fn test_audit_settings_serialization() {
    let settings = AuditSettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_audit_settings_deserialization() {
    let settings = AuditSettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    let deserialized: AuditSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(settings.enabled, deserialized.enabled);
}

#[test]
fn test_audit_settings_disabled() {
    let settings = AuditSettings {
        enabled: false,
        log_level: "error".to_string(),
        events: vec![],
    };

    assert!(!settings.enabled);
    assert!(settings.events.is_empty());
}

#[test]
fn test_audit_settings_log_levels() {
    let levels = vec!["debug", "info", "warn", "error", "critical"];

    for level in levels {
        let settings = AuditSettings {
            enabled: true,
            log_level: level.to_string(),
            events: vec![],
        };
        assert_eq!(settings.log_level, level);
    }
}

#[test]
fn test_audit_settings_event_filtering() {
    let settings = AuditSettings {
        enabled: true,
        log_level: "info".to_string(),
        events: vec![AuditEvent::ExecutionStart, AuditEvent::ExecutionEnd],
    };

    assert!(settings.events.contains(&AuditEvent::ExecutionStart));
    assert!(!settings.events.contains(&AuditEvent::SecurityViolation));
}

// ============================================================================
// AuditEvent Tests (12 tests)
// ============================================================================

#[test]
fn test_audit_event_execution_start() {
    let event = AuditEvent::ExecutionStart;
    assert!(matches!(event, AuditEvent::ExecutionStart));
}

#[test]
fn test_audit_event_execution_end() {
    let event = AuditEvent::ExecutionEnd;
    assert!(matches!(event, AuditEvent::ExecutionEnd));
}

#[test]
fn test_audit_event_security_violation() {
    let event = AuditEvent::SecurityViolation;
    assert!(matches!(event, AuditEvent::SecurityViolation));
}

#[test]
fn test_audit_event_capability_used() {
    let event = AuditEvent::CapabilityUsed;
    assert!(matches!(event, AuditEvent::CapabilityUsed));
}

#[test]
fn test_audit_event_network_access() {
    let event = AuditEvent::NetworkAccess;
    assert!(matches!(event, AuditEvent::NetworkAccess));
}

#[test]
fn test_audit_event_filesystem_access() {
    let event = AuditEvent::FilesystemAccess;
    assert!(matches!(event, AuditEvent::FilesystemAccess));
}

#[test]
fn test_audit_event_equality() {
    let event1 = AuditEvent::ExecutionStart;
    let event2 = AuditEvent::ExecutionStart;
    let event3 = AuditEvent::ExecutionEnd;

    assert_eq!(event1, event2);
    assert_ne!(event1, event3);
}

#[test]
fn test_audit_event_clone() {
    let event1 = AuditEvent::SecurityViolation;
    let event2 = event1.clone();
    assert_eq!(event1, event2);
}

#[test]
fn test_audit_event_debug() {
    let event = AuditEvent::CapabilityUsed;
    let debug_str = format!("{event:?}");
    assert!(debug_str.contains("CapabilityUsed"));
}

#[test]
fn test_audit_event_serialization() {
    let event = AuditEvent::NetworkAccess;
    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_audit_event_deserialization() {
    let event = AuditEvent::FilesystemAccess;
    let json = serde_json::to_string(&event).unwrap();
    let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(event, deserialized);
}

#[test]
fn test_audit_event_all_variants() {
    let events = [
        AuditEvent::ExecutionStart,
        AuditEvent::ExecutionEnd,
        AuditEvent::SecurityViolation,
        AuditEvent::CapabilityUsed,
        AuditEvent::NetworkAccess,
        AuditEvent::FilesystemAccess,
    ];
    assert_eq!(events.len(), 6);
}

// ============================================================================
// SecuritySettings Tests (8 tests)
// ============================================================================

#[test]
fn test_security_settings_default() {
    let settings = SecuritySettings::default();

    assert!(matches!(
        settings.default_isolation_level,
        IsolationLevel::Standard
    ));
    assert!(!settings.default_capabilities.is_empty());
    assert!(settings.security_policies.is_empty());
}

#[test]
fn test_security_settings_custom() {
    let mut policies = HashMap::new();
    policies.insert(
        "default".to_string(),
        SecurityPolicy {
            name: "default".to_string(),
            version: "1.0".to_string(),
            isolation_level: IsolationLevel::Standard,
            allowed_capabilities: vec![],
            denied_capabilities: vec![],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        },
    );

    let settings = SecuritySettings {
        default_isolation_level: IsolationLevel::Enhanced,
        default_capabilities: vec![Capability::Read, Capability::Execute],
        security_policies: policies,
        audit_settings: AuditSettings::default(),
    };

    assert_eq!(settings.security_policies.len(), 1);
}

#[test]
fn test_security_settings_clone() {
    let settings1 = SecuritySettings::default();
    let settings2 = settings1.clone();

    assert_eq!(
        format!("{:?}", settings1.default_isolation_level),
        format!("{:?}", settings2.default_isolation_level)
    );
}

#[test]
fn test_security_settings_debug() {
    let settings = SecuritySettings::default();
    let debug_str = format!("{settings:?}");
    assert!(debug_str.contains("SecuritySettings"));
}

#[test]
fn test_security_settings_serialization() {
    let settings = SecuritySettings::default();
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_security_settings_with_multiple_policies() {
    let mut policies = HashMap::new();

    policies.insert(
        "low".to_string(),
        SecurityPolicy {
            name: "low".to_string(),
            version: "1.0".to_string(),
            isolation_level: IsolationLevel::Basic,
            allowed_capabilities: vec![],
            denied_capabilities: vec![],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        },
    );

    policies.insert(
        "high".to_string(),
        SecurityPolicy {
            name: "high".to_string(),
            version: "1.0".to_string(),
            isolation_level: IsolationLevel::Enhanced,
            allowed_capabilities: vec![],
            denied_capabilities: vec![],
            network_restrictions: NetworkSecurity::default(),
            filesystem_restrictions: FilesystemSecurity::default(),
        },
    );

    let settings = SecuritySettings {
        default_isolation_level: IsolationLevel::Standard,
        default_capabilities: vec![],
        security_policies: policies,
        audit_settings: AuditSettings::default(),
    };

    assert_eq!(settings.security_policies.len(), 2);
}

#[test]
fn test_security_settings_default_capabilities() {
    let settings = SecuritySettings {
        default_isolation_level: IsolationLevel::Standard,
        default_capabilities: vec![
            Capability::Read,
            Capability::Execute,
            Capability::NetworkClient,
        ],
        security_policies: HashMap::new(),
        audit_settings: AuditSettings::default(),
    };

    assert_eq!(settings.default_capabilities.len(), 3);
}

#[test]
fn test_security_settings_audit_enabled() {
    let settings = SecuritySettings::default();
    assert!(settings.audit_settings.enabled);
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_security_policy_coverage_summary() {
    println!("=== Security Policy Test Coverage ===");
    println!("SecurityPolicy Tests:       12 tests");
    println!("AuditSettings Tests:        10 tests");
    println!("AuditEvent Tests:           12 tests");
    println!("SecuritySettings Tests:      8 tests");
    println!("─────────────────────────────────────");
    println!("Total:                      42 tests");
    println!("Module: Security (Policy & Audit)");
    println!("=====================================");
}
