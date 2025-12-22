// ============================================================================
// Policy Action Tests
// ============================================================================

#[test]
fn test_action_allow() {
    let action = PolicyAction::Allow;
    assert!(matches!(action, PolicyAction::Allow));
}

#[test]
fn test_action_deny() {
    let action = PolicyAction::Deny;
    assert!(matches!(action, PolicyAction::Deny));
}

#[test]
fn test_action_allow_with_warning() {
    let action = PolicyAction::AllowWithWarning {
        message: "Warning message".to_string(),
    };

    match action {
        PolicyAction::AllowWithWarning { message } => {
            assert!(!message.is_empty());
        }
        _ => panic!("Expected AllowWithWarning action"),
    }
}

#[test]
fn test_action_deny_with_message() {
    let action = PolicyAction::DenyWithMessage {
        message: "Access denied".to_string(),
    };

    match action {
        PolicyAction::DenyWithMessage { message } => {
            assert_eq!(message, "Access denied");
        }
        _ => panic!("Expected DenyWithMessage action"),
    }
}

#[test]
fn test_action_modify_security_context() {
    let action = PolicyAction::ModifySecurityContext {
        isolation_level: Some(toadstool::security::IsolationLevel::Maximum),
        add_capabilities: vec![toadstool::security::Capability::Read],
        remove_capabilities: vec![],
    };

    match action {
        PolicyAction::ModifySecurityContext {
            isolation_level,
            add_capabilities,
            remove_capabilities,
        } => {
            assert!(isolation_level.is_some());
            assert_eq!(add_capabilities.len(), 1);
            assert!(remove_capabilities.is_empty());
        }
        _ => panic!("Expected ModifySecurityContext action"),
    }
}

#[test]
fn test_action_apply_resource_limits() {
    let action = PolicyAction::ApplyResourceLimits {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
        network_mbps: Some(100.0),
    };

    match action {
        PolicyAction::ApplyResourceLimits {
            cpu_percent,
            memory_mb,
            network_mbps,
        } => {
            assert_eq!(cpu_percent, Some(50.0));
            assert_eq!(memory_mb, Some(1024));
            assert_eq!(network_mbps, Some(100.0));
        }
        _ => panic!("Expected ApplyResourceLimits action"),
    }
}

#[test]
fn test_action_require_authentication() {
    let action = PolicyAction::RequireAuthentication {
        method: "2FA".to_string(),
    };

    match action {
        PolicyAction::RequireAuthentication { method } => {
            assert_eq!(method, "2FA");
        }
        _ => panic!("Expected RequireAuthentication action"),
    }
}

#[test]
fn test_action_log_and_continue() {
    let action = PolicyAction::LogAndContinue {
        level: "warn".to_string(),
        message: "Suspicious activity".to_string(),
    };

    match action {
        PolicyAction::LogAndContinue { level, message } => {
            assert_eq!(level, "warn");
            assert!(!message.is_empty());
        }
        _ => panic!("Expected LogAndContinue action"),
    }
}

#[test]
fn test_action_custom() {
    let mut params = HashMap::new();
    params.insert("retry_count".to_string(), serde_json::json!(3));

    let action = PolicyAction::Custom {
        action: "custom_action".to_string(),
        parameters: params,
    };

    match action {
        PolicyAction::Custom { action, parameters } => {
            assert_eq!(action, "custom_action");
            assert_eq!(parameters.len(), 1);
        }
        _ => panic!("Expected Custom action"),
    }
}

// ============================================================================
// Violation Action Tests
// ============================================================================

#[test]
fn test_violation_action_terminate() {
    let action = ViolationAction::Terminate;
    assert!(matches!(action, ViolationAction::Terminate));
}

#[test]
fn test_violation_action_block() {
    let action = ViolationAction::Block;
    assert!(matches!(action, ViolationAction::Block));
}

#[test]
fn test_violation_action_log_and_continue() {
    let action = ViolationAction::LogAndContinue;
    assert!(matches!(action, ViolationAction::LogAndContinue));
}

#[test]
fn test_violation_action_quarantine() {
    let action = ViolationAction::Quarantine;
    assert!(matches!(action, ViolationAction::Quarantine));
}

#[test]
fn test_violation_action_alert() {
    let action = ViolationAction::Alert;
    assert!(matches!(action, ViolationAction::Alert));
}

