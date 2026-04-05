// SPDX-License-Identifier: AGPL-3.0-or-later
// ============================================================================
// Policy Rule Tests
// ============================================================================

#[test]
fn test_rule_creation() {
    let rule = PolicyRule {
        id: "test-rule".to_string(),
        name: "Test Rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 50,
        enabled: true,
        description: Some("Test description".to_string()),
    };

    assert_eq!(rule.id, "test-rule");
    assert_eq!(rule.priority, 50);
    assert!(rule.enabled);
}

#[test]
fn test_rule_disabled() {
    let rule = PolicyRule {
        id: "disabled-rule".to_string(),
        name: "Disabled Rule".to_string(),
        condition: PolicyCondition::Never,
        action: PolicyAction::Deny,
        priority: 100,
        enabled: false,
        description: None,
    };

    assert!(!rule.enabled);
}

#[test]
fn test_rule_priority_high() {
    let rule = PolicyRule {
        id: "high-priority".to_string(),
        name: "High Priority".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1000,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1000);
}

#[test]
fn test_rule_priority_low() {
    let rule = PolicyRule {
        id: "low-priority".to_string(),
        name: "Low Priority".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    };

    assert_eq!(rule.priority, 1);
}

// ============================================================================
// Policy Condition Tests
// ============================================================================

#[test]
fn test_condition_always() {
    let condition = PolicyCondition::Always;
    assert!(matches!(condition, PolicyCondition::Always));
}

#[test]
fn test_condition_never() {
    let condition = PolicyCondition::Never;
    assert!(matches!(condition, PolicyCondition::Never));
}

#[test]
fn test_condition_workload_type() {
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["native".to_string(), "wasm".to_string()],
    };

    match condition {
        PolicyCondition::WorkloadType { workload_types } => {
            assert_eq!(workload_types.len(), 2);
            assert!(workload_types.contains(&"native".to_string()));
        }
        _ => panic!("Expected WorkloadType condition"),
    }
}

#[test]
fn test_condition_capability() {
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![
            toadstool::security::Capability::Read,
            toadstool::security::Capability::NetworkClient,
        ],
    };

    match condition {
        PolicyCondition::RequiresCapability { capabilities } => {
            assert_eq!(capabilities.len(), 2);
        }
        _ => panic!("Expected RequiresCapability condition"),
    }
}

#[test]
fn test_condition_resource_usage() {
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: Some(2048),
    };

    match condition {
        PolicyCondition::ResourceUsage {
            cpu_percent,
            memory_mb,
        } => {
            assert_eq!(cpu_percent, Some(80.0));
            assert_eq!(memory_mb, Some(2048));
        }
        _ => panic!("Expected ResourceUsage condition"),
    }
}

#[test]
fn test_condition_time_window() {
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5],
    };

    match condition {
        PolicyCondition::TimeWindow {
            start_hour,
            end_hour,
            days,
        } => {
            assert_eq!(start_hour, 9);
            assert_eq!(end_hour, 17);
            assert_eq!(days.len(), 5);
        }
        _ => panic!("Expected TimeWindow condition"),
    }
}

#[test]
fn test_condition_user_context() {
    let condition = PolicyCondition::UserContext {
        users: vec!["admin".to_string()],
        groups: vec!["admins".to_string()],
    };

    match condition {
        PolicyCondition::UserContext { users, groups } => {
            assert_eq!(users.len(), 1);
            assert_eq!(groups.len(), 1);
        }
        _ => panic!("Expected UserContext condition"),
    }
}

#[test]
fn test_condition_network_access() {
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["api.example.com".to_string()],
        ports: vec![443, 8080],
    };

    match condition {
        PolicyCondition::NetworkAccess { hosts, ports } => {
            assert_eq!(hosts.len(), 1);
            assert_eq!(ports.len(), 2);
        }
        _ => panic!("Expected NetworkAccess condition"),
    }
}

#[test]
fn test_condition_filesystem_access() {
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec![PathBuf::from("/tmp")],
        operations: vec!["read".to_string(), "write".to_string()],
    };

    match condition {
        PolicyCondition::FileSystemAccess { paths, operations } => {
            assert_eq!(paths.len(), 1);
            assert_eq!(operations.len(), 2);
        }
        _ => panic!("Expected FileSystemAccess condition"),
    }
}

#[test]
fn test_condition_custom() {
    let mut vars = HashMap::new();
    vars.insert("threshold".to_string(), serde_json::json!(100));

    let condition = PolicyCondition::Custom {
        expression: "value > threshold".to_string(),
        variables: vars,
    };

    match condition {
        PolicyCondition::Custom {
            expression,
            variables,
        } => {
            assert!(!expression.is_empty());
            assert_eq!(variables.len(), 1);
        }
        _ => panic!("Expected Custom condition"),
    }
}

#[test]
fn test_condition_composite_and() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::And));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected Composite condition"),
    }
}

#[test]
fn test_condition_composite_or() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Never],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Or));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected Composite OR condition"),
    }
}

#[test]
fn test_condition_composite_not() {
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };

    match condition {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Not));
            assert_eq!(conditions.len(), 1);
        }
        _ => panic!("Expected Composite NOT condition"),
    }
}

#[test]
fn test_condition_nested_composite() {
    let inner = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    let outer = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![inner, PolicyCondition::Never],
    };

    match outer {
        PolicyCondition::Composite {
            operator,
            conditions,
        } => {
            assert!(matches!(operator, LogicalOperator::Or));
            assert_eq!(conditions.len(), 2);
        }
        _ => panic!("Expected outer Composite condition"),
    }
}

