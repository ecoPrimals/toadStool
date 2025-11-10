//! Policy manager implementation
//!
//! This module provides the policy management trait and file-based implementation,
//! including policy loading, caching, validation, and composition.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use chrono::Utc;
use hex;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::evaluator::ConditionEvaluator;
use crate::executor::ActionExecutor;
use crate::types::*;

/// Cached policy with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CachedPolicy {
    policy: SecurityPolicy,
    cached_at: SystemTime,
    access_count: u64,
    last_accessed: SystemTime,
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

impl FilePolicyManager {
    /// Create new file-based policy manager
    pub fn new(config: PolicyManagerConfig) -> ToadStoolResult<Self> {
        info!("Creating file-based policy manager");

        // Ensure policy directory exists
        if !config.policy_dir.exists() {
            std::fs::create_dir_all(&config.policy_dir).map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to create policy directory {}: {}",
                    config.policy_dir.display(),
                    e
                ))
            })?;
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
        self.config.policy_dir.join(format!("{policy_id}.yaml"))
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

        let content = tokio::fs::read_to_string(&file_path).await.map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to read policy file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        let policy: SecurityPolicy = serde_yaml::from_str(&content).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to parse policy file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(policy)
    }

    /// Save policy to file
    async fn save_policy_to_file(&self, policy: &SecurityPolicy) -> ToadStoolResult<()> {
        let file_path = self.policy_file_path(&policy.id);

        let content = serde_yaml::to_string(policy).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to serialize policy {}: {}",
                policy.id, e
            ))
        })?;

        tokio::fs::write(&file_path, content).await.map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to write policy file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        info!("Saved policy {} to {}", policy.id, file_path.display());
        Ok(())
    }

    /// Merge evaluation results from parent policies
    fn merge_evaluation_results(
        &self,
        target: &mut PolicyEvaluationResult,
        source: PolicyEvaluationResult,
    ) {
        target.applied_rules.extend(source.applied_rules);
        target
            .security_modifications
            .extend(source.security_modifications);
        target
            .resource_modifications
            .extend(source.resource_modifications);
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
            cache.insert(
                policy_id.to_string(),
                CachedPolicy {
                    policy: policy.clone(),
                    cached_at: SystemTime::now(),
                    access_count: 1,
                    last_accessed: SystemTime::now(),
                },
            );
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
            cache.insert(
                policy.id.clone(),
                CachedPolicy {
                    policy: policy.clone(),
                    cached_at: SystemTime::now(),
                    access_count: 0,
                    last_accessed: SystemTime::now(),
                },
            );
        }

        Ok(())
    }

    async fn delete_policy(&self, policy_id: &str) -> ToadStoolResult<()> {
        debug!("Deleting policy: {}", policy_id);

        let file_path = self.policy_file_path(policy_id);

        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await.map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to delete policy file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
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
        let mut dir = tokio::fs::read_dir(&self.config.policy_dir)
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to read policy directory {}: {}",
                    self.config.policy_dir.display(),
                    e
                ))
            })?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            ToadStoolError::configuration(format!("Failed to read directory entry: {e}"))
        })? {
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
                errors.push(format!("Rule {i} has empty ID"));
            }

            if rule.name.is_empty() {
                errors.push(format!("Rule {i} has empty name"));
            }

            // Validate condition
            if let Err(e) = self.condition_evaluator.validate_condition(&rule.condition) {
                errors.push(format!("Rule {i} has invalid condition: {e}"));
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

            let condition_matched = self
                .condition_evaluator
                .evaluate_condition(&rule.condition, context)
                .await?;

            if condition_matched {
                let applied_rule = AppliedRule {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    action: rule.action.clone(),
                    priority: rule.priority,
                    condition_matched: true,
                };

                // Execute action
                self.action_executor
                    .execute_action(&rule.action, &mut result, context)
                    .await?;
                result.applied_rules.push(applied_rule);
            }
        }

        result.evaluation_duration = start_time.elapsed().unwrap_or(Duration::ZERO);
        debug!(
            "Policy evaluation completed in {:?}",
            result.evaluation_duration
        );

        Ok(result)
    }

    async fn compose_policies(&self, policy_ids: &[String]) -> ToadStoolResult<SecurityPolicy> {
        debug!("Composing {} policies", policy_ids.len());

        if policy_ids.is_empty() {
            return Err(ToadStoolError::validation(
                "No policies provided for composition".to_string(),
            ));
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
                composed_policy
                    .metadata
                    .insert(format!("{}_{}", policy.id, key), value.clone());
            }
        }

        debug!(
            "Composed policy created with {} rules",
            composed_policy.rules.len()
        );
        Ok(composed_policy)
    }

    async fn get_policy_dependencies(&self, policy_id: &str) -> ToadStoolResult<Vec<String>> {
        debug!("Getting dependencies for policy: {}", policy_id);

        let mut dependencies = Vec::new();

        // Simple dependency collection without recursion
        if let Ok(policy) = self.load_policy(policy_id).await {
            dependencies.extend(policy.inherits);
        }

        debug!(
            "Found {} dependencies for policy {}",
            dependencies.len(),
            policy_id
        );
        Ok(dependencies)
    }
}
