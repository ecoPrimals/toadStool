// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration-style unit tests for [`toadstool_security_policies::manager::FilePolicyManager`]
//! and the [`toadstool_security_policies::PolicyManager`] trait, covering configuration, I/O
//! edge cases, evaluation, composition, and serde stability for persisted policy data.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use tempfile::TempDir;
use toadstool::security::{FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext};
use toadstool::workload::{ExecutableSource, WorkloadSpec};
use toadstool_security_policies::types::*;
use toadstool_security_policies::{FilePolicyManager, PolicyManager, PolicyManagerConfig};

fn config_in(dir: &TempDir) -> PolicyManagerConfig {
    PolicyManagerConfig {
        policy_dir: dir.path().to_path_buf(),
        cache_enabled: true,
        cache_ttl_hours: 24,
        strict_enforcement: true,
        default_violation_action: ViolationAction::Terminate,
        max_composition_depth: 10,
        validation_timeout_ms: 5000,
    }
}

fn base_policy(id: &str, name: &str) -> SecurityPolicy {
    SecurityPolicy {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".to_string(),
        description: None,
        author: None,
        created_at: SystemTime::UNIX_EPOCH,
        modified_at: SystemTime::UNIX_EPOCH,
        rules: vec![],
        inherits: vec![],
        metadata: HashMap::new(),
        signature: None,
    }
}

fn eval_context_native() -> PolicyEvaluationContext {
    PolicyEvaluationContext {
        workload: WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: "/bin/true".into(),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        },
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Standard,
            capabilities: vec![],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        },
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "h".into(),
            os_type: "Linux".into(),
            os_version: "1".into(),
            architecture: "x86_64".into(),
            cpu_count: 1,
            memory_total_mb: 1024,
            load_average: 0.0,
            uptime_seconds: 1,
        },
        timestamp: SystemTime::UNIX_EPOCH,
        variables: HashMap::new(),
    }
}

fn eval_context_container() -> PolicyEvaluationContext {
    PolicyEvaluationContext {
        workload: WorkloadSpec::Container {
            image: "alpine:latest".into(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        },
        security_context: SecurityContext {
            isolation_level: IsolationLevel::Standard,
            capabilities: vec![],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        },
        requested_capabilities: HashSet::new(),
        user_info: None,
        system_info: SystemInfo {
            hostname: "h".into(),
            os_type: "Linux".into(),
            os_version: "1".into(),
            architecture: "x86_64".into(),
            cpu_count: 1,
            memory_total_mb: 1024,
            load_average: 0.0,
            uptime_seconds: 1,
        },
        timestamp: SystemTime::UNIX_EPOCH,
        variables: HashMap::new(),
    }
}

#[tokio::test]
async fn file_policy_manager_new_creates_policy_directory_and_uses_config_defaults() {
    let tmp = TempDir::new().unwrap();
    let policy_dir = tmp.path().join("policies");
    let config = PolicyManagerConfig {
        policy_dir: policy_dir.clone(),
        cache_enabled: true,
        cache_ttl_hours: 1,
        strict_enforcement: false,
        default_violation_action: ViolationAction::Block,
        max_composition_depth: 3,
        validation_timeout_ms: 100,
    };
    let mgr = FilePolicyManager::new(config).unwrap();
    assert!(policy_dir.is_dir());
    let _ = std::mem::size_of_val(&mgr);
}

#[test]
fn policy_manager_config_default_matches_documented_policy_manager_expectations() {
    let c = PolicyManagerConfig::default();
    assert!(c.cache_enabled);
    assert!(c.strict_enforcement);
    assert_eq!(c.cache_ttl_hours, 24);
    assert_eq!(c.max_composition_depth, 10);
    assert_eq!(c.validation_timeout_ms, 5000);
    assert!(matches!(
        c.default_violation_action,
        ViolationAction::Terminate
    ));
}

#[test]
fn policy_manager_config_serde_json_round_trip_preserves_fields() {
    let tmp = TempDir::new().unwrap();
    let original = PolicyManagerConfig {
        policy_dir: tmp.path().join("p"),
        cache_enabled: false,
        cache_ttl_hours: 7,
        strict_enforcement: false,
        default_violation_action: ViolationAction::Alert,
        max_composition_depth: 42,
        validation_timeout_ms: 999,
    };
    let json = serde_json::to_string(&original).unwrap();
    let back: PolicyManagerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.policy_dir, original.policy_dir);
    assert!(!back.cache_enabled);
    assert_eq!(back.cache_ttl_hours, 7);
    assert_eq!(back.max_composition_depth, 42);
    assert!(matches!(
        back.default_violation_action,
        ViolationAction::Alert
    ));
}

#[tokio::test]
async fn file_policy_manager_new_succeeds_when_policy_dir_already_exists() {
    let tmp = TempDir::new().unwrap();
    let policy_dir = tmp.path().join("existing");
    std::fs::create_dir_all(&policy_dir).unwrap();
    let config = PolicyManagerConfig {
        policy_dir,
        ..config_in(&tmp)
    };
    assert!(FilePolicyManager::new(config).is_ok());
}

#[tokio::test]
async fn load_policy_returns_configuration_error_when_file_missing() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.load_policy("missing").await.is_err());
}

#[tokio::test]
async fn load_policy_parses_toml_when_toml_file_exists() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let p = base_policy("toml_only", "n");
    mgr.save_policy(&p).await.unwrap();
    let loaded = mgr.load_policy("toml_only").await.unwrap();
    assert_eq!(loaded.id, "toml_only");
}

#[tokio::test]
async fn load_policy_parses_yaml_when_no_toml_file_exists() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let p = base_policy("yaml_policy", "n");
    let yaml = serde_yaml_ng::to_string(&p).unwrap();
    std::fs::write(dir.join("yaml_policy.yaml"), yaml).unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let loaded = mgr.load_policy("yaml_policy").await.unwrap();
    assert_eq!(loaded.id, "yaml_policy");
}

#[tokio::test]
async fn load_policy_prefers_toml_over_yaml_when_both_exist() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let mut from_toml = base_policy("dup", "from_toml");
    from_toml.version = "2.0.0".into();
    let mut from_yaml = base_policy("dup", "from_yaml");
    from_yaml.version = "9.9.9".into();
    std::fs::write(dir.join("dup.toml"), toml::to_string(&from_toml).unwrap()).unwrap();
    std::fs::write(
        dir.join("dup.yaml"),
        serde_yaml_ng::to_string(&from_yaml).unwrap(),
    )
    .unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let loaded = mgr.load_policy("dup").await.unwrap();
    assert_eq!(loaded.version, "2.0.0");
}

#[tokio::test]
async fn load_policy_fails_on_invalid_toml_content() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("bad.toml"), "not valid toml {{{").unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.load_policy("bad").await.is_err());
}

#[tokio::test]
async fn load_policy_fails_on_invalid_yaml_content() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("bad.yaml"), "{").unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.load_policy("bad").await.is_err());
}

#[tokio::test]
async fn load_policy_with_zero_hour_cache_ttl_always_misses_cache_and_reloads_from_disk() {
    let tmp = TempDir::new().unwrap();
    let mut cfg = config_in(&tmp);
    cfg.cache_ttl_hours = 0;
    let mgr = FilePolicyManager::new(cfg).unwrap();
    let mut p = base_policy("volatile", "n");
    mgr.save_policy(&p).await.unwrap();
    mgr.load_policy("volatile").await.unwrap();
    p.version = "2.0.0".into();
    mgr.save_policy(&p).await.unwrap();
    let again = mgr.load_policy("volatile").await.unwrap();
    assert_eq!(again.version, "2.0.0");
}

#[tokio::test]
async fn save_policy_strict_enforcement_rejects_invalid_policy() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("x", "y");
    p.id = String::new();
    assert!(mgr.save_policy(&p).await.is_err());
}

#[tokio::test]
async fn save_policy_non_strict_allows_save_despite_validation_errors() {
    let tmp = TempDir::new().unwrap();
    let mut cfg = config_in(&tmp);
    cfg.strict_enforcement = false;
    let mgr = FilePolicyManager::new(cfg).unwrap();
    let mut p = base_policy("bad_meta", "name");
    p.id = String::new();
    assert!(mgr.save_policy(&p).await.is_ok());
}

#[tokio::test]
async fn validate_policy_collects_field_rule_and_condition_errors() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("", "");
    p.version = String::new();
    p.inherits.push(String::new());
    p.rules.push(PolicyRule {
        id: String::new(),
        name: String::new(),
        condition: PolicyCondition::WorkloadType {
            workload_types: vec![],
        },
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    });
    let errs = mgr.validate_policy(&p).await.unwrap();
    assert!(errs.iter().any(|e| e.contains("ID")));
    assert!(errs.iter().any(|e| e.contains("name")));
    assert!(errs.iter().any(|e| e.contains("version")));
    assert!(errs.iter().any(|e| e.contains("inherit from itself")));
    assert!(errs.iter().any(|e| e.contains("empty ID")));
    assert!(errs.iter().any(|e| e.contains("invalid condition")));
}

#[tokio::test]
async fn list_policies_returns_sorted_toml_stems_and_ignores_non_toml_files() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    std::fs::write(tmp.path().join("note.txt"), b"noise").unwrap();
    let b = base_policy("b", "b");
    let a = base_policy("a", "a");
    mgr.save_policy(&b).await.unwrap();
    mgr.save_policy(&a).await.unwrap();
    let listed = mgr.list_policies().await.unwrap();
    assert_eq!(listed, vec!["a".to_string(), "b".to_string()]);
}

#[tokio::test]
async fn evaluate_policy_runs_matching_rules_highest_priority_first_and_last_action_sets_result() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("prio", "p");
    p.rules = vec![
        PolicyRule {
            id: "low".into(),
            name: "low".into(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 1,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "high".into(),
            name: "high".into(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Deny,
            priority: 100,
            enabled: true,
            description: None,
        },
        PolicyRule {
            id: "off".into(),
            name: "off".into(),
            condition: PolicyCondition::Always,
            action: PolicyAction::Allow,
            priority: 200,
            enabled: false,
            description: None,
        },
    ];
    mgr.save_policy(&p).await.unwrap();
    let res = mgr
        .evaluate_policy("prio", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(res.result, PolicyResult::Allow);
    let order: Vec<_> = res
        .applied_rules
        .iter()
        .map(|r| r.rule_id.as_str())
        .collect();
    assert_eq!(order, vec!["high", "low"]);
}

#[tokio::test]
async fn evaluate_policy_never_condition_does_not_apply_rule() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("never_rule", "n");
    p.rules = vec![PolicyRule {
        id: "n".into(),
        name: "n".into(),
        condition: PolicyCondition::Never,
        action: PolicyAction::Deny,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&p).await.unwrap();
    let res = mgr
        .evaluate_policy("never_rule", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(res.result, PolicyResult::Allow);
    assert!(res.applied_rules.is_empty());
}

#[tokio::test]
async fn evaluate_policy_workload_type_matches_native_and_container_contexts() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut native_ok = base_policy("wt_native", "n");
    native_ok.rules = vec![PolicyRule {
        id: "r".into(),
        name: "r".into(),
        condition: PolicyCondition::WorkloadType {
            workload_types: vec!["native".into()],
        },
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&native_ok).await.unwrap();
    let r1 = mgr
        .evaluate_policy("wt_native", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(r1.result, PolicyResult::Allow);
    assert_eq!(r1.applied_rules.len(), 1);

    let mut container_ok = base_policy("wt_container", "c");
    container_ok.rules = vec![PolicyRule {
        id: "r".into(),
        name: "r".into(),
        condition: PolicyCondition::WorkloadType {
            workload_types: vec!["container".into()],
        },
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&container_ok).await.unwrap();
    let r2 = mgr
        .evaluate_policy("wt_container", &eval_context_container())
        .await
        .unwrap();
    assert_eq!(r2.result, PolicyResult::Allow);
}

#[tokio::test]
async fn evaluate_policy_inherits_parent_deny_when_child_has_no_rules() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut parent = base_policy("par", "p");
    parent.rules = vec![PolicyRule {
        id: "d".into(),
        name: "d".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Deny,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&parent).await.unwrap();
    let mut child = base_policy("child", "c");
    child.inherits = vec!["par".into()];
    child.rules = vec![];
    mgr.save_policy(&child).await.unwrap();
    let res = mgr
        .evaluate_policy("child", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(res.result, PolicyResult::Deny);
    assert!(res.applied_rules.iter().any(|r| r.rule_id == "d"));
}

#[tokio::test]
async fn evaluate_policy_child_allow_rule_overrides_inherited_parent_deny() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut parent = base_policy("p2", "p");
    parent.rules = vec![PolicyRule {
        id: "d".into(),
        name: "d".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Deny,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&parent).await.unwrap();
    let mut child = base_policy("child2", "c");
    child.inherits = vec!["p2".into()];
    child.rules = vec![PolicyRule {
        id: "a".into(),
        name: "a".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 10,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&child).await.unwrap();
    let res = mgr
        .evaluate_policy("child2", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(res.result, PolicyResult::Allow);
    assert!(res.applied_rules.iter().any(|r| r.rule_id == "d"));
    assert!(res.applied_rules.iter().any(|r| r.rule_id == "a"));
}

#[tokio::test]
async fn evaluate_policy_merge_retains_parent_modified_when_child_has_no_rules() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut parent = base_policy("pmod", "p");
    parent.rules = vec![PolicyRule {
        id: "m".into(),
        name: "m".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::ApplyResourceLimits {
            cpu_percent: Some(10.0),
            memory_mb: None,
            network_mbps: None,
        },
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&parent).await.unwrap();
    let mut child = base_policy("cmod", "c");
    child.inherits = vec!["pmod".into()];
    child.rules = vec![];
    mgr.save_policy(&child).await.unwrap();
    let res = mgr
        .evaluate_policy("cmod", &eval_context_native())
        .await
        .unwrap();
    assert_eq!(res.result, PolicyResult::Modified);
}

#[tokio::test]
async fn evaluate_policy_returns_error_when_rule_condition_evaluation_errors() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("net", "n");
    p.rules = vec![PolicyRule {
        id: "x".into(),
        name: "x".into(),
        condition: PolicyCondition::NetworkAccess {
            hosts: vec![],
            ports: vec![],
        },
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&p).await.unwrap();
    assert!(
        mgr.evaluate_policy("net", &eval_context_native())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn evaluate_policy_returns_error_for_invalid_composite_not_arity() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("notbad", "n");
    p.rules = vec![PolicyRule {
        id: "x".into(),
        name: "x".into(),
        condition: PolicyCondition::Composite {
            operator: LogicalOperator::Not,
            conditions: vec![PolicyCondition::Always, PolicyCondition::Never],
        },
        action: PolicyAction::Allow,
        priority: 1,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&p).await.unwrap();
    assert!(
        mgr.evaluate_policy("notbad", &eval_context_native())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn compose_policies_errors_on_empty_input() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.compose_policies(&[]).await.is_err());
}

#[tokio::test]
async fn compose_policies_merges_rules_and_prefixed_metadata() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut a = base_policy("ca", "a");
    a.metadata.insert("k".into(), serde_json::json!(1));
    a.rules = vec![PolicyRule {
        id: "r1".into(),
        name: "r1".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Allow,
        priority: 5,
        enabled: true,
        description: None,
    }];
    let mut b = base_policy("cb", "b");
    b.metadata.insert("k".into(), serde_json::json!(2));
    b.rules = vec![PolicyRule {
        id: "r2".into(),
        name: "r2".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::Deny,
        priority: 10,
        enabled: true,
        description: None,
    }];
    mgr.save_policy(&a).await.unwrap();
    mgr.save_policy(&b).await.unwrap();
    let composed = mgr
        .compose_policies(&["ca".into(), "cb".into()])
        .await
        .unwrap();
    assert_eq!(composed.rules.len(), 2);
    assert!(composed.metadata.contains_key("ca_k"));
    assert!(composed.metadata.contains_key("cb_k"));
    assert!(composed.id.starts_with("composed_"));
}

#[tokio::test]
async fn compose_policies_errors_when_dependency_policy_missing() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.compose_policies(&["nope".into()]).await.is_err());
}

#[tokio::test]
async fn get_policy_dependencies_returns_empty_when_policy_file_missing() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let deps = mgr.get_policy_dependencies("ghost").await.unwrap();
    assert!(deps.is_empty());
}

#[tokio::test]
async fn get_policy_dependencies_returns_inherits_when_present() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("deps", "d");
    p.inherits = vec!["p1".into(), "p2".into()];
    mgr.save_policy(&p).await.unwrap();
    let deps = mgr.get_policy_dependencies("deps").await.unwrap();
    assert_eq!(deps, vec!["p1", "p2"]);
}

#[tokio::test]
async fn delete_policy_succeeds_when_file_missing_and_removes_cache_entry_when_present() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    assert!(mgr.delete_policy("nope").await.is_ok());
    let p = base_policy("gone", "g");
    mgr.save_policy(&p).await.unwrap();
    mgr.load_policy("gone").await.unwrap();
    mgr.delete_policy("gone").await.unwrap();
    assert!(mgr.load_policy("gone").await.is_err());
}

#[tokio::test]
async fn security_policy_round_trips_through_toml_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let mgr = FilePolicyManager::new(config_in(&tmp)).unwrap();
    let mut p = base_policy("round", "r");
    p.description = Some("d".into());
    p.rules = vec![PolicyRule {
        id: "r".into(),
        name: "r".into(),
        condition: PolicyCondition::Always,
        action: PolicyAction::AllowWithWarning {
            message: "w".into(),
        },
        priority: 3,
        enabled: true,
        description: Some("x".into()),
    }];
    mgr.save_policy(&p).await.unwrap();
    let loaded = mgr.load_policy("round").await.unwrap();
    assert_eq!(loaded.id, p.id);
    assert_eq!(loaded.rules.len(), 1);
    match &loaded.rules[0].action {
        PolicyAction::AllowWithWarning { message } => assert_eq!(message, "w"),
        _ => panic!("expected AllowWithWarning"),
    }
}
