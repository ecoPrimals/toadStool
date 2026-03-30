// SPDX-License-Identifier: AGPL-3.0-only
//! Core evaluator tests for security policy condition evaluation
//!
//! This test module provides comprehensive coverage of the `ConditionEvaluator`,
//! testing all condition types, validation logic, and edge cases.

use std::collections::HashMap;
use toadstool::security::Capability;
use toadstool_security_policies::evaluator::ConditionEvaluator;
use toadstool_security_policies::types::*;

#[test]
fn test_evaluator_creation() {
    let evaluator = ConditionEvaluator::new();
    // Verify evaluator can be created
    assert!(std::mem::size_of_val(&evaluator) > 0);
}

#[test]
fn test_evaluator_default() {
    let evaluator = ConditionEvaluator::default();
    // Verify default implementation works
    assert!(std::mem::size_of_val(&evaluator) > 0);
}

// ============================================================================
// Condition Validation Tests
// ============================================================================

#[test]
fn test_validate_always_condition() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Always;

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_never_condition() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Never;

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_workload_type_condition_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["container".to_string(), "wasm".to_string()],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_workload_type_condition_empty() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec![],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Workload types cannot be empty");
}

#[test]
fn test_validate_capability_condition_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![Capability::NetworkClient, Capability::Write],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_capability_condition_empty() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Capabilities cannot be empty");
}

#[test]
fn test_validate_time_window_condition_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5], // Monday-Friday
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_time_window_condition_invalid_start_hour() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 24, // Invalid: > 23
        end_hour: 17,
        days: vec![1, 2, 3],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Hours must be 0-23");
}

#[test]
fn test_validate_time_window_condition_invalid_end_hour() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 25, // Invalid: > 23
        days: vec![1, 2, 3],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Hours must be 0-23");
}

#[test]
fn test_validate_time_window_condition_invalid_day() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 7], // Invalid: 7 > 6
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Days must be 0-6");
}

#[test]
fn test_validate_custom_condition_valid() {
    let evaluator = ConditionEvaluator::new();
    let mut variables = HashMap::new();
    variables.insert("threshold".to_string(), serde_json::json!(100));

    let condition = PolicyCondition::Custom {
        expression: "cpu_usage < threshold".to_string(),
        variables,
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_custom_condition_empty_expression() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Custom {
        expression: String::new(),
        variables: HashMap::new(),
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Custom expression cannot be empty");
}

#[test]
fn test_validate_composite_condition_and() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![
            PolicyCondition::Always,
            PolicyCondition::WorkloadType {
                workload_types: vec!["container".to_string()],
            },
        ],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_composite_condition_or() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_composite_condition_not() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_composite_condition_nested_invalid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![
            PolicyCondition::Always,
            PolicyCondition::WorkloadType {
                workload_types: vec![], // Invalid: empty
            },
        ],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Workload types cannot be empty");
}

// ============================================================================
// Resource Usage Condition Tests
// ============================================================================

#[test]
fn test_validate_resource_usage_condition_cpu_only() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: None,
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_resource_usage_condition_memory_only() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: None,
        memory_mb: Some(1024),
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_resource_usage_condition_both() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(80.0),
        memory_mb: Some(2048),
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

// ============================================================================
// Network and FileSystem Condition Tests
// ============================================================================

#[test]
fn test_validate_network_access_condition() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::NetworkAccess {
        hosts: vec!["example.com".to_string(), "api.service.local".to_string()],
        ports: vec![80, 443, 8080],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_filesystem_access_condition() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::FileSystemAccess {
        paths: vec!["/tmp".into(), "/var/data".into()],
        operations: vec!["read".to_string(), "write".to_string()],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_user_context_condition() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::UserContext {
        users: vec!["alice".to_string(), "bob".to_string()],
        groups: vec!["admin".to_string(), "developers".to_string()],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_validate_time_window_boundary_hours() {
    let evaluator = ConditionEvaluator::new();

    // Test boundary: 0 and 23 are valid
    let condition = PolicyCondition::TimeWindow {
        start_hour: 0,
        end_hour: 23,
        days: vec![0, 6], // Sunday and Saturday
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_time_window_all_days() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 0,
        end_hour: 23,
        days: vec![0, 1, 2, 3, 4, 5, 6], // All days
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_deeply_nested_composite() {
    let evaluator = ConditionEvaluator::new();

    // Create a deeply nested composite condition
    let inner_condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };

    let middle_condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![inner_condition.clone(), inner_condition],
    };

    let outer_condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![middle_condition],
    };

    let result = evaluator.validate_condition(&outer_condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_single_capability() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![Capability::NetworkClient],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

#[test]
fn test_validate_many_workload_types() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec![
            "container".to_string(),
            "wasm".to_string(),
            "native".to_string(),
            "python".to_string(),
            "gpu".to_string(),
        ],
    };

    let result = evaluator.validate_condition(&condition);
    assert!(result.is_ok());
}

// ============================================================================
// Module Integration Tests
// ============================================================================

#[test]
fn test_evaluator_handles_all_condition_types() {
    let evaluator = ConditionEvaluator::new();

    let conditions = vec![
        PolicyCondition::Always,
        PolicyCondition::Never,
        PolicyCondition::WorkloadType {
            workload_types: vec!["test".to_string()],
        },
        PolicyCondition::RequiresCapability {
            capabilities: vec![Capability::NetworkClient],
        },
        PolicyCondition::ResourceUsage {
            cpu_percent: Some(50.0),
            memory_mb: Some(1024),
        },
        PolicyCondition::TimeWindow {
            start_hour: 9,
            end_hour: 17,
            days: vec![1, 2, 3, 4, 5],
        },
        PolicyCondition::UserContext {
            users: vec!["test".to_string()],
            groups: vec!["test".to_string()],
        },
        PolicyCondition::NetworkAccess {
            hosts: vec!["localhost".to_string()],
            ports: vec![8080],
        },
        PolicyCondition::FileSystemAccess {
            paths: vec!["/tmp".into()],
            operations: vec!["read".to_string()],
        },
        PolicyCondition::Custom {
            expression: "true".to_string(),
            variables: HashMap::new(),
        },
    ];

    for condition in conditions {
        let result = evaluator.validate_condition(&condition);
        assert!(
            result.is_ok(),
            "Failed to validate condition: {condition:?}"
        );
    }
}
