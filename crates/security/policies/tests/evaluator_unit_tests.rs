// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for policy condition evaluator
//! Target: crates/security/policies/src/evaluator.rs (2.58% coverage)

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::SystemTime;

use toadstool::security::{Capability, SecurityContext};
use toadstool::workload::{ExecutableSource, WorkloadSpec};
use toadstool_security_policies::evaluator::ConditionEvaluator;
use toadstool_security_policies::types::{
    LogicalOperator, PolicyCondition, PolicyEvaluationContext, SystemInfo, UserInfo,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_context_native() -> PolicyEvaluationContext {
    PolicyEvaluationContext {
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/bin/test"),
            },
            args: Some(vec![]),
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext::default(),
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "test-host".to_string(),
            os_type: "linux".to_string(),
            os_version: "6.16.3".to_string(),
            architecture: "x86_64".to_string(),
            cpu_count: 8,
            memory_total_mb: 16384,
            load_average: 0.5,
            uptime_seconds: 3600,
        },
        timestamp: SystemTime::now(),
        variables: HashMap::new(),
    }
}

#[allow(dead_code)]
fn create_test_context_with_capabilities() -> PolicyEvaluationContext {
    let mut context = create_test_context_native();
    let mut caps = HashSet::new();
    caps.insert(Capability::NetworkClient);
    caps.insert(Capability::Read);
    context.requested_capabilities = caps;
    context
}

fn create_test_context_with_user() -> PolicyEvaluationContext {
    let mut context = create_test_context_native();
    context.user_info = Some(UserInfo {
        user_id: "user123".to_string(),
        username: "testuser".to_string(),
        groups: vec!["admin".to_string(), "developers".to_string()],
        roles: vec!["admin".to_string()],
        attributes: HashMap::new(),
    });
    context
}

// ============================================================================
// Test: ConditionEvaluator::new()
// ============================================================================

#[test]
fn test_evaluator_new() {
    let evaluator = ConditionEvaluator::new();
    // Should create successfully (basic smoke test)
    assert!(std::mem::size_of_val(&evaluator) > 0);
}

#[test]
fn test_evaluator_default() {
    let evaluator = ConditionEvaluator::default();
    // Should create successfully using Default trait
    assert!(std::mem::size_of_val(&evaluator) > 0);
}

// ============================================================================
// Test: Validation - Always/Never
// ============================================================================

#[test]
fn test_validate_always() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Always;
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_never() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Never;
    assert!(evaluator.validate_condition(&condition).is_ok());
}

// ============================================================================
// Test: Validation - WorkloadType
// ============================================================================

#[test]
fn test_validate_workload_type_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["native".to_string(), "wasm".to_string()],
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_workload_type_empty() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec![],
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Workload types cannot be empty");
}

// ============================================================================
// Test: Validation - RequiresCapability
// ============================================================================

#[test]
fn test_validate_capability_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![Capability::NetworkClient],
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_capability_empty() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![],
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Capabilities cannot be empty");
}

// ============================================================================
// Test: Validation - TimeWindow
// ============================================================================

#[test]
fn test_validate_time_window_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5], // Monday-Friday
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_time_window_invalid_start_hour() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 24,
        end_hour: 17,
        days: vec![],
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Hours must be 0-23");
}

#[test]
fn test_validate_time_window_invalid_end_hour() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 25,
        days: vec![],
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Hours must be 0-23");
}

#[test]
fn test_validate_time_window_invalid_day() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 7], // 7 is invalid
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Days must be 0-6");
}

// ============================================================================
// Test: Validation - Custom
// ============================================================================

#[test]
fn test_validate_custom_valid() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Custom {
        expression: "workload.type == 'native'".to_string(),
        variables: HashMap::new(),
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_custom_empty() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Custom {
        expression: String::new(),
        variables: HashMap::new(),
    };
    let result = evaluator.validate_condition(&condition);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Custom expression cannot be empty");
}

// ============================================================================
// Test: Validation - Composite
// ============================================================================

#[test]
fn test_validate_composite_and() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![
            PolicyCondition::Always,
            PolicyCondition::WorkloadType {
                workload_types: vec!["native".to_string()],
            },
        ],
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

#[test]
fn test_validate_composite_with_invalid_child() {
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
}

#[test]
fn test_validate_composite_nested() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![
            PolicyCondition::Composite {
                operator: LogicalOperator::And,
                conditions: vec![
                    PolicyCondition::Always,
                    PolicyCondition::WorkloadType {
                        workload_types: vec!["native".to_string()],
                    },
                ],
            },
            PolicyCondition::Never,
        ],
    };
    assert!(evaluator.validate_condition(&condition).is_ok());
}

// ============================================================================
// Test: Evaluation - Always/Never
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_always() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Always;
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_never() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Never;
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - WorkloadType
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_workload_type_match() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["native".to_string(), "wasm".to_string()],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_workload_type_no_match() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::WorkloadType {
        workload_types: vec!["container".to_string(), "wasm".to_string()],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - RequiresCapability
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_capability_match() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![Capability::NetworkClient],
    };
    let mut context = create_test_context_native();
    let mut caps = HashSet::new();
    caps.insert(Capability::NetworkClient);
    context.requested_capabilities = caps;

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_capability_no_match() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::RequiresCapability {
        capabilities: vec![Capability::NetworkClient],
    };
    let context = create_test_context_native(); // No NetworkClient capability

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - ResourceUsage
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_resource_usage() {
    let evaluator = ConditionEvaluator::new();
    // Use ceilings that always pass on any real machine:
    // 100% CPU and 1 TiB memory. This validates that the evaluator reads
    // actual sysinfo and returns Ok(true) when below these generous bounds.
    let condition = PolicyCondition::ResourceUsage {
        cpu_percent: Some(100.0),
        memory_mb: Some(1_000_000), // 1 TiB in MB — exceeds any real machine
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

// ============================================================================
// Test: Evaluation - TimeWindow
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_time_window_any_day() {
    let evaluator = ConditionEvaluator::new();
    // Empty days means any day
    let condition = PolicyCondition::TimeWindow {
        start_hour: 0,
        end_hour: 23,
        days: vec![],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap()); // 24-hour window, any day
}

// ============================================================================
// Test: Evaluation - UserContext
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_user_context_match_user() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::UserContext {
        users: vec!["testuser".to_string()],
        groups: vec![],
    };
    let context = create_test_context_with_user();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_user_context_match_group() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::UserContext {
        users: vec![],
        groups: vec!["admin".to_string()],
    };
    let context = create_test_context_with_user();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_user_context_no_user_info() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::UserContext {
        users: vec![],
        groups: vec![],
    };
    let context = create_test_context_native(); // No user info

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap()); // Empty users/groups with no user info => true
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_user_context_no_match() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::UserContext {
        users: vec!["otheruser".to_string()],
        groups: vec![],
    };
    let context = create_test_context_with_user();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - Composite AND
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_and_all_true() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_and_one_false() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Never],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_and_all_false() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Never],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - Composite OR
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_or_all_true() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_or_one_true() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_or_all_false() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Or,
        conditions: vec![PolicyCondition::Never, PolicyCondition::Never],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// ============================================================================
// Test: Evaluation - Composite NOT
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_not_true() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Always],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_not_false() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Never],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_composite_not_invalid_count() {
    let evaluator = ConditionEvaluator::new();
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Never], // Should be exactly 1
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_err());
}

// ============================================================================
// Test: Evaluation - Complex Nested Conditions
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_complex_nested_and_or() {
    let evaluator = ConditionEvaluator::new();
    // (Always AND (Never OR Always)) should be true
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![
            PolicyCondition::Always,
            PolicyCondition::Composite {
                operator: LogicalOperator::Or,
                conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
            },
        ],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_complex_triple_nested() {
    let evaluator = ConditionEvaluator::new();
    // NOT(Always AND (Never OR Always)) should be false
    let condition = PolicyCondition::Composite {
        operator: LogicalOperator::Not,
        conditions: vec![PolicyCondition::Composite {
            operator: LogicalOperator::And,
            conditions: vec![
                PolicyCondition::Always,
                PolicyCondition::Composite {
                    operator: LogicalOperator::Or,
                    conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
                },
            ],
        }],
    };
    let context = create_test_context_native();

    let result = evaluator.evaluate_condition(&condition, &context).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
