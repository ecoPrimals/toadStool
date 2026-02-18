// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Migration planning logic — evaluation of when and where to migrate.

use crate::cloud_provider_trait::WorkloadLocation;
use crate::composition_constraints::{CompositionRequest, Constraint, ConstraintPriority};
use crate::workload_migration::{CostImpact, MigrationRecommendation, MigrationTarget};

use super::MigrationCoordinator;
use crate::ToadStoolResult;
use std::collections::HashMap;
use tracing::{debug, info};

impl MigrationCoordinator {
    /// Evaluate if workload should migrate
    pub async fn should_migrate(
        &self,
        workload_id: &str,
        constraints: &[Constraint],
    ) -> ToadStoolResult<MigrationRecommendation> {
        info!("🔍 Evaluating migration for workload: {}", workload_id);

        let current_location = self.get_workload_location(workload_id).await;

        let request = CompositionRequest {
            name: workload_id.to_string(),
            constraints: constraints.to_vec(),
            priority: ConstraintPriority::Normal,
            metadata: HashMap::new(),
        };

        let current_eval = self.engine.evaluate(&request).await?;

        debug!(
            "Current location feasibility: {}, score: {}",
            current_eval.is_feasible, current_eval.overall_score
        );

        if current_eval.is_feasible && current_eval.overall_score >= 0.9 {
            return Ok(MigrationRecommendation {
                should_migrate: false,
                reason: "Current location optimal".to_string(),
                target: None,
                cost_impact: None,
                confidence: 1.0,
            });
        }

        let recommendation = self
            .evaluate_migration_targets(workload_id, constraints, &current_location)
            .await?;

        Ok(recommendation)
    }

    /// Evaluate potential migration targets
    pub(super) async fn evaluate_migration_targets(
        &self,
        _workload_id: &str,
        constraints: &[Constraint],
        current_location: &Option<WorkloadLocation>,
    ) -> ToadStoolResult<MigrationRecommendation> {
        let providers = self.providers.read().await;
        let available = providers.available_providers();

        if available.is_empty() {
            return Ok(MigrationRecommendation {
                should_migrate: false,
                reason: "No cloud providers available".to_string(),
                target: None,
                cost_impact: None,
                confidence: 0.0,
            });
        }

        let has_cost_constraint = constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxCostPerHour(_) | Constraint::MinimizeCost));

        let requires_gpu = constraints
            .iter()
            .any(|c| matches!(c, Constraint::RequiresGPU));

        match current_location {
            None | Some(WorkloadLocation::Local { .. }) => {
                if requires_gpu && !self.runtime.has_direct_gpu_access() {
                    Ok(MigrationRecommendation {
                        should_migrate: true,
                        reason: "Local lacks GPU, cloud may have it".to_string(),
                        target: Some(MigrationTarget::Cloud {
                            provider: available[0].clone(),
                            region: "us-west-1".to_string(),
                            estimated_cost_per_hour: 5.0,
                        }),
                        cost_impact: Some(CostImpact {
                            current_cost_per_hour: 0.0,
                            new_cost_per_hour: 5.0,
                            savings_per_hour: -5.0,
                            migration_cost: 0.1,
                        }),
                        confidence: 0.8,
                    })
                } else if has_cost_constraint {
                    Ok(MigrationRecommendation {
                        should_migrate: false,
                        reason: "Cost-sensitive, local is cheapest".to_string(),
                        target: Some(MigrationTarget::Local),
                        cost_impact: None,
                        confidence: 0.9,
                    })
                } else {
                    Ok(MigrationRecommendation {
                        should_migrate: false,
                        reason: "Local is sufficient".to_string(),
                        target: None,
                        cost_impact: None,
                        confidence: 0.7,
                    })
                }
            }
            Some(WorkloadLocation::Cloud { provider, .. }) => {
                if has_cost_constraint {
                    Ok(MigrationRecommendation {
                        should_migrate: true,
                        reason: "Cloud costs high, local available".to_string(),
                        target: Some(MigrationTarget::Local),
                        cost_impact: Some(CostImpact {
                            current_cost_per_hour: 5.0,
                            new_cost_per_hour: 0.0,
                            savings_per_hour: 5.0,
                            migration_cost: 0.1,
                        }),
                        confidence: 0.8,
                    })
                } else {
                    Ok(MigrationRecommendation {
                        should_migrate: false,
                        reason: format!("Cloud ({}) is working well", provider),
                        target: None,
                        cost_impact: None,
                        confidence: 0.7,
                    })
                }
            }
        }
    }
}
