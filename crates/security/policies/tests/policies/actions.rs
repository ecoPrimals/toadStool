// SPDX-License-Identifier: AGPL-3.0-only
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

// ============================================================================
// LogicalOperator Tests (NEW)
// ============================================================================

#[test]
fn test_logical_operator_and() {
    let op = LogicalOperator::And;
    assert!(matches!(op, LogicalOperator::And));
}

#[test]
fn test_logical_operator_or() {
    let op = LogicalOperator::Or;
    assert!(matches!(op, LogicalOperator::Or));
}

#[test]
fn test_logical_operator_not() {
    let op = LogicalOperator::Not;
    assert!(matches!(op, LogicalOperator::Not));
}

#[test]
fn test_logical_operator_clone() {
    let op1 = LogicalOperator::And;
    let op2 = op1.clone();
    assert!(matches!(op2, LogicalOperator::And));
}

// ============================================================================
// PolicyResult Tests (NEW)
// ============================================================================

#[test]
fn test_policy_result_allow() {
    let result = PolicyResult::Allow;
    assert!(matches!(result, PolicyResult::Allow));
}

#[test]
fn test_policy_result_deny() {
    let result = PolicyResult::Deny;
    assert!(matches!(result, PolicyResult::Deny));
}

// ============================================================================
// Extended Policy Condition Tests - Complex Scenarios
// ============================================================================

#[test]
fn test_composite_condition_and() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    if let PolicyCondition::Composite {
        operator,
        conditions,
    } = condition
    {
        assert!(matches!(operator, LogicalOperator::And));
        assert_eq!(conditions.len(), 2);
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_composite_condition_or() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
    };

    if let PolicyCondition::Composite { operator, .. } = condition {
        assert!(matches!(operator, LogicalOperator::Or));
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_composite_condition_not() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };

    if let PolicyCondition::Composite { operator, .. } = condition {
        assert!(matches!(operator, LogicalOperator::Not));
    } else {
        panic!("Expected Composite condition");
    }
}

#[test]
fn test_time_window_condition() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5], // Monday-Friday
    };

    if let PolicyCondition::TimeWindow {
        start_hour,
        end_hour,
        days,
    } = condition
    {
        assert_eq!(start_hour, 9);
        assert_eq!(end_hour, 17);
        assert_eq!(days.len(), 5);
    } else {
        panic!("Expected TimeWindow condition");
    }
}

#[test]
fn test_time_window_24_hours() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 0,
        end_hour: 23,
        days: vec![0, 1, 2, 3, 4, 5, 6], // All days
    };

    if let PolicyCondition::TimeWindow { days, .. } = condition {
        assert_eq!(days.len(), 7);
    } else {
        panic!("Expected TimeWindow condition");
    }
}

#[test]
fn test_user_context_condition() {
    let condition = PolicyCondition::UserContext {
        users: vec!["alice".to_string(), "bob".to_string()],
        groups: vec!["admin".to_string(), "developers".to_string()],
    };

    if let PolicyCondition::UserContext { users, groups } = condition {
        assert_eq!(users.len(), 2);
        assert_eq!(groups.len(), 2);
        assert!(users.contains(&"alice".to_string()));
    } else {
        panic!("Expected UserContext condition");
    }
}

#[test]
fn test_user_context_empty_groups() {
    let condition = PolicyCondition::UserContext {
        users: vec!["root".to_string()],
        groups: vec![],
    };

    if let PolicyCondition::UserContext { users, groups } = condition {
        assert_eq!(users.len(), 1);
        assert_eq!(groups.len(), 0);
    } else {
        panic!("Expected UserContext condition");
    }
}

#[test]
fn test_network_access_condition() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["api.example.com".to_string(), "192.168.1.1".to_string()],
        ports: vec![80, 443, 8080],
    };

    if let PolicyCondition::NetworkAccess { hosts, ports } = condition {
        assert_eq!(hosts.len(), 2);
        assert_eq!(ports.len(), 3);
        assert!(ports.contains(&443));
    } else {
        panic!("Expected NetworkAccess condition");
    }
}

#[test]
fn test_network_access_all_ports() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["localhost".to_string()],
        ports: vec![],
    };

    if let PolicyCondition::NetworkAccess { ports, .. } = condition {
        assert_eq!(ports.len(), 0);
    } else {
        panic!("Expected NetworkAccess condition");
    }
}

#[test]
fn test_filesystem_access_condition() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/tmp"), PathBuf::from("/var/log")],
        operations: vec!["read".to_string(), "write".to_string()],
    };

    if let PolicyCondition::FileSystemAccess { paths, operations } = condition {
        assert_eq!(paths.len(), 2);
        assert_eq!(operations.len(), 2);
    } else {
        panic!("Expected FileSystemAccess condition");
    }
}

#[test]
fn test_filesystem_access_read_only() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/etc")],
        operations: vec!["read".to_string()],
    };

    if let PolicyCondition::FileSystemAccess { operations, .. } = condition {
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0], "read");
    } else {
        panic!("Expected FileSystemAccess condition");
    }
}

#[test]
fn test_custom_condition() {
    let mut variables = HashMap::new();
    variables.insert("threshold".to_string(), serde_json::json!(100));

    let condition = PolicyCondition::Custom {
        expression: "value > threshold".to_string(),
        variables,
    };

    if let PolicyCondition::Custom {
        expression,
        variables,
    } = condition
    {
        assert!(expression.contains("threshold"));
        assert_eq!(variables.len(), 1);
    } else {
        panic!("Expected Custom condition");
    }
}

#[test]
fn test_resource_usage_cpu_only() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: None,
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert_eq!(cpu_percent, Some(80.0));
        assert_eq!(memory_mb, None);
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

#[test]
fn test_resource_usage_memory_only() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: None,
        memory_mb: Some(2048),
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert_eq!(cpu_percent, None);
        assert_eq!(memory_mb, Some(2048));
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

#[test]
fn test_resource_usage_both() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(50.0),
        memory_mb: Some(1024),
    };

    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = condition
    {
        assert!(cpu_percent.is_some());
        assert!(memory_mb.is_some());
    } else {
        panic!("Expected ResourceUsage condition");
    }
}

