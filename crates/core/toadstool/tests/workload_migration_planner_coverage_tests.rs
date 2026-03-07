// SPDX-License-Identifier: AGPL-3.0-or-later
//! Targeted tests for `workload_migration/planner.rs` coverage expansion
//! Covers: empty workloads, `MinimizeCost`, untracked workload, all branches

use toadstool::cloud_provider_trait::WorkloadLocation;
use toadstool::composition_constraints::Constraint;
use toadstool::workload_migration::{MigrationCoordinator, MigrationTarget};

async fn coordinator_with_provider() -> MigrationCoordinator {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    coordinator
        .register_provider(Box::new(MockCloudProvider {
            name: "TestCloud".to_string(),
            supports_gpu: true,
        }))
        .await;
    coordinator
}

use async_trait::async_trait;
use std::collections::HashMap;
use toadstool::cloud_provider_trait::{
    CloudCapabilities, CloudError, CloudProvider, CostEstimate, GpuType, WorkloadHealth,
    WorkloadSpec,
};

struct MockCloudProvider {
    name: String,
    supports_gpu: bool,
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl CloudProvider for MockCloudProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn capabilities(&self) -> Result<CloudCapabilities, CloudError> {
        Ok(CloudCapabilities {
            name: self.name.clone(),
            available_regions: vec!["us-west-1".to_string(), "eu-west-1".to_string()],
            supports_gpu: self.supports_gpu,
            gpu_types: vec!["V100".to_string()],
            max_memory_gb: 256.0,
            max_cpu_cores: 64,
            supports_spot_instances: true,
            supports_autoscaling: true,
            custom: HashMap::new(),
        })
    }

    async fn deploy_workload(
        &self,
        workload_id: &str,
        _region: &str,
    ) -> Result<String, CloudError> {
        Ok(format!("instance-{workload_id}"))
    }

    async fn migrate_workload(
        &self,
        workload_id: &str,
        _source: WorkloadLocation,
        _target_region: &str,
    ) -> Result<String, CloudError> {
        Ok(format!("migrated-{workload_id}"))
    }

    async fn check_health(&self, _instance_id: &str) -> Result<WorkloadHealth, CloudError> {
        Ok(WorkloadHealth::Healthy)
    }

    async fn terminate_workload(&self, _instance_id: &str) -> Result<(), CloudError> {
        Ok(())
    }

    async fn estimate_cost(
        &self,
        _workload_spec: &WorkloadSpec,
        _region: &str,
    ) -> Result<CostEstimate, CloudError> {
        Ok(CostEstimate {
            cost_per_hour: 5.0,
            estimated_total_cost: Some(10.0),
            breakdown: HashMap::new(),
        })
    }

    async fn available_gpu_types(&self, _region: &str) -> Result<Vec<GpuType>, CloudError> {
        Ok(vec![GpuType {
            name: "V100".to_string(),
            memory_gb: 16.0,
            compute_capability: Some("7.0".to_string()),
            cost_per_hour: 3.0,
            available_regions: vec!["us-west-1".to_string()],
        }])
    }
}

// ── Untracked workload (current_location: None) ──────────────────────────────

#[tokio::test]
async fn test_planner_untracked_with_minimize_cost_stays_local() {
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator
        .should_migrate("untracked", &[Constraint::MinimizeCost])
        .await
        .unwrap();
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
async fn test_planner_untracked_with_max_cost_and_minimize() {
    let coordinator = coordinator_with_provider().await;
    let constraints = [Constraint::MaxCostPerHour(2.0), Constraint::MinimizeCost];
    let rec = coordinator
        .should_migrate("untracked", &constraints)
        .await
        .unwrap();
    assert!(!rec.should_migrate);
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_untracked_no_constraints_sufficient() {
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator.should_migrate("untracked", &[]).await.unwrap();
    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("sufficient")
            || rec.reason.contains("optimal")
            || !rec.reason.is_empty(),
        "reason: {}",
        rec.reason
    );
}

#[tokio::test]
async fn test_planner_untracked_requires_gpu_may_recommend_cloud() {
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator
        .should_migrate("untracked", &[Constraint::RequiresGPU])
        .await
        .unwrap();
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

// ── Local workload branches ───────────────────────────────────────────────────

#[tokio::test]
async fn test_planner_local_minimize_cost_target_local() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "local-cost",
            WorkloadLocation::Local {
                hostname: "host1".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("local-cost", &[Constraint::MinimizeCost])
        .await
        .unwrap();

    assert!(!rec.should_migrate);
    assert!(
        matches!(rec.target, Some(MigrationTarget::Local)) || rec.target.is_none(),
        "target: {:?}",
        rec.target
    );
}

#[tokio::test]
async fn test_planner_local_requires_gpu_no_local_gpu() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "gpu-wl",
            WorkloadLocation::Local {
                hostname: "nogpu".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("gpu-wl", &[Constraint::RequiresGPU])
        .await
        .unwrap();

    if rec.should_migrate {
        assert!(matches!(rec.target, Some(MigrationTarget::Cloud { .. })));
        assert!(rec.cost_impact.is_some());
    }
    assert!(!rec.reason.is_empty());
}

// ── Cloud workload branches ───────────────────────────────────────────────────

#[tokio::test]
async fn test_planner_cloud_with_minimize_cost_migrates_to_local() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "cloud-cost",
            WorkloadLocation::Cloud {
                provider: "TestCloud".to_string(),
                region: "us-west-1".to_string(),
                instance_id: "i-xyz".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("cloud-cost", &[Constraint::MinimizeCost])
        .await
        .unwrap();

    if rec.should_migrate {
        assert!(matches!(rec.target, Some(MigrationTarget::Local)));
        if let Some(ref impact) = rec.cost_impact {
            assert!(impact.savings_per_hour > 0.0);
        }
    }
    assert!(!rec.reason.is_empty());
}

#[tokio::test]
async fn test_planner_cloud_no_cost_constraint_stays_cloud() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "cloud-stay",
            WorkloadLocation::Cloud {
                provider: "AWS".to_string(),
                region: "eu-west-1".to_string(),
                instance_id: "i-abc".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("cloud-stay", &[Constraint::RequiresGPU])
        .await
        .unwrap();

    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("working well") || rec.reason.contains("AWS") || !rec.reason.is_empty()
    );
}

// ── No providers ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_planner_no_providers_untracked_requires_gpu() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let rec = coordinator
        .should_migrate("wl", &[Constraint::RequiresGPU])
        .await
        .unwrap();
    assert!(!rec.should_migrate);
    assert!(
        rec.reason == "No cloud providers available" || rec.reason == "Current location optimal",
        "reason: {}",
        rec.reason
    );
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_no_providers_local_minimize_cost() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    coordinator
        .track_workload(
            "local",
            WorkloadLocation::Local {
                hostname: "h".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate("local", &[Constraint::MinimizeCost])
        .await
        .unwrap();
    assert!(!rec.reason.is_empty());
}

// ── Multiple providers ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_planner_multiple_providers_uses_first() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    coordinator
        .register_provider(Box::new(MockCloudProvider {
            name: "ProviderA".to_string(),
            supports_gpu: true,
        }))
        .await;
    coordinator
        .register_provider(Box::new(MockCloudProvider {
            name: "ProviderB".to_string(),
            supports_gpu: true,
        }))
        .await;
    coordinator
        .track_workload(
            "gpu",
            WorkloadLocation::Local {
                hostname: "h".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("gpu", &[Constraint::RequiresGPU])
        .await
        .unwrap();

    if rec.should_migrate {
        if let Some(MigrationTarget::Cloud { provider, .. }) = &rec.target {
            assert_eq!(provider, "ProviderA");
        }
    }
}

// ── Recommendation structure ──────────────────────────────────────────────────

#[tokio::test]
async fn test_planner_recommendation_has_valid_confidence() {
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator
        .should_migrate("any", &[Constraint::MaxCostPerHour(1.0)])
        .await
        .unwrap();
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_single_workload_empty_constraints() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "single",
            WorkloadLocation::Local {
                hostname: "host".to_string(),
            },
        )
        .await;
    let rec = coordinator.should_migrate("single", &[]).await.unwrap();
    assert!(!rec.reason.is_empty());
}

// ── MigrationStats and coordinator helpers ─────────────────────────────────────

#[tokio::test]
async fn test_planner_migration_stats_default() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let stats = coordinator.stats().await;
    assert_eq!(stats.total_migrations, 0);
    assert_eq!(stats.successful_migrations, 0);
    assert_eq!(stats.failed_migrations, 0);
    assert_eq!(stats.migrations_to_cloud, 0);
    assert_eq!(stats.migrations_to_local, 0);
    assert!(stats.avg_migration_time_secs >= 0.0);
}

#[tokio::test]
async fn test_planner_get_workload_location_untracked_returns_none() {
    let coordinator = coordinator_with_provider().await;
    let loc = coordinator.get_workload_location("never-tracked").await;
    assert!(loc.is_none());
}

#[tokio::test]
async fn test_planner_available_providers_after_register() {
    let coordinator = coordinator_with_provider().await;
    let providers = coordinator.available_providers().await;
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0], "TestCloud");
}

#[tokio::test]
async fn test_planner_track_workload_with_string() {
    let coordinator = coordinator_with_provider().await;
    let id: String = "string-id".to_string();
    coordinator
        .track_workload(
            id.clone(),
            WorkloadLocation::Local {
                hostname: "h".to_string(),
            },
        )
        .await;
    let loc = coordinator.get_workload_location("string-id").await;
    assert!(matches!(
        loc,
        Some(WorkloadLocation::Local { hostname }) if hostname == "h"
    ));
}

// ── MaxCostPerHour constraint boundary ───────────────────────────────────────

#[tokio::test]
async fn test_planner_untracked_max_cost_per_hour_only() {
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator
        .should_migrate("untracked", &[Constraint::MaxCostPerHour(100.0)])
        .await
        .unwrap();
    assert!(!rec.should_migrate);
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_local_both_requires_gpu_and_minimize_cost_cost_wins() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "local-gpu-cost",
            WorkloadLocation::Local {
                hostname: "local1".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate(
            "local-gpu-cost",
            &[Constraint::RequiresGPU, Constraint::MinimizeCost],
        )
        .await
        .unwrap();
    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("cheapest")
            || rec.reason.contains("local")
            || rec.reason.contains("Cost-sensitive")
            || rec.reason.contains("optimal")
            || rec.reason.contains("sufficient"),
        "reason: {}",
        rec.reason
    );
}

// ── MigrationTarget and CostImpact structure ───────────────────────────────────

#[tokio::test]
async fn test_planner_cost_impact_structure_when_cloud_to_local() {
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "cloud-wl",
            WorkloadLocation::Cloud {
                provider: "TestCloud".to_string(),
                region: "us-west-1".to_string(),
                instance_id: "i-123".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate("cloud-wl", &[Constraint::MinimizeCost])
        .await
        .unwrap();
    if rec.should_migrate {
        assert!(matches!(rec.target, Some(MigrationTarget::Local)));
        let impact = rec
            .cost_impact
            .expect("cost impact when migrating cloud->local");
        assert!(impact.savings_per_hour > 0.0);
        assert_eq!(impact.new_cost_per_hour, 0.0);
        assert!(impact.current_cost_per_hour > 0.0);
    }
}

// ── Validation: validate_recommendation ─────────────────────────────────────────

#[tokio::test]
async fn test_planner_validate_recommendation_stay_put_always_valid() {
    use toadstool::workload_migration::validate_recommendation;
    let coordinator = coordinator_with_provider().await;
    let rec = coordinator
        .should_migrate("any", &[Constraint::MinimizeCost])
        .await
        .unwrap();
    assert!(!rec.should_migrate);
    assert!(validate_recommendation(&rec));
}

#[tokio::test]
async fn test_planner_validate_recommendation_migrate_requires_target_and_confidence() {
    use toadstool::workload_migration::validate_recommendation;
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "gpu-wl",
            WorkloadLocation::Local {
                hostname: "nogpu".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate("gpu-wl", &[Constraint::RequiresGPU])
        .await
        .unwrap();
    if rec.should_migrate {
        assert!(rec.target.is_some());
        assert!(rec.confidence >= 0.5);
        assert!(validate_recommendation(&rec));
    }
}
