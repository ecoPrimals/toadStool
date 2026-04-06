// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security policy type definitions
//!
//! This module contains all the core type definitions for the security policy system,
//! including policy structures, rules, conditions, actions, and evaluation results.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use toadstool::security::{Capability, IsolationLevel, SecurityContext};
use toadstool::workload::WorkloadSpec;

/// Policy management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyManagerConfig {
    /// Policy storage directory
    pub policy_dir: PathBuf,
    /// Enable policy caching
    pub cache_enabled: bool,
    /// Cache TTL in hours
    pub cache_ttl_hours: u64,
    /// Enable strict policy enforcement
    pub strict_enforcement: bool,
    /// Default policy violation action
    pub default_violation_action: ViolationAction,
    /// Maximum policy composition depth
    pub max_composition_depth: u32,
    /// Policy validation timeout in milliseconds
    pub validation_timeout_ms: u64,
}

impl Default for PolicyManagerConfig {
    fn default() -> Self {
        let env = toadstool_common::platform_paths::PathEnv::from_env();
        let paths = toadstool_common::platform_paths::PlatformPaths::new(&env);

        Self {
            policy_dir: paths.config_dir().join("toadstool").join("policies"),
            cache_enabled: true,
            cache_ttl_hours: 24,
            strict_enforcement: true,
            default_violation_action: ViolationAction::Terminate,
            max_composition_depth: 10,
            validation_timeout_ms: 5000,
        }
    }
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Unique policy identifier
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy version
    pub version: String,
    /// Policy description
    pub description: Option<String>,
    /// Policy author/organization
    pub author: Option<String>,
    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Last modified timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub modified_at: SystemTime,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
    /// Policy inheritance (parent policies)
    pub inherits: Vec<String>,
    /// Policy metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Policy signature for integrity verification
    pub signature: Option<String>,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: PolicyCondition,
    /// Rule action
    pub action: PolicyAction,
    /// Rule priority (higher = more important)
    pub priority: u32,
    /// Rule enabled status
    pub enabled: bool,
    /// Rule description
    pub description: Option<String>,
}

/// Policy condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Always apply (catch-all)
    Always,
    /// Never apply (disabled)
    Never,
    /// Workload type condition.
    WorkloadType {
        /// Allowed workload types (e.g. native, wasm, container).
        workload_types: Vec<String>,
    },
    /// Capability requirement condition.
    RequiresCapability {
        /// Required capabilities.
        capabilities: Vec<Capability>,
    },
    /// Resource usage condition.
    ResourceUsage {
        /// Maximum CPU percent (0–100).
        cpu_percent: Option<f64>,
        /// Maximum memory in MB.
        memory_mb: Option<u64>,
    },
    /// Time-based condition.
    TimeWindow {
        /// Start hour (0–23).
        start_hour: u8,
        /// End hour (0–23).
        end_hour: u8,
        /// Days of week (0=Sunday–6=Saturday).
        days: Vec<u8>,
    },
    /// User/group condition.
    UserContext {
        /// Allowed usernames.
        users: Vec<String>,
        /// Allowed groups.
        groups: Vec<String>,
    },
    /// Network access condition.
    NetworkAccess {
        /// Allowed hosts.
        hosts: Vec<String>,
        /// Allowed ports.
        ports: Vec<u16>,
    },
    /// File system access condition.
    FileSystemAccess {
        /// Allowed paths.
        paths: Vec<PathBuf>,
        /// Allowed operations (read, write, etc.).
        operations: Vec<String>,
    },
    /// Custom condition with expression.
    Custom {
        /// Expression string (Phase 2+ evaluation).
        expression: String,
        /// Variables for expression evaluation.
        variables: HashMap<String, serde_json::Value>,
    },
    /// Composite condition (AND/OR/NOT).
    Composite {
        /// Logical operator.
        operator: LogicalOperator,
        /// Child conditions.
        conditions: Vec<Self>,
    },
}

/// Logical operators for composite conditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    /// All conditions must match.
    And,
    /// Any condition must match.
    Or,
    /// Negate the single child condition.
    Not,
}

/// Policy action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
    /// Allow with warnings.
    AllowWithWarning {
        /// Warning message.
        message: String,
    },
    /// Deny with custom message.
    DenyWithMessage {
        /// Denial message.
        message: String,
    },
    /// Modify security context.
    ModifySecurityContext {
        /// New isolation level (if any).
        isolation_level: Option<IsolationLevel>,
        /// Capabilities to add.
        add_capabilities: Vec<Capability>,
        /// Capabilities to remove.
        remove_capabilities: Vec<Capability>,
    },
    /// Apply resource limits.
    ApplyResourceLimits {
        /// CPU limit (percent).
        cpu_percent: Option<f64>,
        /// Memory limit (MB).
        memory_mb: Option<u64>,
        /// Network bandwidth limit (Mbps).
        network_mbps: Option<f64>,
    },
    /// Require additional authentication.
    RequireAuthentication {
        /// Authentication method.
        method: String,
    },
    /// Log and continue execution.
    LogAndContinue {
        /// Log level.
        level: String,
        /// Log message.
        message: String,
    },
    /// Custom action with parameters.
    Custom {
        /// Action identifier.
        action: String,
        /// Action parameters.
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Policy violation actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationAction {
    /// Terminate execution immediately
    Terminate,
    /// Block operation and continue
    Block,
    /// Log violation and continue
    LogAndContinue,
    /// Quarantine workload
    Quarantine,
    /// Send alert to administrators
    Alert,
}

/// Policy evaluation result
#[derive(Debug, Clone)]
pub struct PolicyEvaluationResult {
    /// Evaluation unique ID
    pub evaluation_id: Uuid,
    /// Policy that was evaluated
    pub policy_id: String,
    /// Overall result
    pub result: PolicyResult,
    /// Applied rules
    pub applied_rules: Vec<AppliedRule>,
    /// Security context modifications
    pub security_modifications: Vec<SecurityModification>,
    /// Resource limit modifications
    pub resource_modifications: Vec<ResourceModification>,
    /// Warnings generated
    pub warnings: Vec<PolicyWarning>,
    /// Evaluation duration
    pub evaluation_duration: Duration,
    /// Evaluation timestamp
    pub timestamp: SystemTime,
}

/// Policy evaluation result types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    /// Operation allowed.
    Allow,
    /// Operation denied.
    Deny,
    /// Context modified (allow with changes).
    Modified,
    /// Additional authentication required.
    RequiresAuth,
}

/// Applied rule information from evaluation.
#[derive(Debug, Clone)]
pub struct AppliedRule {
    /// Rule identifier.
    pub rule_id: String,
    /// Rule name.
    pub rule_name: String,
    /// Action that was applied.
    pub action: PolicyAction,
    /// Rule priority.
    pub priority: u32,
    /// Whether the condition matched.
    pub condition_matched: bool,
}

/// Security context modification from policy.
#[derive(Debug, Clone)]
pub struct SecurityModification {
    /// Type of modification.
    pub modification_type: String,
    /// Previous value.
    pub old_value: serde_json::Value,
    /// New value.
    pub new_value: serde_json::Value,
    /// Reason for modification.
    pub reason: String,
}

/// Resource limit modification from policy.
#[derive(Debug, Clone)]
pub struct ResourceModification {
    /// Resource type (e.g. cpu_percent, memory_mb).
    pub resource_type: String,
    /// Previous limit (if any).
    pub old_limit: Option<f64>,
    /// New limit.
    pub new_limit: f64,
    /// Reason for modification.
    pub reason: String,
}

/// Policy warning generated during evaluation.
#[derive(Debug, Clone)]
pub struct PolicyWarning {
    /// Warning level (info, warning, error).
    pub level: String,
    /// Warning message.
    pub message: String,
    /// Rule that generated the warning (if any).
    pub rule_id: Option<String>,
}

/// Policy evaluation context
#[derive(Debug, Clone)]
pub struct PolicyEvaluationContext {
    /// Workload specification
    pub workload: WorkloadSpec,
    /// Current security context
    pub security_context: SecurityContext,
    /// Requested capabilities
    pub requested_capabilities: HashSet<Capability>,
    /// User information
    pub user_info: Option<UserInfo>,
    /// System information
    pub system_info: SystemInfo,
    /// Evaluation timestamp
    pub timestamp: SystemTime,
    /// Additional context variables
    pub variables: HashMap<String, serde_json::Value>,
}

/// User information for policy evaluation.
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Group memberships.
    pub groups: Vec<String>,
    /// Assigned roles.
    pub roles: Vec<String>,
    /// Additional attributes.
    pub attributes: HashMap<String, String>,
}

/// System information for policy evaluation.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Hostname.
    pub hostname: String,
    /// OS type.
    pub os_type: String,
    /// OS version.
    pub os_version: String,
    /// CPU architecture.
    pub architecture: String,
    /// CPU core count.
    pub cpu_count: u32,
    /// Total memory in MB.
    pub memory_total_mb: u64,
    /// Load average.
    pub load_average: f64,
    /// System uptime in seconds.
    pub uptime_seconds: u64,
}

/// File-based policy configuration.
#[derive(Debug, Clone, Default)]
pub struct FilePolicyConfig {
    /// Directory containing policy files.
    pub policy_directory: String,
    /// Whether to cache loaded policies.
    pub cache_enabled: bool,
    /// Cache TTL in seconds.
    pub cache_ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
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
}
