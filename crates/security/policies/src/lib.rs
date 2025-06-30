//! Advanced Security Policy Management for ToadStool
//!
//! This crate provides comprehensive security policy management, including:
//! - Policy composition and validation
//! - Dynamic policy resolution
//! - Cross-platform security enforcement
//! - Security event monitoring and alerting

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::{DateTime, Utc, Timelike, Datelike};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
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
        Self {
            policy_dir: PathBuf::from("/etc/toadstool/policies"),
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
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
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
    ResourceUsage { cpu_percent: Option<f64>, memory_mb: Option<u64> },
    /// Time-based condition
    TimeWindow { start_hour: u8, end_hour: u8, days: Vec<u8> },
    /// User/group condition
    UserContext { users: Vec<String>, groups: Vec<String> },
    /// Network access condition
    NetworkAccess { hosts: Vec<String>, ports: Vec<u16> },
    /// File system access condition
    FileSystemAccess { paths: Vec<PathBuf>, operations: Vec<String> },
    /// Custom condition with expression
    Custom { expression: String, variables: HashMap<String, serde_json::Value> },
    /// Composite condition (AND/OR/NOT)
    Composite { operator: LogicalOperator, conditions: Vec<PolicyCondition> },
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
    Custom { action: String, parameters: HashMap<String, serde_json::Value> },
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
#[derive(Debug, Clone, PartialEq)]
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

/// Policy manager trait
#[async_trait]
pub trait PolicyManager: Send + Sync {
    /// Load policy from storage
    async fn load_policy(&self, policy_id: &str) -> ToadStoolResult<SecurityPolicy>;
    
    /// Save policy to storage
    async fn save_policy(&self, policy: &SecurityPolicy) -> ToadStoolResult<()>;
    
    /// Delete policy from storage
    async fn delete_policy(&self, policy_id: &str) -> ToadStoolResult<()>;
    
    /// List all available policies
    async fn list_policies(&self) -> ToadStoolResult<Vec<String>>;
    
    /// Validate policy structure and rules
    async fn validate_policy(&self, policy: &SecurityPolicy) -> ToadStoolResult<Vec<String>>;
    
    /// Evaluate policy against context
    async fn evaluate_policy(
        &self,
        policy_id: &str,
        context: &PolicyEvaluationContext,
    ) -> ToadStoolResult<PolicyEvaluationResult>;
    
    /// Compose multiple policies into a single effective policy
    async fn compose_policies(&self, policy_ids: &[String]) -> ToadStoolResult<SecurityPolicy>;
    
    /// Get policy dependencies (inheritance chain)
    async fn get_policy_dependencies(&self, policy_id: &str) -> ToadStoolResult<Vec<String>>;
}

/// File-based policy manager implementation
pub struct FilePolicyManager {
    config: PolicyManagerConfig,
    policy_cache: Arc<RwLock<HashMap<String, CachedPolicy>>>,
    condition_evaluator: ConditionEvaluator,
    action_executor: ActionExecutor,
}

/// Cached policy with metadata
#[derive(Debug, Clone)]
struct CachedPolicy {
    policy: SecurityPolicy,
    cached_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
}

impl FilePolicyManager {
    /// Create new file-based policy manager
    pub fn new(config: PolicyManagerConfig) -> ToadStoolResult<Self> {
        info!("Creating file-based policy manager");
        
        // Ensure policy directory exists
        if !config.policy_dir.exists() {
            std::fs::create_dir_all(&config.policy_dir)
                .map_err(|e| ToadStoolError::configuration(format!(
                    "Failed to create policy directory {}: {}",
                    config.policy_dir.display(),
                    e
                )))?;
        }
        
        Ok(Self {
            config,
            policy_cache: Arc::new(RwLock::new(HashMap::new())),
            condition_evaluator: ConditionEvaluator::new(),
            action_executor: ActionExecutor::new(),
        })
    }
    
    /// Generate policy file path
    fn policy_file_path(&self, policy_id: &str) -> PathBuf {
        self.config.policy_dir.join(format!("{}.yaml", policy_id))
    }
    
    /// Check if cached policy is still valid
    fn is_cache_valid(&self, cached_policy: &CachedPolicy) -> bool {
        if !self.config.cache_enabled {
            return false;
        }
        
        let cache_duration = Duration::from_secs(self.config.cache_ttl_hours * 3600);
        cached_policy.cached_at.elapsed().unwrap_or(Duration::MAX) < cache_duration
    }
    
    /// Load policy from file
    async fn load_policy_from_file(&self, policy_id: &str) -> ToadStoolResult<SecurityPolicy> {
        let file_path = self.policy_file_path(policy_id);
        
        if !file_path.exists() {
            return Err(ToadStoolError::configuration(format!(
                "Policy file not found: {}",
                file_path.display()
            )));
        }
        
        let content = tokio::fs::read_to_string(&file_path).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to read policy file {}: {}",
                file_path.display(),
                e
            )))?;
        
        let policy: SecurityPolicy = serde_yaml::from_str(&content)
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to parse policy file {}: {}",
                file_path.display(),
                e
            )))?;
        
        Ok(policy)
    }
    
    /// Save policy to file
    async fn save_policy_to_file(&self, policy: &SecurityPolicy) -> ToadStoolResult<()> {
        let file_path = self.policy_file_path(&policy.id);
        
        let content = serde_yaml::to_string(policy)
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to serialize policy {}: {}",
                policy.id,
                e
            )))?;
        
        tokio::fs::write(&file_path, content).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to write policy file {}: {}",
                file_path.display(),
                e
            )))?;
        
        info!("Saved policy {} to {}", policy.id, file_path.display());
        Ok(())
    }
}

#[async_trait]
impl PolicyManager for FilePolicyManager {
    async fn load_policy(&self, policy_id: &str) -> ToadStoolResult<SecurityPolicy> {
        debug!("Loading policy: {}", policy_id);
        
        // Check cache first
        {
            let cache = self.policy_cache.read().await;
            if let Some(cached) = cache.get(policy_id) {
                if self.is_cache_valid(cached) {
                    debug!("Policy {} found in cache", policy_id);
                    return Ok(cached.policy.clone());
                }
            }
        }
        
        // Load from file
        let policy = self.load_policy_from_file(policy_id).await?;
        
        // Update cache
        if self.config.cache_enabled {
            let mut cache = self.policy_cache.write().await;
            cache.insert(policy_id.to_string(), CachedPolicy {
                policy: policy.clone(),
                cached_at: SystemTime::now(),
                access_count: 1,
                last_accessed: SystemTime::now(),
            });
        }
        
        Ok(policy)
    }
    
    async fn save_policy(&self, policy: &SecurityPolicy) -> ToadStoolResult<()> {
        debug!("Saving policy: {}", policy.id);
        
        // Validate policy first
        let validation_errors = self.validate_policy(policy).await?;
        if !validation_errors.is_empty() && self.config.strict_enforcement {
            return Err(ToadStoolError::validation(format!(
                "Policy validation failed: {}",
                validation_errors.join(", ")
            )));
        }
        
        // Save to file
        self.save_policy_to_file(policy).await?;
        
        // Update cache
        if self.config.cache_enabled {
            let mut cache = self.policy_cache.write().await;
            cache.insert(policy.id.clone(), CachedPolicy {
                policy: policy.clone(),
                cached_at: SystemTime::now(),
                access_count: 0,
                last_accessed: SystemTime::now(),
            });
        }
        
        Ok(())
    }
    
    async fn delete_policy(&self, policy_id: &str) -> ToadStoolResult<()> {
        debug!("Deleting policy: {}", policy_id);
        
        let file_path = self.policy_file_path(policy_id);
        
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await
                .map_err(|e| ToadStoolError::configuration(format!(
                    "Failed to delete policy file {}: {}",
                    file_path.display(),
                    e
                )))?;
        }
        
        // Remove from cache
        let mut cache = self.policy_cache.write().await;
        cache.remove(policy_id);
        
        info!("Deleted policy: {}", policy_id);
        Ok(())
    }
    
    async fn list_policies(&self) -> ToadStoolResult<Vec<String>> {
        debug!("Listing policies in {}", self.config.policy_dir.display());
        
        let mut policies = Vec::new();
        let mut dir = tokio::fs::read_dir(&self.config.policy_dir).await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to read policy directory {}: {}",
                self.config.policy_dir.display(),
                e
            )))?;
        
        while let Some(entry) = dir.next_entry().await
            .map_err(|e| ToadStoolError::configuration(format!(
                "Failed to read directory entry: {}",
                e
            )))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    policies.push(stem.to_string());
                }
            }
        }
        
        policies.sort();
        debug!("Found {} policies", policies.len());
        Ok(policies)
    }
    
    async fn validate_policy(&self, policy: &SecurityPolicy) -> ToadStoolResult<Vec<String>> {
        debug!("Validating policy: {}", policy.id);
        let mut errors = Vec::new();
        
        // Basic validation
        if policy.id.is_empty() {
            errors.push("Policy ID cannot be empty".to_string());
        }
        
        if policy.name.is_empty() {
            errors.push("Policy name cannot be empty".to_string());
        }
        
        if policy.version.is_empty() {
            errors.push("Policy version cannot be empty".to_string());
        }
        
        // Validate rules
        for (i, rule) in policy.rules.iter().enumerate() {
            if rule.id.is_empty() {
                errors.push(format!("Rule {} has empty ID", i));
            }
            
            if rule.name.is_empty() {
                errors.push(format!("Rule {} has empty name", i));
            }
            
            // Validate condition
            if let Err(e) = self.condition_evaluator.validate_condition(&rule.condition) {
                errors.push(format!("Rule {} has invalid condition: {}", i, e));
            }
        }
        
        // Check for circular dependencies in inheritance - simplified check
        if policy.inherits.contains(&policy.id) {
            errors.push("Policy cannot inherit from itself".to_string());
        }
        
        debug!("Policy validation completed with {} errors", errors.len());
        Ok(errors)
    }
    
    async fn evaluate_policy(
        &self,
        policy_id: &str,
        context: &PolicyEvaluationContext,
    ) -> ToadStoolResult<PolicyEvaluationResult> {
        let start_time = SystemTime::now();
        debug!("Evaluating policy {} against context", policy_id);
        
        let policy = self.load_policy(policy_id).await?;
        let mut result = PolicyEvaluationResult {
            evaluation_id: Uuid::new_v4(),
            policy_id: policy_id.to_string(),
            result: PolicyResult::Allow,
            applied_rules: Vec::new(),
            security_modifications: Vec::new(),
            resource_modifications: Vec::new(),
            warnings: Vec::new(),
            evaluation_duration: Duration::default(),
            timestamp: start_time,
        };
        
        // Evaluate inherited policies first
        for parent_id in &policy.inherits {
            let parent_result = self.evaluate_policy(parent_id, context).await?;
            self.merge_evaluation_results(&mut result, parent_result);
        }
        
        // Evaluate current policy rules
        let mut rules_by_priority: Vec<_> = policy.rules.iter().collect();
        rules_by_priority.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        for rule in rules_by_priority {
            if !rule.enabled {
                continue;
            }
            
            let condition_matched = self.condition_evaluator
                .evaluate_condition(&rule.condition, context).await?;
            
            if condition_matched {
                let applied_rule = AppliedRule {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    action: rule.action.clone(),
                    priority: rule.priority,
                    condition_matched: true,
                };
                
                // Execute action
                self.action_executor.execute_action(&rule.action, &mut result, context).await?;
                result.applied_rules.push(applied_rule);
            }
        }
        
        result.evaluation_duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        debug!("Policy evaluation completed in {:?}", result.evaluation_duration);
        
        Ok(result)
    }
    
    async fn compose_policies(&self, policy_ids: &[String]) -> ToadStoolResult<SecurityPolicy> {
        debug!("Composing {} policies", policy_ids.len());
        
        if policy_ids.is_empty() {
            return Err(ToadStoolError::validation("No policies provided for composition".to_string()));
        }
        
        // Load all policies
        let mut policies = Vec::new();
        for policy_id in policy_ids {
            let policy = self.load_policy(policy_id).await?;
            policies.push(policy);
        }
        
        // Create composed policy
        let composed_id = self.generate_composed_policy_id(policy_ids);
        let mut composed_policy = SecurityPolicy {
            id: composed_id,
            name: format!("Composed Policy: {}", policy_ids.join(", ")),
            version: "1.0.0".to_string(),
            description: Some("Automatically composed policy".to_string()),
            author: Some("ToadStool Policy Manager".to_string()),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            rules: Vec::new(),
            inherits: Vec::new(),
            metadata: HashMap::new(),
            signature: None,
        };
        
        // Merge rules from all policies (sorted by priority)
        let mut all_rules = Vec::new();
        for policy in &policies {
            for rule in &policy.rules {
                all_rules.push(rule.clone());
            }
        }
        
        all_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        composed_policy.rules = all_rules;
        
        // Merge metadata
        for policy in &policies {
            for (key, value) in &policy.metadata {
                composed_policy.metadata.insert(
                    format!("{}_{}", policy.id, key),
                    value.clone(),
                );
            }
        }
        
        debug!("Composed policy created with {} rules", composed_policy.rules.len());
        Ok(composed_policy)
    }
    
    async fn get_policy_dependencies(&self, policy_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Getting dependencies for policy: {}", policy_id);
        
        let mut dependencies = Vec::new();
        
        // Simple dependency collection without recursion
        if let Ok(policy) = self.load_policy(policy_id).await {
            dependencies.extend(policy.inherits);
        }
        
        debug!("Found {} dependencies for policy {}", dependencies.len(), policy_id);
        Ok(dependencies)
    }
}

impl FilePolicyManager {
    
    /// Merge evaluation results from parent policies
    fn merge_evaluation_results(
        &self,
        target: &mut PolicyEvaluationResult,
        source: PolicyEvaluationResult,
    ) {
        target.applied_rules.extend(source.applied_rules);
        target.security_modifications.extend(source.security_modifications);
        target.resource_modifications.extend(source.resource_modifications);
        target.warnings.extend(source.warnings);
        
        // Update result based on priority
        match (&target.result, &source.result) {
            (_, PolicyResult::Deny) => target.result = PolicyResult::Deny,
            (PolicyResult::Allow, other) => target.result = other.clone(),
            _ => {} // Keep existing result
        }
    }
    
    /// Generate composed policy ID
    fn generate_composed_policy_id(&self, policy_ids: &[String]) -> String {
        let mut hasher = Sha256::new();
        for id in policy_ids {
            hasher.update(id.as_bytes());
        }
        format!("composed_{}", &hex::encode(hasher.finalize())[..16])
    }
    

}

/// Condition evaluator for policy rules
pub struct ConditionEvaluator {
    regex_cache: Arc<RwLock<HashMap<String, Regex>>>,
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator {
    pub fn new() -> Self {
        Self {
            regex_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Validate condition structure
    pub fn validate_condition(&self, condition: &PolicyCondition) -> Result<(), String> {
        match condition {
            PolicyCondition::Always | PolicyCondition::Never => Ok(()),
            PolicyCondition::WorkloadType { workload_types } => {
                if workload_types.is_empty() {
                    Err("Workload types cannot be empty".to_string())
                } else {
                    Ok(())
                }
            }
            PolicyCondition::RequiresCapability { capabilities } => {
                if capabilities.is_empty() {
                    Err("Capabilities cannot be empty".to_string())
                } else {
                    Ok(())
                }
            }
            PolicyCondition::TimeWindow { start_hour, end_hour, days } => {
                if *start_hour > 23 || *end_hour > 23 {
                    Err("Hours must be 0-23".to_string())
                } else if days.iter().any(|&d| d > 6) {
                    Err("Days must be 0-6".to_string())
                } else {
                    Ok(())
                }
            }
            PolicyCondition::Custom { expression, .. } => {
                if expression.is_empty() {
                    Err("Custom expression cannot be empty".to_string())
                } else {
                    Ok(())
                }
            }
            PolicyCondition::Composite { conditions, .. } => {
                for condition in conditions {
                    self.validate_condition(condition)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    /// Evaluate condition against context
    pub async fn evaluate_condition(
        &self,
        condition: &PolicyCondition,
        context: &PolicyEvaluationContext,
    ) -> ToadStoolResult<bool> {
        match condition {
            PolicyCondition::Always => Ok(true),
            PolicyCondition::Never => Ok(false),
            
            PolicyCondition::WorkloadType { workload_types } => {
                let workload_type = match &context.workload {
                    WorkloadSpec::Native { .. } => "native",
                    WorkloadSpec::Wasm { .. } => "wasm",
                    WorkloadSpec::Container { .. } => "container",
                    WorkloadSpec::Gpu { .. } => "gpu",
                    WorkloadSpec::Script { .. } => "script",
                };
                Ok(workload_types.contains(&workload_type.to_string()))
            }
            
            PolicyCondition::RequiresCapability { capabilities } => {
                Ok(capabilities.iter().any(|cap| context.requested_capabilities.contains(cap)))
            }
            
            PolicyCondition::ResourceUsage { cpu_percent: _, memory_mb: _ } => {
                // This would typically check current resource usage
                // For now, return true as a placeholder
                Ok(true)
            }
            
            PolicyCondition::TimeWindow { start_hour, end_hour, days } => {
                let now = chrono::Utc::now();
                let hour = now.hour() as u8;
                let day = now.weekday().num_days_from_sunday() as u8;
                
                let hour_match = if start_hour <= end_hour {
                    hour >= *start_hour && hour <= *end_hour
                } else {
                    hour >= *start_hour || hour <= *end_hour
                };
                
                let day_match = days.is_empty() || days.contains(&day);
                Ok(hour_match && day_match)
            }
            
            PolicyCondition::UserContext { users, groups } => {
                if let Some(user_info) = &context.user_info {
                    let user_match = users.is_empty() || users.contains(&user_info.username);
                    let group_match = groups.is_empty() || 
                        groups.iter().any(|g| user_info.groups.contains(g));
                    Ok(user_match && group_match)
                } else {
                    Ok(users.is_empty() && groups.is_empty())
                }
            }
            
            PolicyCondition::Composite { operator, conditions } => {
                match operator {
                    LogicalOperator::And => {
                        for condition in conditions {
                            if !Box::pin(self.evaluate_condition(condition, context)).await? {
                                return Ok(false);
                            }
                        }
                        Ok(true)
                    }
                    LogicalOperator::Or => {
                        for condition in conditions {
                            if Box::pin(self.evaluate_condition(condition, context)).await? {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    }
                    LogicalOperator::Not => {
                        if conditions.len() != 1 {
                            return Err(ToadStoolError::validation(
                                "NOT operator requires exactly one condition".to_string()
                            ));
                        }
                        let result = Box::pin(self.evaluate_condition(&conditions[0], context)).await?;
                        Ok(!result)
                    }
                }
            }
            
            _ => {
                warn!("Unimplemented condition evaluation: {:?}", condition);
                Ok(false)
            }
        }
    }
}

/// Action executor for policy rules
pub struct ActionExecutor;

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self
    }
    
    /// Execute policy action
    pub async fn execute_action(
        &self,
        action: &PolicyAction,
        result: &mut PolicyEvaluationResult,
        _context: &PolicyEvaluationContext,
    ) -> ToadStoolResult<()> {
        match action {
            PolicyAction::Allow => {
                result.result = PolicyResult::Allow;
            }
            
            PolicyAction::Deny => {
                result.result = PolicyResult::Deny;
            }
            
            PolicyAction::AllowWithWarning { message } => {
                result.result = PolicyResult::Allow;
                result.warnings.push(PolicyWarning {
                    level: "warning".to_string(),
                    message: message.clone(),
                    rule_id: None,
                });
            }
            
            PolicyAction::DenyWithMessage { message } => {
                result.result = PolicyResult::Deny;
                result.warnings.push(PolicyWarning {
                    level: "error".to_string(),
                    message: message.clone(),
                    rule_id: None,
                });
            }
            
            PolicyAction::ModifySecurityContext { 
                isolation_level, 
                add_capabilities, 
                remove_capabilities 
            } => {
                result.result = PolicyResult::Modified;
                
                if let Some(level) = isolation_level {
                    result.security_modifications.push(SecurityModification {
                        modification_type: "isolation_level".to_string(),
                        old_value: serde_json::Value::Null,
                        new_value: serde_json::to_value(level).unwrap_or(serde_json::Value::Null),
                        reason: "Policy enforcement".to_string(),
                    });
                }
                
                for capability in add_capabilities {
                    result.security_modifications.push(SecurityModification {
                        modification_type: "add_capability".to_string(),
                        old_value: serde_json::Value::Null,
                        new_value: serde_json::to_value(capability).unwrap_or(serde_json::Value::Null),
                        reason: "Policy requirement".to_string(),
                    });
                }
                
                for capability in remove_capabilities {
                    result.security_modifications.push(SecurityModification {
                        modification_type: "remove_capability".to_string(),
                        old_value: serde_json::to_value(capability).unwrap_or(serde_json::Value::Null),
                        new_value: serde_json::Value::Null,
                        reason: "Policy restriction".to_string(),
                    });
                }
            }
            
            PolicyAction::ApplyResourceLimits { cpu_percent, memory_mb, network_mbps } => {
                result.result = PolicyResult::Modified;
                
                if let Some(cpu) = cpu_percent {
                    result.resource_modifications.push(ResourceModification {
                        resource_type: "cpu_percent".to_string(),
                        old_limit: None,
                        new_limit: *cpu,
                        reason: "Policy enforcement".to_string(),
                    });
                }
                
                if let Some(memory) = memory_mb {
                    result.resource_modifications.push(ResourceModification {
                        resource_type: "memory_mb".to_string(),
                        old_limit: None,
                        new_limit: *memory as f64,
                        reason: "Policy enforcement".to_string(),
                    });
                }
                
                if let Some(network) = network_mbps {
                    result.resource_modifications.push(ResourceModification {
                        resource_type: "network_mbps".to_string(),
                        old_limit: None,
                        new_limit: *network,
                        reason: "Policy enforcement".to_string(),
                    });
                }
            }
            
            PolicyAction::RequireAuthentication { method } => {
                result.result = PolicyResult::RequiresAuth;
                result.warnings.push(PolicyWarning {
                    level: "info".to_string(),
                    message: format!("Additional authentication required: {}", method),
                    rule_id: None,
                });
            }
            
            PolicyAction::LogAndContinue { level, message } => {
                result.warnings.push(PolicyWarning {
                    level: level.clone(),
                    message: message.clone(),
                    rule_id: None,
                });
            }
            
            PolicyAction::Custom { action, parameters } => {
                debug!("Executing custom action: {} with parameters: {:?}", action, parameters);
                result.warnings.push(PolicyWarning {
                    level: "info".to_string(),
                    message: format!("Custom action executed: {}", action),
                    rule_id: None,
                });
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_policy() -> SecurityPolicy {
        SecurityPolicy {
            id: "test-policy".to_string(),
            name: "Test Policy".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test policy for unit tests".to_string()),
            author: Some("Test Author".to_string()),
            created_at: Utc::now(),
            modified_at: Utc::now(),
            rules: vec![
                PolicyRule {
                    id: "rule-1".to_string(),
                    name: "Allow Native Workloads".to_string(),
                    condition: PolicyCondition::WorkloadType {
                        workload_types: vec!["native".to_string()],
                    },
                    action: PolicyAction::Allow,
                    priority: 100,
                    enabled: true,
                    description: Some("Allow native workloads".to_string()),
                },
            ],
            inherits: Vec::new(),
            metadata: HashMap::new(),
            signature: None,
        }
    }
    
    fn create_test_context() -> PolicyEvaluationContext {
        PolicyEvaluationContext {
            workload: WorkloadSpec::Native {
                executable: toadstool::workload::ExecutableSource::File {
                    path: PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            security_context: SecurityContext::default(),
            requested_capabilities: HashSet::new(),
            user_info: None,
            system_info: SystemInfo {
                hostname: "test-host".to_string(),
                os_type: "Linux".to_string(),
                os_version: "5.4.0".to_string(),
                architecture: "x86_64".to_string(),
                cpu_count: 4,
                memory_total_mb: 8192,
                load_average: 0.5,
                uptime_seconds: 3600,
            },
            timestamp: SystemTime::now(),
            variables: HashMap::new(),
        }
    }
    
    #[tokio::test]
    async fn test_policy_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = PolicyManagerConfig {
            policy_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = FilePolicyManager::new(config).unwrap();
        assert!(temp_dir.path().exists());
    }
    
    #[tokio::test]
    async fn test_policy_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = PolicyManagerConfig {
            policy_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = FilePolicyManager::new(config).unwrap();
        let policy = create_test_policy();
        
        // Save policy
        manager.save_policy(&policy).await.unwrap();
        
        // Load policy
        let loaded_policy = manager.load_policy(&policy.id).await.unwrap();
        assert_eq!(loaded_policy.id, policy.id);
        assert_eq!(loaded_policy.name, policy.name);
        assert_eq!(loaded_policy.rules.len(), policy.rules.len());
    }
    
    #[tokio::test]
    async fn test_policy_evaluation() {
        let temp_dir = TempDir::new().unwrap();
        let config = PolicyManagerConfig {
            policy_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let manager = FilePolicyManager::new(config).unwrap();
        let policy = create_test_policy();
        let context = create_test_context();
        
        // Save policy
        manager.save_policy(&policy).await.unwrap();
        
        // Evaluate policy
        let result = manager.evaluate_policy(&policy.id, &context).await.unwrap();
        assert_eq!(result.result, PolicyResult::Allow);
        assert_eq!(result.applied_rules.len(), 1);
    }
    
    #[tokio::test]
    async fn test_condition_evaluation() {
        let evaluator = ConditionEvaluator::new();
        let context = create_test_context();
        
        // Test workload type condition
        let condition = PolicyCondition::WorkloadType {
            workload_types: vec!["native".to_string()],
        };
        let result = evaluator.evaluate_condition(&condition, &context).await.unwrap();
        assert!(result);
        
        // Test always condition
        let condition = PolicyCondition::Always;
        let result = evaluator.evaluate_condition(&condition, &context).await.unwrap();
        assert!(result);
        
        // Test never condition
        let condition = PolicyCondition::Never;
        let result = evaluator.evaluate_condition(&condition, &context).await.unwrap();
        assert!(!result);
    }
}
