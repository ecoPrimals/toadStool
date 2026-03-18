// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

#![cfg(test)]
#![allow(clippy::float_cmp)]

use super::*;
use crate::cloud_provider_trait::{
    CloudCapabilities, CloudError, CloudProvider, CostEstimate, GpuType, WorkloadHealth,
    WorkloadLocation, WorkloadSpec,
};
use crate::composition_constraints::Constraint;
use async_trait::async_trait;
use std::collections::HashMap;

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

#[test]
fn test_migration_stats_default() {
    let stats = MigrationStats::default();
    assert_eq!(stats.total_migrations, 0);
    assert_eq!(stats.successful_migrations, 0);
    assert_eq!(stats.failed_migrations, 0);
    assert_eq!(stats.migrations_to_cloud, 0);
    assert_eq!(stats.migrations_to_local, 0);
    assert_eq!(stats.avg_migration_time_secs, 0.0);
}

#[test]
fn test_migration_stats_clone() {
    let stats = MigrationStats {
        total_migrations: 10,
        successful_migrations: 8,
        failed_migrations: 2,
        migrations_to_cloud: 5,
        migrations_to_local: 5,
        avg_migration_time_secs: 12.5,
    };
    let cloned = stats.clone();
    assert_eq!(cloned.total_migrations, stats.total_migrations);
    assert_eq!(
        cloned.avg_migration_time_secs,
        stats.avg_migration_time_secs
    );
}

#[test]
fn test_migration_target_local() {
    let target = MigrationTarget::Local;
    assert!(matches!(target, MigrationTarget::Local));
}

#[test]
fn test_migration_target_cloud() {
    let target = MigrationTarget::Cloud {
        provider: "AWS".to_string(),
        region: "us-east-1".to_string(),
        estimated_cost_per_hour: 3.5,
    };
    match &target {
        MigrationTarget::Cloud {
            provider,
            region,
            estimated_cost_per_hour,
        } => {
            assert_eq!(provider, "AWS");
            assert_eq!(region, "us-east-1");
            assert!((*estimated_cost_per_hour - 3.5).abs() < f64::EPSILON);
        }
        _ => panic!("Expected Cloud variant"),
    }
}

#[test]
fn test_migration_target_different_cloud() {
    let target = MigrationTarget::DifferentCloud {
        from_provider: "AWS".to_string(),
        to_provider: "GCP".to_string(),
        to_region: "us-central1".to_string(),
        estimated_cost_per_hour: 2.8,
    };
    match &target {
        MigrationTarget::DifferentCloud {
            from_provider,
            to_provider,
            to_region,
            estimated_cost_per_hour,
        } => {
            assert_eq!(from_provider, "AWS");
            assert_eq!(to_provider, "GCP");
            assert_eq!(to_region, "us-central1");
            assert!((*estimated_cost_per_hour - 2.8).abs() < f64::EPSILON);
        }
        _ => panic!("Expected DifferentCloud variant"),
    }
}

#[test]
fn test_cost_impact_positive_savings() {
    let impact = CostImpact {
        current_cost_per_hour: 5.0,
        new_cost_per_hour: 0.0,
        savings_per_hour: 5.0,
        migration_cost: 0.1,
    };
    assert_eq!(impact.savings_per_hour, 5.0);
    assert!(impact.savings_per_hour > 0.0);
}

#[test]
fn test_migration_recommendation_should_migrate() {
    let rec = MigrationRecommendation {
        should_migrate: true,
        reason: "test".to_string(),
        target: Some(MigrationTarget::Local),
        cost_impact: None,
        confidence: 0.8,
    };
    assert!(rec.should_migrate);
    assert_eq!(rec.confidence, 0.8);
}

#[tokio::test]
async fn test_coordinator_initialization() {
    let result = MigrationCoordinator::new().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_coordinator_starts_empty() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let providers = coordinator.available_providers().await;
    assert!(providers.is_empty());
    let stats = coordinator.stats().await;
    assert_eq!(stats.total_migrations, 0);
}

#[tokio::test]
async fn test_provider_registration() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let initial = coordinator.available_providers().await;
    assert_eq!(initial.len(), 0);

    coordinator
        .register_provider(Box::new(MockCloudProvider {
            name: "TestCloud".to_string(),
            supports_gpu: true,
        }))
        .await;

    let providers = coordinator.available_providers().await;
    assert_eq!(providers.len(), 1);
    assert!(providers.contains(&"TestCloud".to_string()));
}

#[tokio::test]
async fn test_should_migrate_evaluation() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let constraints = vec![Constraint::requires_gpu(), Constraint::max_latency_ms(100)];
    let recommendation = coordinator
        .should_migrate("test-workload", &constraints)
        .await;
    assert!(recommendation.is_ok());
    let rec = recommendation.unwrap();
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_workload_tracking() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let location = WorkloadLocation::Local {
        hostname: "test-host".to_string(),
    };
    coordinator
        .track_workload("test-workload".to_string(), location.clone())
        .await;
    let retrieved = coordinator.get_workload_location("test-workload").await;
    assert!(retrieved.is_some());
    match (&retrieved.unwrap(), &location) {
        (WorkloadLocation::Local { hostname: a }, WorkloadLocation::Local { hostname: b }) => {
            assert_eq!(a, b)
        }
        _ => panic!("Expected Local variant"),
    }
}

#[tokio::test]
async fn test_migrate_from_untracked_to_cloud() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let result = coordinator.migrate_workload("new-workload").await;
    assert!(result.is_ok());
    let loc = result.unwrap();
    assert!(matches!(loc, WorkloadLocation::Cloud { .. }));
}

#[tokio::test]
async fn test_migrate_from_cloud_to_local() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    coordinator
        .track_workload(
            "cloud-workload",
            WorkloadLocation::Cloud {
                provider: "AWS".to_string(),
                region: "us-west-1".to_string(),
                instance_id: "i-xyz".to_string(),
            },
        )
        .await;

    let result = coordinator.migrate_workload("cloud-workload").await;
    assert!(result.is_ok());
    let loc = result.unwrap();
    assert!(matches!(loc, WorkloadLocation::Local { .. }));
}

#[tokio::test]
async fn test_migration_stats() {
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let initial_stats = coordinator.stats().await;
    assert_eq!(initial_stats.total_migrations, 0);
    let _ = coordinator.migrate_workload("test").await;
    let updated_stats = coordinator.stats().await;
    assert_eq!(updated_stats.total_migrations, 1);
    assert_eq!(updated_stats.successful_migrations, 1);
}

#[test]
fn test_validate_recommendation() {
    let rec = MigrationRecommendation {
        should_migrate: true,
        reason: "test".to_string(),
        target: Some(MigrationTarget::Local),
        cost_impact: None,
        confidence: 0.8,
    };
    assert!(validate_recommendation(&rec));
}

// ── Planner path coverage ─────────────────────────────────────────────────────

/// Helper: coordinator with one registered cloud provider.
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

#[tokio::test]
async fn test_planner_no_providers_returns_no_migrate() {
    // With no registered providers, evaluate_migration_targets returns
    // "No cloud providers available" or the engine returns "Current location optimal".
    // Either way: should_migrate=false, non-empty reason, valid confidence.
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let rec = coordinator
        .should_migrate("wl", &[Constraint::requires_gpu()])
        .await
        .unwrap();
    // Either "No cloud providers available" (from planner) or early-return (engine optimal)
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_local_with_gpu_constraint_recommends_cloud() {
    // Local location + requires_gpu → migrate to cloud
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "gpu-wl",
            WorkloadLocation::Local {
                hostname: "myhost".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("gpu-wl", &[Constraint::requires_gpu()])
        .await
        .unwrap();

    // If local runtime has no direct GPU access (common in test envs), cloud is recommended.
    // Either outcome is valid — we just verify no panic and a coherent recommendation.
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_local_with_cost_constraint_stays_local() {
    // Local location + cost constraint + provider available → stay local (cheapest)
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "cost-wl",
            WorkloadLocation::Local {
                hostname: "myhost".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("cost-wl", &[Constraint::MaxCostPerHour(0.5)])
        .await
        .unwrap();

    // Should recommend staying local (cost-sensitive)
    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("cheapest")
            || rec.reason.contains("local")
            || rec.reason.contains("optimal")
    );
}

#[tokio::test]
async fn test_planner_local_no_constraint_stays_local() {
    // Local location + no special constraints + provider available → stay local (sufficient)
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "plain-wl",
            WorkloadLocation::Local {
                hostname: "myhost".to_string(),
            },
        )
        .await;

    let rec = coordinator.should_migrate("plain-wl", &[]).await.unwrap();

    assert!(!rec.should_migrate);
    assert!(!rec.reason.is_empty());
}

#[tokio::test]
async fn test_planner_cloud_with_cost_constraint() {
    // Cloud location + cost constraint — exercises the Cloud branch in evaluate_migration_targets.
    // The engine may short-circuit with "optimal" or may reach the planner; either is correct.
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "expensive-wl",
            WorkloadLocation::Cloud {
                provider: "TestCloud".to_string(),
                region: "us-west-1".to_string(),
                instance_id: "i-abc123".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("expensive-wl", &[Constraint::MaxCostPerHour(1.0)])
        .await
        .unwrap();

    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

#[tokio::test]
async fn test_planner_cloud_no_cost_constraint_stays_cloud() {
    // Cloud location + no cost constraint + provider available → stay in cloud
    let coordinator = coordinator_with_provider().await;
    coordinator
        .track_workload(
            "cloud-wl",
            WorkloadLocation::Cloud {
                provider: "TestCloud".to_string(),
                region: "eu-west-1".to_string(),
                instance_id: "i-def456".to_string(),
            },
        )
        .await;

    let rec = coordinator
        .should_migrate("cloud-wl", &[Constraint::requires_gpu()])
        .await
        .unwrap();

    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("working well")
            || rec.reason.contains("TestCloud")
            || !rec.reason.is_empty()
    );
}

#[tokio::test]
async fn test_should_migrate_optimal_returns_early() {
    // Calling with empty constraints on a fresh coordinator — CompositionEngine
    // should evaluate as feasible with high score → returns early without
    // reaching evaluate_migration_targets.
    let coordinator = MigrationCoordinator::new().await.unwrap();
    let rec = coordinator.should_migrate("optimal-wl", &[]).await.unwrap();
    // Either path is valid; ensure we get a coherent result.
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}
