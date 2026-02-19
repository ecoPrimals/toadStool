//! Policy condition evaluation
//!
//! This module handles the evaluation of policy conditions, including composite conditions,
//! time windows, resource checks, and custom expressions.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Timelike};
use regex::Regex;
use tokio::sync::RwLock;
use tracing::warn;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::workload::WorkloadSpec;

use crate::types::*;

/// Condition evaluator for policy rules
pub struct ConditionEvaluator {
    #[allow(dead_code)]
    regex_cache: Arc<RwLock<HashMap<String, Regex>>>,
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionEvaluator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            regex_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate condition structure
    #[allow(clippy::only_used_in_recursion)]
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
            PolicyCondition::TimeWindow {
                start_hour,
                end_hour,
                days,
            } => {
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
                    WorkloadSpec::Python { .. } => "python",
                    WorkloadSpec::AiMl { .. } => "aiml",
                    WorkloadSpec::Cuda { .. } => "cuda",
                };
                Ok(workload_types.contains(&workload_type.to_string()))
            }

            PolicyCondition::RequiresCapability { capabilities } => Ok(capabilities
                .iter()
                .any(|cap| context.requested_capabilities.contains(cap))),

            PolicyCondition::ResourceUsage {
                cpu_percent,
                memory_mb,
            } => {
                use sysinfo::System;
                let mut sys = System::new_all();
                sys.refresh_all();

                let cpus = sys.cpus();
                let current_cpu = if cpus.is_empty() {
                    0.0_f32
                } else {
                    cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
                };
                let current_mem_mb = (sys.used_memory() / (1024 * 1024)) as u32;

                let cpu_ok =
                    cpu_percent.is_none_or(|threshold| f64::from(current_cpu) <= threshold);
                let mem_ok =
                    memory_mb.is_none_or(|threshold_mb| u64::from(current_mem_mb) <= threshold_mb);

                Ok(cpu_ok && mem_ok)
            }

            PolicyCondition::TimeWindow {
                start_hour,
                end_hour,
                days,
            } => {
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
                    let group_match =
                        groups.is_empty() || groups.iter().any(|g| user_info.groups.contains(g));
                    Ok(user_match && group_match)
                } else {
                    Ok(users.is_empty() && groups.is_empty())
                }
            }

            PolicyCondition::Composite {
                operator,
                conditions,
            } => match operator {
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
                            "NOT operator requires exactly one condition".to_string(),
                        ));
                    }
                    let result = Box::pin(self.evaluate_condition(&conditions[0], context)).await?;
                    Ok(!result)
                }
            },

            _ => {
                warn!("Unimplemented condition evaluation: {:?}", condition);
                Ok(false)
            }
        }
    }
}
