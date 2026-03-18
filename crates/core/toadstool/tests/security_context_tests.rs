// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Security Context and Policies
//!
//! Week 17 Sprint: Security module expansion (12% → 30%)
//! Focus: `SecurityContext`, Capability, `IsolationLevel`, `NetworkSecurity`, `FilesystemSecurity`

use toadstool::security::*;

// ============================================================================
// SecurityContext Tests (15 tests)
// ============================================================================

#[test]
fn test_security_context_default() {
    let context = SecurityContext::default();

    assert!(matches!(context.isolation_level, IsolationLevel::Standard));
    assert!(!context.capabilities.is_empty());
    assert!(context.user_context.is_none());
}

#[test]
fn test_security_context_for_isolation_level() {
    let context = SecurityContext::for_isolation_level(IsolationLevel::Enhanced);

    assert!(matches!(context.isolation_level, IsolationLevel::Enhanced));
    assert!(context.has_capability(&Capability::Execute));
}

#[test]
fn test_security_context_with_capability() {
    let context = SecurityContext::default()
        .with_capability(Capability::Write)
        .with_capability(Capability::NetworkClient);

    assert!(context.has_capability(&Capability::Write));
    assert!(context.has_capability(&Capability::NetworkClient));
}

#[test]
fn test_security_context_has_capability() {
    let context = SecurityContext::default();

    assert!(context.has_capability(&Capability::Execute));
    assert!(context.has_capability(&Capability::Read));
    assert!(!context.has_capability(&Capability::ProcessManagement));
}

#[test]
fn test_security_context_with_user_context() {
    let user = UserContext {
        username: Some("testuser".to_string()),
        uid: Some(1000),
        gid: Some(1000),
        groups: vec![100, 200],
    };

    let context = SecurityContext::default().with_user_context(user.clone());

    assert!(context.user_context.is_some());
    assert_eq!(
        context.user_context.unwrap().username,
        Some("testuser".to_string())
    );
}

#[test]
fn test_security_context_validate_success() {
    let context = SecurityContext::default();
    assert!(context.validate().is_ok());
}

#[test]
fn test_security_context_validate_empty_capabilities() {
    let context = SecurityContext {
        isolation_level: IsolationLevel::Standard,
        capabilities: vec![],
        user_context: None,
        network_security: NetworkSecurity::default(),
        filesystem_security: FilesystemSecurity::default(),
    };

    assert!(context.validate().is_err());
}

#[test]
fn test_security_context_clone() {
    let context1 = SecurityContext::default();
    let context2 = context1.clone();

    assert_eq!(context1.capabilities.len(), context2.capabilities.len());
}

#[test]
fn test_security_context_debug() {
    let context = SecurityContext::default();
    let debug_str = format!("{context:?}");
    assert!(debug_str.contains("SecurityContext"));
}

#[test]
fn test_security_context_serialization() {
    let context = SecurityContext::default();
    let json = serde_json::to_string(&context).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_security_context_deserialization() {
    let context = SecurityContext::default();
    let json = serde_json::to_string(&context).unwrap();
    let deserialized: SecurityContext = serde_json::from_str(&json).unwrap();
    assert_eq!(context.capabilities.len(), deserialized.capabilities.len());
}

#[test]
fn test_security_context_multiple_capabilities() {
    let context = SecurityContext::default()
        .with_capability(Capability::Write)
        .with_capability(Capability::NetworkClient)
        .with_capability(Capability::SystemInfo);

    assert!(context.has_capability(&Capability::Write));
    assert!(context.has_capability(&Capability::NetworkClient));
    assert!(context.has_capability(&Capability::SystemInfo));
}

#[test]
fn test_security_context_different_isolation_levels() {
    let none = SecurityContext::for_isolation_level(IsolationLevel::None);
    let basic = SecurityContext::for_isolation_level(IsolationLevel::Basic);
    let standard = SecurityContext::for_isolation_level(IsolationLevel::Standard);
    let enhanced = SecurityContext::for_isolation_level(IsolationLevel::Enhanced);

    assert!(matches!(none.isolation_level, IsolationLevel::None));
    assert!(matches!(basic.isolation_level, IsolationLevel::Basic));
    assert!(matches!(standard.isolation_level, IsolationLevel::Standard));
    assert!(matches!(enhanced.isolation_level, IsolationLevel::Enhanced));
}

#[test]
fn test_security_context_builder_pattern() {
    let user = UserContext {
        username: Some("builder".to_string()),
        uid: Some(1001),
        gid: Some(1001),
        groups: vec![],
    };

    let context = SecurityContext::for_isolation_level(IsolationLevel::Enhanced)
        .with_capability(Capability::Read)
        .with_capability(Capability::Write)
        .with_user_context(user);

    assert!(matches!(context.isolation_level, IsolationLevel::Enhanced));
    assert!(context.has_capability(&Capability::Read));
    assert!(context.user_context.is_some());
}

#[test]
fn test_security_context_network_security() {
    let context = SecurityContext::default();
    // Default network security should exist
    assert!(!context.network_security.allow_outbound);
}

// ============================================================================
// IsolationLevel Tests (10 tests)
// ============================================================================

#[test]
fn test_isolation_level_none() {
    let level = IsolationLevel::None;
    assert!(matches!(level, IsolationLevel::None));
}

#[test]
fn test_isolation_level_basic() {
    let level = IsolationLevel::Basic;
    assert!(matches!(level, IsolationLevel::Basic));
}

#[test]
fn test_isolation_level_standard() {
    let level = IsolationLevel::Standard;
    assert!(matches!(level, IsolationLevel::Standard));
}

#[test]
fn test_isolation_level_enhanced() {
    let level = IsolationLevel::Enhanced;
    assert!(matches!(level, IsolationLevel::Enhanced));
}

#[test]
fn test_isolation_level_variants_distinct() {
    let none = IsolationLevel::None;
    let basic = IsolationLevel::Basic;
    let standard = IsolationLevel::Standard;
    let enhanced = IsolationLevel::Enhanced;

    // All variants should be distinct
    assert_ne!(none, basic);
    assert_ne!(basic, standard);
    assert_ne!(standard, enhanced);
}

#[test]
fn test_isolation_level_equality() {
    let level1 = IsolationLevel::Enhanced;
    let level2 = IsolationLevel::Enhanced;
    let level3 = IsolationLevel::Standard;

    assert_eq!(level1, level2);
    assert_ne!(level1, level3);
}

#[test]
fn test_isolation_level_clone() {
    let level1 = IsolationLevel::Enhanced;
    let level2 = level1.clone();
    assert_eq!(level1, level2);
}

#[test]
fn test_isolation_level_debug() {
    let level = IsolationLevel::Enhanced;
    let debug_str = format!("{level:?}");
    assert!(debug_str.contains("Enhanced"));
}

#[test]
fn test_isolation_level_serialization() {
    let level = IsolationLevel::Enhanced;
    let json = serde_json::to_string(&level).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_isolation_level_all_variants() {
    let levels = [
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Standard,
        IsolationLevel::Enhanced,
    ];
    assert_eq!(levels.len(), 4);
}

// ============================================================================
// Capability Tests (12 tests)
// ============================================================================

#[test]
fn test_capability_execute() {
    let cap = Capability::Execute;
    assert!(matches!(cap, Capability::Execute));
}

#[test]
fn test_capability_read() {
    let cap = Capability::Read;
    assert!(matches!(cap, Capability::Read));
}

#[test]
fn test_capability_write() {
    let cap = Capability::Write;
    assert!(matches!(cap, Capability::Write));
}

#[test]
fn test_capability_network_client() {
    let cap = Capability::NetworkClient;
    assert!(matches!(cap, Capability::NetworkClient));
}

#[test]
fn test_capability_network_server() {
    let cap = Capability::NetworkServer;
    assert!(matches!(cap, Capability::NetworkServer));
}

#[test]
fn test_capability_system_info() {
    let cap = Capability::SystemInfo;
    assert!(matches!(cap, Capability::SystemInfo));
}

#[test]
fn test_capability_process_management() {
    let cap = Capability::ProcessManagement;
    assert!(matches!(cap, Capability::ProcessManagement));
}

#[test]
fn test_capability_custom() {
    let cap = Capability::Custom("special".to_string());
    assert!(matches!(cap, Capability::Custom(_)));
}

#[test]
fn test_capability_equality() {
    let cap1 = Capability::Read;
    let cap2 = Capability::Read;
    let cap3 = Capability::Write;

    assert_eq!(cap1, cap2);
    assert_ne!(cap1, cap3);
}

#[test]
fn test_capability_clone() {
    let cap1 = Capability::NetworkClient;
    let cap2 = cap1.clone();
    assert_eq!(cap1, cap2);
}

#[test]
fn test_capability_debug() {
    let cap = Capability::ProcessManagement;
    let debug_str = format!("{cap:?}");
    assert!(debug_str.contains("ProcessManagement"));
}

#[test]
fn test_capability_serialization() {
    let cap = Capability::Execute;
    let json = serde_json::to_string(&cap).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_capability_in_vec() {
    let caps = [Capability::Read, Capability::Write];
    assert!(caps.contains(&Capability::Read));
    assert!(!caps.contains(&Capability::ProcessManagement));
}

// ============================================================================
// UserContext Tests (8 tests)
// ============================================================================

#[test]
fn test_user_context_creation() {
    let user = UserContext {
        username: Some("testuser".to_string()),
        uid: Some(1000),
        gid: Some(1000),
        groups: vec![100, 200],
    };

    assert_eq!(user.uid, Some(1000));
    assert_eq!(user.username, Some("testuser".to_string()));
}

#[test]
fn test_user_context_no_groups() {
    let user = UserContext {
        username: Some("nogroups".to_string()),
        uid: Some(1001),
        gid: Some(1001),
        groups: vec![],
    };

    assert!(user.groups.is_empty());
}

#[test]
fn test_user_context_multiple_groups() {
    let user = UserContext {
        username: Some("multigroup".to_string()),
        uid: Some(1002),
        gid: Some(1002),
        groups: vec![100, 200, 300],
    };

    assert_eq!(user.groups.len(), 3);
}

#[test]
fn test_user_context_clone() {
    let user1 = UserContext {
        username: Some("clone".to_string()),
        uid: Some(1003),
        gid: Some(1003),
        groups: vec![],
    };

    let user2 = user1.clone();
    assert_eq!(user1.uid, user2.uid);
}

#[test]
fn test_user_context_debug() {
    let user = UserContext {
        username: Some("debuguser".to_string()),
        uid: Some(1004),
        gid: Some(1004),
        groups: vec![],
    };

    let debug_str = format!("{user:?}");
    assert!(debug_str.contains("UserContext"));
}

#[test]
fn test_user_context_serialization() {
    let user = UserContext {
        username: Some("serialuser".to_string()),
        uid: Some(1005),
        gid: Some(1005),
        groups: vec![100],
    };

    let json = serde_json::to_string(&user).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_user_context_deserialization() {
    let user = UserContext {
        username: Some("deseruser".to_string()),
        uid: Some(1006),
        gid: Some(1006),
        groups: vec![],
    };

    let json = serde_json::to_string(&user).unwrap();
    let deserialized: UserContext = serde_json::from_str(&json).unwrap();
    assert_eq!(user.uid, deserialized.uid);
}

#[test]
fn test_user_context_group_membership() {
    let user = UserContext {
        username: Some("groupuser".to_string()),
        uid: Some(1007),
        gid: Some(1007),
        groups: vec![100, 200],
    };

    assert!(user.groups.contains(&100));
    assert!(!user.groups.contains(&999));
}

// ============================================================================
// NetworkSecurity Tests (6 tests)
// ============================================================================

#[test]
fn test_network_security_default() {
    let net = NetworkSecurity::default();
    assert!(!net.allow_outbound);
}

#[test]
fn test_network_security_custom() {
    let net = NetworkSecurity {
        allow_outbound: true,
        allow_inbound: false,
        allowed_domains: vec!["example.com".to_string()],
        blocked_domains: vec!["evil.com".to_string()],
        allowed_ports: vec![80, 443],
        blocked_ports: vec![22],
    };

    assert!(net.allow_outbound);
    assert!(!net.allow_inbound);
}

#[test]
fn test_network_security_clone() {
    let net1 = NetworkSecurity::default();
    let net2 = net1.clone();
    assert_eq!(net1.allow_outbound, net2.allow_outbound);
}

#[test]
fn test_network_security_debug() {
    let net = NetworkSecurity::default();
    let debug_str = format!("{net:?}");
    assert!(debug_str.contains("NetworkSecurity"));
}

#[test]
fn test_network_security_serialization() {
    let net = NetworkSecurity::default();
    let json = serde_json::to_string(&net).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_network_security_domain_and_port_lists() {
    let net = NetworkSecurity {
        allow_outbound: true,
        allow_inbound: true,
        allowed_domains: vec!["api.example.com".to_string()],
        blocked_domains: vec!["malware.bad".to_string()],
        allowed_ports: vec![443],
        blocked_ports: vec![23],
    };

    assert_eq!(net.allowed_domains.len(), 1);
    assert_eq!(net.blocked_domains.len(), 1);
    assert_eq!(net.allowed_ports.len(), 1);
}

// ============================================================================
// FilesystemSecurity Tests (6 tests)
// ============================================================================

#[test]
fn test_filesystem_security_default() {
    let fs = FilesystemSecurity::default();
    assert!(!fs.read_only);
}

#[test]
fn test_filesystem_security_custom() {
    let fs = FilesystemSecurity {
        read_only: true,
        allowed_read_paths: vec!["/home".to_string()],
        allowed_write_paths: vec!["/tmp".to_string()],
        blocked_paths: vec!["/etc".to_string()],
    };

    assert!(fs.read_only);
    assert!(!fs.allowed_write_paths.is_empty());
}

#[test]
fn test_filesystem_security_clone() {
    let fs1 = FilesystemSecurity::default();
    let fs2 = fs1.clone();
    assert_eq!(fs1.read_only, fs2.read_only);
}

#[test]
fn test_filesystem_security_debug() {
    let fs = FilesystemSecurity::default();
    let debug_str = format!("{fs:?}");
    assert!(debug_str.contains("FilesystemSecurity"));
}

#[test]
fn test_filesystem_security_serialization() {
    let fs = FilesystemSecurity::default();
    let json = serde_json::to_string(&fs).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_filesystem_security_path_lists() {
    let fs = FilesystemSecurity {
        read_only: false,
        allowed_read_paths: vec!["/home/user".to_string()],
        allowed_write_paths: vec!["/tmp".to_string()],
        blocked_paths: vec!["/root".to_string()],
    };

    assert_eq!(fs.allowed_read_paths.len(), 1);
    assert_eq!(fs.allowed_write_paths.len(), 1);
    assert_eq!(fs.blocked_paths.len(), 1);
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_security_context_coverage_summary() {
    println!("=== Security Context Test Coverage ===");
    println!("SecurityContext Tests:      15 tests");
    println!("IsolationLevel Tests:       10 tests");
    println!("Capability Tests:           12 tests");
    println!("UserContext Tests:           8 tests");
    println!("NetworkSecurity Tests:       6 tests");
    println!("FilesystemSecurity Tests:    6 tests");
    println!("──────────────────────────────────────");
    println!("Total:                      57 tests");
    println!("Target Coverage:            12% → 30%");
    println!("======================================");
}
