//! Security context validation tests
//! 
//! Tests the security context creation, validation, and enforcement.

use toadstool::{SecurityContext, SecuritySettings, ToadStoolResult};

#[test]
fn test_security_context_default() {
    let context = SecurityContext::default();
    assert!(context.validate().is_ok(), "Default security context should be valid");
}

#[test]
fn test_security_context_creation() {
    let context = SecurityContext {
        user_id: Some("test-user".to_string()),
        permissions: vec!["read".to_string(), "write".to_string()],
        isolation_level: toadstool::IsolationLevel::Process,
        resource_limits: Default::default(),
    };
    
    assert!(context.validate().is_ok());
    assert_eq!(context.user_id, Some("test-user".to_string()));
    assert_eq!(context.permissions.len(), 2);
}

#[test]
fn test_security_settings_validation() {
    let settings = SecuritySettings {
        enabled: true,
        enforce_sandboxing: true,
        allow_network: false,
        allow_filesystem: true,
        max_memory_mb: 1024,
    };
    
    assert!(settings.enabled);
    assert!(!settings.allow_network);
}

#[test]
fn test_isolation_levels() {
    use toadstool::IsolationLevel;
    
    let levels = vec![
        IsolationLevel::None,
        IsolationLevel::Process,
        IsolationLevel::Container,
        IsolationLevel::VM,
    ];
    
    for level in levels {
        let context = SecurityContext {
            isolation_level: level,
            ..Default::default()
        };
        assert!(context.validate().is_ok());
    }
}

#[test]
fn test_permission_checking() {
    let mut context = SecurityContext::default();
    context.permissions = vec!["read".to_string(), "execute".to_string()];
    
    assert!(context.has_permission("read"));
    assert!(context.has_permission("execute"));
    assert!(!context.has_permission("write"));
}

#[test]
fn test_security_context_cloning() {
    let context = SecurityContext {
        user_id: Some("user1".to_string()),
        permissions: vec!["read".to_string()],
        isolation_level: toadstool::IsolationLevel::Container,
        resource_limits: Default::default(),
    };
    
    let cloned = context.clone();
    assert_eq!(context.user_id, cloned.user_id);
    assert_eq!(context.permissions, cloned.permissions);
}

#[test]
fn test_security_context_serialization() {
    let context = SecurityContext {
        user_id: Some("test".to_string()),
        permissions: vec!["admin".to_string()],
        isolation_level: toadstool::IsolationLevel::Process,
        resource_limits: Default::default(),
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
        user_id: Some("restricted-user".to_string()),
        permissions: vec![],
        isolation_level: toadstool::IsolationLevel::VM,
        resource_limits: Some(toadstool::resources::ResourceLimits {
            max_cpu_percent: 25.0,
            max_memory_mb: 512,
            max_disk_io_mbps: 10,
            max_network_mbps: 10,
        }),
    };
    
    assert!(context.validate().is_ok());
    assert!(context.permissions.is_empty());
}

#[test]
fn test_permissive_security_context() {
    let context = SecurityContext {
        user_id: None,
        permissions: vec!["*".to_string()], // All permissions
        isolation_level: toadstool::IsolationLevel::None,
        resource_limits: None,
    };
    
    // Permissive context should still be valid
    assert!(context.validate().is_ok());
}

