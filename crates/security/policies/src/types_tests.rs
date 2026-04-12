// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

#[test]
fn policy_manager_config_default() {
    let config = PolicyManagerConfig::default();
    assert!(config.cache_enabled);
    assert!(config.strict_enforcement);
    assert_eq!(config.cache_ttl_hours, 24);
    assert_eq!(config.max_composition_depth, 10);
    assert_eq!(config.validation_timeout_ms, 5000);
}

#[test]
fn policy_condition_always_serde() {
    let cond = PolicyCondition::Always;
    let json = serde_json::to_string(&cond).unwrap();
    let deser: PolicyCondition = serde_json::from_str(&json).unwrap();
    assert!(matches!(deser, PolicyCondition::Always));
}

#[test]
fn policy_condition_never_serde() {
    let cond = PolicyCondition::Never;
    let json = serde_json::to_string(&cond).unwrap();
    let deser: PolicyCondition = serde_json::from_str(&json).unwrap();
    assert!(matches!(deser, PolicyCondition::Never));
}

#[test]
fn policy_condition_composite() {
    let cond = PolicyCondition::Composite {
        operator: LogicalOperator::And,
        conditions: vec![PolicyCondition::Always, PolicyCondition::Never],
    };
    let json = serde_json::to_string(&cond).unwrap();
    let deser: PolicyCondition = serde_json::from_str(&json).unwrap();
    if let PolicyCondition::Composite {
        operator,
        conditions,
    } = deser
    {
        assert!(matches!(operator, LogicalOperator::And));
        assert_eq!(conditions.len(), 2);
    } else {
        unreachable!("expected Composite");
    }
}

#[test]
fn policy_action_variants_serde() {
    let actions: Vec<PolicyAction> = vec![
        PolicyAction::Allow,
        PolicyAction::Deny,
        PolicyAction::AllowWithWarning {
            message: "caution".to_string(),
        },
        PolicyAction::DenyWithMessage {
            message: "blocked".to_string(),
        },
        PolicyAction::LogAndContinue {
            level: "warn".to_string(),
            message: "logged".to_string(),
        },
        PolicyAction::RequireAuthentication {
            method: "mfa".to_string(),
        },
    ];
    for action in &actions {
        let json = serde_json::to_string(action).unwrap();
        let _deser: PolicyAction = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn violation_action_variants_serde() {
    let actions = [
        ViolationAction::Terminate,
        ViolationAction::Block,
        ViolationAction::LogAndContinue,
        ViolationAction::Quarantine,
        ViolationAction::Alert,
    ];
    for a in &actions {
        let json = serde_json::to_string(a).unwrap();
        let deser: ViolationAction = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{a:?}"), format!("{deser:?}"));
    }
}

#[test]
fn logical_operator_serde() {
    let ops = [
        LogicalOperator::And,
        LogicalOperator::Or,
        LogicalOperator::Not,
    ];
    for op in &ops {
        let json = serde_json::to_string(op).unwrap();
        let _: LogicalOperator = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn policy_result_equality() {
    assert_eq!(PolicyResult::Allow, PolicyResult::Allow);
    assert_ne!(PolicyResult::Allow, PolicyResult::Deny);
    assert_ne!(PolicyResult::Modified, PolicyResult::RequiresAuth);
}

#[test]
fn file_policy_config_default() {
    let config = FilePolicyConfig::default();
    assert!(config.policy_directory.is_empty());
    assert!(!config.cache_enabled);
    assert_eq!(config.cache_ttl_seconds, 0);
}

#[test]
fn policy_rule_serde() {
    let rule = PolicyRule {
        id: "r1".to_string(),
        name: "test_rule".to_string(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 100,
        enabled: true,
        description: Some("A test rule".to_string()),
    };
    let json = serde_json::to_string(&rule).unwrap();
    let deser: PolicyRule = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.id, "r1");
    assert_eq!(deser.priority, 100);
    assert!(deser.enabled);
}

#[test]
fn security_policy_serde() {
    let policy = SecurityPolicy {
        id: "p1".to_string(),
        name: "test_policy".to_string(),
        version: "1.0".to_string(),
        description: None,
        author: Some("test".to_string()),
        created_at: SystemTime::UNIX_EPOCH,
        modified_at: SystemTime::UNIX_EPOCH,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    };
    let json = serde_json::to_string(&policy).unwrap();
    let deser: SecurityPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.id, "p1");
    assert!(deser.rules.is_empty());
}

#[test]
fn policy_condition_workload_type() {
    let cond = PolicyCondition::WorkloadType {
        workload_types: vec!["native".to_string(), "wasm".to_string()],
    };
    let json = serde_json::to_string(&cond).unwrap();
    assert!(json.contains("native"));
}

#[test]
fn policy_condition_resource_usage() {
    let cond = PolicyCondition::ResourceUsage {
        cpu_percent: Some(90.0),
        memory_mb: Some(4096),
    };
    let json = serde_json::to_string(&cond).unwrap();
    let deser: PolicyCondition = serde_json::from_str(&json).unwrap();
    if let PolicyCondition::ResourceUsage {
        cpu_percent,
        memory_mb,
    } = deser
    {
        assert_eq!(cpu_percent, Some(90.0));
        assert_eq!(memory_mb, Some(4096));
    } else {
        unreachable!("wrong variant");
    }
}

#[test]
fn policy_condition_time_window() {
    let cond = PolicyCondition::TimeWindow {
        start_hour: 9,
        end_hour: 17,
        days: vec![1, 2, 3, 4, 5],
    };
    let json = serde_json::to_string(&cond).unwrap();
    assert!(json.contains("TimeWindow"));
}
