// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy action execution
//!
//! This module handles the execution of policy actions, including resource limits,
//! security context modifications, and authentication requirements.

use tracing::debug;

use toadstool::error::ToadStoolResult;

use crate::types::{
    PolicyAction, PolicyEvaluationContext, PolicyEvaluationResult, PolicyResult, PolicyWarning,
    ResourceModification, SecurityModification,
};

/// Action executor for policy rules
pub struct ActionExecutor;

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionExecutor {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Execute policy action
    pub fn execute_action(
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
                remove_capabilities,
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
                        new_value: serde_json::to_value(capability)
                            .unwrap_or(serde_json::Value::Null),
                        reason: "Policy requirement".to_string(),
                    });
                }

                for capability in remove_capabilities {
                    result.security_modifications.push(SecurityModification {
                        modification_type: "remove_capability".to_string(),
                        old_value: serde_json::to_value(capability)
                            .unwrap_or(serde_json::Value::Null),
                        new_value: serde_json::Value::Null,
                        reason: "Policy restriction".to_string(),
                    });
                }
            }

            PolicyAction::ApplyResourceLimits {
                cpu_percent,
                memory_mb,
                network_mbps,
            } => {
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
                    message: format!("Additional authentication required: {method}"),
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
                debug!(
                    "Executing custom action: {} with parameters: {:?}",
                    action, parameters
                );
                result.warnings.push(PolicyWarning {
                    level: "info".to_string(),
                    message: format!("Custom action executed: {action}"),
                    rule_id: None,
                });
            }
        }

        Ok(())
    }
}
