// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy manager implementation
//!
//! This module provides the policy management trait and file-based implementation,
//! including policy loading, caching, validation, and composition.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use std::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::cache::{CachedPolicy, is_cache_valid};
use crate::composition::{build_composed_policy, merge_evaluation_results};
use crate::evaluator::ConditionEvaluator;
use crate::executor::ActionExecutor;
use crate::types::{
    AppliedRule, PolicyEvaluationContext, PolicyEvaluationResult, PolicyManagerConfig,
    PolicyResult, SecurityPolicy,
};

/// Policy manager trait
#[expect(
    async_fn_in_trait,
    reason = "all implementors are Send + Sync; trait is internal, no dyn dispatch"
)]
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
    /// Get policy file path - prefers TOML (ecoBin compliant), falls back to YAML
    fn policy_file_path(&self, policy_id: &str) -> PathBuf {
        // Check for TOML first (ecoBin preferred)
        let toml_path = self.config.policy_dir.join(format!("{policy_id}.toml"));
        if toml_path.exists() {
            return toml_path;
        }
        // Fall back to YAML for backwards compatibility
        let yaml_path = self.config.policy_dir.join(format!("{policy_id}.yaml"));
        if yaml_path.exists() {
            return yaml_path;
        }
        // New files use TOML
        toml_path
    }

    /// Load policy from file
    fn load_policy_from_file(&self, policy_id: &str) -> ToadStoolResult<SecurityPolicy> {
        let file_path = self.policy_file_path(policy_id);

        if !file_path.exists() {
            return Err(ToadStoolError::configuration(format!(
                "Policy file not found: {}",
                file_path.display()
            )));
        }

        let content = std::fs::read_to_string(&file_path).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to read policy file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        // Parse based on file extension (TOML preferred, YAML for backwards compatibility)
        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let policy: SecurityPolicy = match extension {
            "toml" => toml::from_str(&content).map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to parse TOML policy {}: {}",
                    file_path.display(),
                    e
                ))
            })?,
            _ => serde_yaml_ng::from_str(&content).map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to parse YAML policy {}: {}",
                    file_path.display(),
                    e
                ))
            })?,
        };

        Ok(policy)
    }

    /// Save policy to file (TOML format - ecoBin compliant)
    fn save_policy_to_file(&self, policy: &SecurityPolicy) -> ToadStoolResult<()> {
        // Always save as TOML (ecoBin compliant, pure Rust)
        let file_path = self.config.policy_dir.join(format!("{}.toml", policy.id));

        let content = toml::to_string_pretty(policy).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to serialize policy {}: {}",
                policy.id, e
            ))
        })?;

        std::fs::write(&file_path, content).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to write policy file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        info!("Saved policy {} to {}", policy.id, file_path.display());
        Ok(())
    }
}

impl PolicyManager for FilePolicyManager {
    async fn load_policy(&self, policy_id: &str) -> ToadStoolResult<SecurityPolicy> {
        debug!("Loading policy: {}", policy_id);

        // Check cache first; update LRU metadata on hit.
        {
            let mut cache = self
                .policy_cache
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(cached) = cache.get_mut(policy_id)
                && is_cache_valid(cached, &self.config)
            {
                debug!(
                    "Policy {} found in cache (hits: {})",
                    policy_id,
                    cached.access_count + 1
                );
                cached.touch();
                return Ok(cached.policy.clone());
            }
        }

        // Load from file
        let policy = self.load_policy_from_file(policy_id)?;

        // Update cache
        if self.config.cache_enabled {
            let mut cache = self
                .policy_cache
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        self.save_policy_to_file(policy)?;

        // Update cache
        if self.config.cache_enabled {
            let mut cache = self
                .policy_cache
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            std::fs::remove_file(&file_path).map_err(|e| {
                ToadStoolError::configuration(format!(
                    "Failed to delete policy file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
        }

        // Remove from cache
        self.policy_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(policy_id);

        info!("Deleted policy: {}", policy_id);
        Ok(())
    }

    async fn list_policies(&self) -> ToadStoolResult<Vec<String>> {
        debug!("Listing policies in {}", self.config.policy_dir.display());

        let mut policies = Vec::new();
        let dir = std::fs::read_dir(&self.config.policy_dir).map_err(|e| {
            ToadStoolError::configuration(format!(
                "Failed to read policy directory {}: {}",
                self.config.policy_dir.display(),
                e
            ))
        })?;

        for entry in dir {
            let entry = entry.map_err(|e| {
                ToadStoolError::configuration(format!("Failed to read directory entry: {e}"))
            })?;
            let path = entry.path();
            // Policies are persisted as TOML (see save_policy_to_file).
            if path.extension().and_then(|s| s.to_str()) == Some("toml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                policies.push(stem.to_string());
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
            let parent_result = Box::pin(self.evaluate_policy(parent_id, context)).await?;
            merge_evaluation_results(&mut result, parent_result);
        }

        // Evaluate current policy rules
        let mut rules_by_priority: Vec<_> = policy.rules.iter().collect();
        rules_by_priority.sort_by_key(|r| std::cmp::Reverse(r.priority));

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
                    .execute_action(&rule.action, &mut result, context)?;
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

        let composed_policy = build_composed_policy(policy_ids, &policies);
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
