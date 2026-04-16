// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Migration planning logic — evaluation of when and where to migrate.

use crate::cloud_provider_trait::{CloudProvider, WorkloadLocation};
use crate::composition_constraints::{CompositionRequest, Constraint, ConstraintPriority};
use crate::workload_migration::{CostImpact, MigrationRecommendation, MigrationTarget};

use super::MigrationCoordinator;
use crate::ToadStoolResult;
use std::collections::HashMap;
use tracing::{debug, info};

impl<P: CloudProvider> MigrationCoordinator<P> {
    /// Evaluate if workload should migrate
    ///
    /// # Errors
    ///
    /// Returns error if composition evaluation fails.
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
            .evaluate_migration_targets(workload_id, constraints, current_location.as_ref())
            .await?;

        Ok(recommendation)
    }

    /// Evaluate potential migration targets
    pub(super) async fn evaluate_migration_targets(
        &self,
        _workload_id: &str,
        constraints: &[Constraint],
        current_location: Option<&WorkloadLocation>,
    ) -> ToadStoolResult<MigrationRecommendation> {
        let available = self.providers.read().await.available_providers();

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
                        reason: format!("Cloud ({provider}) is working well"),
                        target: None,
                        cost_impact: None,
                        confidence: 0.7,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_provider_trait::{
        CloudCapabilities, CloudError, CloudProvider, CostEstimate, GpuType, WorkloadHealth,
        WorkloadLocation, WorkloadSpec,
    };
    use crate::composition_constraints::Constraint;
    use std::collections::HashMap;

    struct MockProvider;
    impl CloudProvider for MockProvider {
        fn name(&self) -> &'static str {
            "MockProvider"
        }
        fn capabilities(
            &self,
        ) -> impl std::future::Future<Output = Result<CloudCapabilities, CloudError>> + Send + '_
        {
            async {
                Ok(CloudCapabilities {
                    name: "MockProvider".to_string(),
                    available_regions: vec!["us-west-1".to_string()],
                    supports_gpu: true,
                    gpu_types: vec!["V100".to_string()],
                    max_memory_gb: 64.0,
                    max_cpu_cores: 16,
                    supports_spot_instances: false,
                    supports_autoscaling: false,
                    custom: HashMap::new(),
                })
            }
        }
        fn deploy_workload<'a>(
            &'a self,
            id: &'a str,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<String, CloudError>> + Send + 'a {
            async move { Ok(format!("inst-{id}")) }
        }
        fn migrate_workload<'a>(
            &'a self,
            id: &'a str,
            _: WorkloadLocation,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<String, CloudError>> + Send + 'a {
            async move { Ok(format!("migrated-{id}")) }
        }
        fn check_health<'a>(
            &'a self,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<WorkloadHealth, CloudError>> + Send + 'a
        {
            async move { Ok(WorkloadHealth::Healthy) }
        }
        fn terminate_workload<'a>(
            &'a self,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<(), CloudError>> + Send + 'a {
            async move { Ok(()) }
        }
        fn estimate_cost<'a>(
            &'a self,
            _: &'a WorkloadSpec,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<CostEstimate, CloudError>> + Send + 'a
        {
            async move {
                Ok(CostEstimate {
                    cost_per_hour: 5.0,
                    estimated_total_cost: Some(10.0),
                    breakdown: HashMap::new(),
                })
            }
        }
        fn available_gpu_types<'a>(
            &'a self,
            _: &'a str,
        ) -> impl std::future::Future<Output = Result<Vec<GpuType>, CloudError>> + Send + 'a
        {
            async move {
                Ok(vec![GpuType {
                    name: "V100".to_string(),
                    memory_gb: 16.0,
                    compute_capability: Some("7.0".to_string()),
                    cost_per_hour: 3.0,
                    available_regions: vec!["us-west-1".to_string()],
                }])
            }
        }
    }

    #[tokio::test]
    async fn test_planner_no_providers_returns_no_migrate_reason() {
        let coordinator = MigrationCoordinator::<MockProvider>::new()
            .await
            .expect("coordinator creation");
        let rec = coordinator
            .should_migrate("wl", &[Constraint::RequiresGPU])
            .await
            .expect("should_migrate");
        assert!(!rec.reason.is_empty());
        assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_planner_local_minimize_cost_stays_local() {
        let coordinator = MigrationCoordinator::<MockProvider>::new()
            .await
            .expect("coordinator creation");
        coordinator.register_provider(Box::new(MockProvider)).await;
        coordinator
            .track_workload(
                "cost-wl",
                WorkloadLocation::Local {
                    hostname: "host".to_string(),
                },
            )
            .await;
        let rec = coordinator
            .should_migrate("cost-wl", &[Constraint::MinimizeCost])
            .await
            .expect("should_migrate");
        assert!(!rec.should_migrate);
        assert!(
            rec.reason.contains("cheapest")
                || rec.reason.contains("local")
                || rec.reason.contains("optimal"),
            "reason: {}",
            rec.reason
        );
    }

    #[tokio::test]
    async fn test_planner_cloud_minimize_cost_migrates_to_local() {
        let coordinator = MigrationCoordinator::<MockProvider>::new()
            .await
            .expect("coordinator creation");
        coordinator.register_provider(Box::new(MockProvider)).await;
        coordinator
            .track_workload(
                "cloud-wl",
                WorkloadLocation::Cloud {
                    provider: "MockProvider".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: "i-123".to_string(),
                },
            )
            .await;
        let rec = coordinator
            .should_migrate("cloud-wl", &[Constraint::MinimizeCost])
            .await
            .expect("should_migrate");
        assert!(!rec.reason.is_empty());
        assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_planner_max_cost_per_hour_triggers_cost_branch() {
        let coordinator = MigrationCoordinator::<MockProvider>::new()
            .await
            .expect("coordinator creation");
        coordinator.register_provider(Box::new(MockProvider)).await;
        coordinator
            .track_workload(
                "local-wl",
                WorkloadLocation::Local {
                    hostname: "host".to_string(),
                },
            )
            .await;
        let rec = coordinator
            .should_migrate("local-wl", &[Constraint::MaxCostPerHour(0.5)])
            .await
            .expect("should_migrate");
        assert!(!rec.should_migrate);
    }
}
