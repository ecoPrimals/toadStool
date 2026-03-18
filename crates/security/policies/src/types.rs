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
    /// Workload type condition
    WorkloadType { workload_types: Vec<String> },
    /// Capability requirement condition
    RequiresCapability { capabilities: Vec<Capability> },
    /// Resource usage condition
    ResourceUsage {
        cpu_percent: Option<f64>,
        memory_mb: Option<u64>,
    },
    /// Time-based condition
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
        days: Vec<u8>,
    },
    /// User/group condition
    UserContext {
        users: Vec<String>,
        groups: Vec<String>,
    },
    /// Network access condition
    NetworkAccess { hosts: Vec<String>, ports: Vec<u16> },
    /// File system access condition
    FileSystemAccess {
        paths: Vec<PathBuf>,
        operations: Vec<String>,
    },
    /// Custom condition with expression
    Custom {
        expression: String,
        variables: HashMap<String, serde_json::Value>,
    },
    /// Composite condition (AND/OR/NOT)
    Composite {
        operator: LogicalOperator,
        conditions: Vec<Self>,
    },
}

/// Logical operators for composite conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

/// Policy action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the operation
    Allow,
    /// Deny the operation
    Deny,
    /// Allow with warnings
    AllowWithWarning { message: String },
    /// Deny with custom message
    DenyWithMessage { message: String },
    /// Modify security context
    ModifySecurityContext {
        isolation_level: Option<IsolationLevel>,
        add_capabilities: Vec<Capability>,
        remove_capabilities: Vec<Capability>,
    },
    /// Apply resource limits
    ApplyResourceLimits {
        cpu_percent: Option<f64>,
        memory_mb: Option<u64>,
        network_mbps: Option<f64>,
    },
    /// Require additional authentication
    RequireAuthentication { method: String },
    /// Log and continue
    LogAndContinue { level: String, message: String },
    /// Custom action with parameters
    Custom {
        action: String,
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

/// Policy evaluation result types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    Allow,
    Deny,
    Modified,
    RequiresAuth,
}

/// Applied rule information
#[derive(Debug, Clone)]
pub struct AppliedRule {
    pub rule_id: String,
    pub rule_name: String,
    pub action: PolicyAction,
    pub priority: u32,
    pub condition_matched: bool,
}

/// Security context modification
#[derive(Debug, Clone)]
pub struct SecurityModification {
    pub modification_type: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub reason: String,
}

/// Resource limit modification
#[derive(Debug, Clone)]
pub struct ResourceModification {
    pub resource_type: String,
    pub old_limit: Option<f64>,
    pub new_limit: f64,
    pub reason: String,
}

/// Policy warning
#[derive(Debug, Clone)]
pub struct PolicyWarning {
    pub level: String,
    pub message: String,
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

/// User information for policy evaluation
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub groups: Vec<String>,
    pub roles: Vec<String>,
    pub attributes: HashMap<String, String>,
}

/// System information for policy evaluation
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_type: String,
    pub os_version: String,
    pub architecture: String,
    pub cpu_count: u32,
    pub memory_total_mb: u64,
    pub load_average: f64,
    pub uptime_seconds: u64,
}

/// File policy configuration
#[derive(Debug, Clone, Default)]
pub struct FilePolicyConfig {
    pub policy_directory: String,
    pub cache_enabled: bool,
    pub cache_ttl_seconds: u64,
}
