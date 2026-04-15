// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy composition and evaluation merge helpers.

use std::collections::HashMap;
use std::time::SystemTime;

use sha2::{Digest, Sha256};

use crate::types::{PolicyEvaluationResult, PolicyResult, SecurityPolicy};

/// Merge evaluation results from parent policies
pub(crate) fn merge_evaluation_results(
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
pub(crate) fn generate_composed_policy_id(policy_ids: &[String]) -> String {
    let mut hasher = Sha256::new();
    for id in policy_ids {
        hasher.update(id.as_bytes());
    }
    format!("composed_{}", &hex::encode(hasher.finalize())[..16])
}

/// Build a merged [`SecurityPolicy`] from loaded policies (rules + metadata).
pub(crate) fn build_composed_policy(
    policy_ids: &[String],
    policies: &[SecurityPolicy],
) -> SecurityPolicy {
    let composed_id = generate_composed_policy_id(policy_ids);
    let mut composed_policy = SecurityPolicy {
        id: composed_id,
        name: format!("Composed Policy: {}", policy_ids.join(", ")),
        version: "1.0.0".to_string(),
        description: Some("Automatically composed policy".to_string()),
        author: Some("ToadStool Policy Manager".to_string()),
        created_at: SystemTime::now(),
        modified_at: SystemTime::now(),
        rules: Vec::new(),
        inherits: Vec::new(),
        metadata: HashMap::new(),
        signature: None,
    };

    // Merge rules from all policies (sorted by priority)
    let mut all_rules = Vec::new();
    for policy in policies {
        for rule in &policy.rules {
            all_rules.push(rule.clone());
        }
    }

    all_rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    composed_policy.rules = all_rules;

    // Merge metadata
    for policy in policies {
        for (key, value) in &policy.metadata {
            composed_policy
                .metadata
                .insert(format!("{}_{}", policy.id, key), value.clone());
        }
    }

    composed_policy
}
