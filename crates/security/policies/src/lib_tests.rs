// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for security policies library
//!
//! This module provides comprehensive unit tests for the security policies crate,
//! focusing on code paths that aren't covered by integration tests.

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use toadstool::security::Capability;

    // ============================================================================
    // Evaluator Tests - Core Functionality
    // ============================================================================

    #[test]
    fn test_evaluator_creation() {
        let evaluator = ConditionEvaluator::new();
        assert!(std::mem::size_of_val(&evaluator) > 0);
    }

    #[test]
    fn test_evaluator_default() {
        let evaluator = ConditionEvaluator::default();
        assert!(std::mem::size_of_val(&evaluator) > 0);
    }

    #[test]
    fn test_validate_always_condition() {
        let evaluator = ConditionEvaluator::new();
        let result = evaluator.validate_condition(&PolicyCondition::Always);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_never_condition() {
        let evaluator = ConditionEvaluator::new();
        let result = evaluator.validate_condition(&PolicyCondition::Never);
        assert!(result.is_ok());
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

    #[test]
    fn test_validate_workload_type_valid() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::WorkloadType {
            workload_types: vec!["compute".to_string()],
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
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

    #[test]
    fn test_validate_capability_valid() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::RequiresCapability {
            capabilities: vec![Capability::NetworkClient],
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_time_window_invalid_hour() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::TimeWindow {
            start_hour: 25, // Invalid
            end_hour: 10,
            days: vec![0, 1],
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
            days: vec![0, 7], // Invalid day (7)
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Days must be 0-6");
    }

    #[test]
    fn test_validate_time_window_valid() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::TimeWindow {
            start_hour: 9,
            end_hour: 17,
            days: vec![1, 2, 3, 4, 5], // Weekdays
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_empty_expression() {
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
    fn test_validate_custom_valid() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::Custom {
            expression: "workload.priority > 5".to_string(),
            variables: HashMap::new(),
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_composite_and() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::Composite {
            operator: LogicalOperator::And,
            conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_composite_or() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::Composite {
            operator: LogicalOperator::Or,
            conditions: vec![PolicyCondition::Never, PolicyCondition::Always],
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_composite_not() {
        let evaluator = ConditionEvaluator::new();
        let condition = PolicyCondition::Composite {
            operator: LogicalOperator::Not,
            conditions: vec![PolicyCondition::Never],
        };
        let result = evaluator.validate_condition(&condition);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_composite_nested() {
        let evaluator = ConditionEvaluator::new();
        let inner = PolicyCondition::Composite {
            operator: LogicalOperator::And,
            conditions: vec![PolicyCondition::Always, PolicyCondition::Always],
        };
        let outer = PolicyCondition::Composite {
            operator: LogicalOperator::Or,
            conditions: vec![inner, PolicyCondition::Never],
        };
        let result = evaluator.validate_condition(&outer);
        assert!(result.is_ok());
    }

    // ============================================================================
    // Executor Tests - Action Execution
    // ============================================================================

    #[test]
    fn test_executor_creation() {
        let _executor = ActionExecutor::new();
        // Executor created successfully - that's the test
    }

    #[test]
    fn test_executor_default() {
        let _ = ActionExecutor;
        // Executor created successfully - that's the test
    }

    // ============================================================================
    // Policy Type Tests - Construction and Validation
    // ============================================================================

    #[test]
    fn test_security_policy_creation() {
        let policy = SecurityPolicy {
            id: "test-policy".to_string(),
            name: "Test Policy".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test policy description".to_string()),
            author: Some("ToadStool Team".to_string()),
            created_at: SystemTime::now(),
            modified_at: SystemTime::now(),
            rules: vec![],
            inherits: vec![],
            metadata: HashMap::new(),
            signature: None,
        };
        assert_eq!(policy.id, "test-policy");
        assert_eq!(policy.version, "1.0.0");
    }

    #[test]
    fn test_policy_rule_with_always_condition() {
        let rule = PolicyRule {
            id: "allow-all".to_string(),
            name: "Allow All Rule".to_string(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 100,
            enabled: true,
            description: Some("Allow all workloads".to_string()),
        };
        assert_eq!(rule.id, "allow-all");
        assert!(matches!(rule.action, PolicyAction::Allow));
    }

    #[test]
    fn test_policy_rule_with_deny_action() {
        let rule = PolicyRule {
            id: "deny-untrusted".to_string(),
            name: "Deny Untrusted Rule".to_string(),
            condition: PolicyCondition::Never,
            action: PolicyAction::DenyWithMessage {
                message: "Untrusted source".to_string(),
            },
            priority: 200,
            enabled: true,
            description: None,
        };
        assert_eq!(rule.priority, 200);
        assert!(matches!(rule.action, PolicyAction::DenyWithMessage { .. }));
    }

    #[test]
    fn test_policy_evaluation_result_allow() {
        use std::time::{Duration, SystemTime};
        use uuid::Uuid;

        let result = PolicyEvaluationResult {
            evaluation_id: Uuid::new_v4(),
            policy_id: "test-policy".to_string(),
            result: PolicyResult::Allow,
            applied_rules: vec![],
            security_modifications: vec![],
            resource_modifications: vec![],
            warnings: vec![],
            evaluation_duration: Duration::from_millis(5),
            timestamp: SystemTime::now(),
        };
        assert!(matches!(result.result, PolicyResult::Allow));
    }

    #[test]
    fn test_policy_evaluation_result_deny() {
        use std::time::{Duration, SystemTime};
        use uuid::Uuid;

        let result = PolicyEvaluationResult {
            evaluation_id: Uuid::new_v4(),
            policy_id: "test-policy".to_string(),
            result: PolicyResult::Deny,
            applied_rules: vec![],
            security_modifications: vec![],
            resource_modifications: vec![],
            warnings: vec![],
            evaluation_duration: Duration::from_millis(3),
            timestamp: SystemTime::now(),
        };
        assert!(matches!(result.result, PolicyResult::Deny));
    }

    #[test]
    fn test_policy_action_modify_security() {
        use toadstool::security::IsolationLevel;
        let action = PolicyAction::ModifySecurityContext {
            isolation_level: Some(IsolationLevel::Maximum),
            add_capabilities: vec![],
            remove_capabilities: vec![],
        };
        assert!(matches!(action, PolicyAction::ModifySecurityContext { .. }));
    }

    #[test]
    fn test_policy_action_modify_resources() {
        let action = PolicyAction::ApplyResourceLimits {
            cpu_percent: Some(2.0),
            memory_mb: Some(1024),
            network_mbps: Some(100.0),
        };
        assert!(matches!(action, PolicyAction::ApplyResourceLimits { .. }));
    }

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

    // ============================================================================
    // Policy Manager Config Tests
    // ============================================================================

    #[test]
    fn test_policy_manager_config_default() {
        let config = PolicyManagerConfig::default();
        assert!(config.max_composition_depth > 0);
        assert!(config.validation_timeout_ms > 0);
        assert!(config.cache_ttl_hours > 0);
    }

    #[test]
    fn test_policy_manager_config_custom() {
        use std::path::PathBuf;
        let config = PolicyManagerConfig {
            policy_dir: PathBuf::from("/custom/policies"),
            cache_enabled: true,
            cache_ttl_hours: 48,
            strict_enforcement: true,
            default_violation_action: types::ViolationAction::Terminate,
            max_composition_depth: 5,
            validation_timeout_ms: 10_000,
        };
        assert_eq!(config.max_composition_depth, 5);
        assert_eq!(config.validation_timeout_ms, 10_000);
        assert_eq!(config.cache_ttl_hours, 48);
    }

    // ============================================================================
    // Policy Warning Tests
    // ============================================================================

    #[test]
    fn test_policy_warning_creation() {
        let warning = PolicyWarning {
            level: "warning".to_string(),
            message: "Resource limit adjusted".to_string(),
            rule_id: Some("rule-1".to_string()),
        };
        assert_eq!(warning.level, "warning");
        assert!(warning.rule_id.is_some());
    }

    // ============================================================================
    // Applied Rule Tests
    // ============================================================================

    #[test]
    fn test_applied_rule_creation() {
        let applied = AppliedRule {
            rule_id: "rule-1".to_string(),
            rule_name: "Test Rule".to_string(),
            action: PolicyAction::Allow,
            priority: 100,
            condition_matched: true,
        };
        assert_eq!(applied.rule_id, "rule-1");
        assert!(matches!(applied.action, PolicyAction::Allow));
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
    fn test_violation_action_alert() {
        let action = ViolationAction::Alert;
        assert!(matches!(action, ViolationAction::Alert));
    }

    #[test]
    fn test_violation_action_block() {
        let action = ViolationAction::Block;
        assert!(matches!(action, ViolationAction::Block));
    }

    #[test]
    fn test_violation_action_quarantine() {
        let action = ViolationAction::Quarantine;
        assert!(matches!(action, ViolationAction::Quarantine));
    }
}
