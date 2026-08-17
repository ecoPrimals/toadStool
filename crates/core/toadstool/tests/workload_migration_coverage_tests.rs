// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for workload migration planner
//! Target: exercise all branches in should_migrate and evaluate_migration_targets.

use std::collections::HashMap;
use toadstool::cloud_provider_trait::{
    CloudCapabilities, CloudError, CloudProvider, CostEstimate, GpuType, NoopCloudProvider,
    WorkloadHealth, WorkloadLocation, WorkloadSpec,
};
use toadstool::composition_constraints::Constraint;
use toadstool::workload_migration::MigrationCoordinator;

struct MockProvider;
impl CloudProvider for MockProvider {
    fn name(&self) -> &'static str {
        "MockProvider"
    }
    fn capabilities(
        &self,
    ) -> impl std::future::Future<Output = Result<CloudCapabilities, CloudError>> + Send + '_ {
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
    ) -> impl std::future::Future<Output = Result<WorkloadHealth, CloudError>> + Send + 'a {
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
    ) -> impl std::future::Future<Output = Result<CostEstimate, CloudError>> + Send + 'a {
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
    ) -> impl std::future::Future<Output = Result<Vec<GpuType>, CloudError>> + Send + 'a {
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

// ─── No providers ───────────────────────────────────────────────────────────

#[tokio::test]
async fn planner_no_providers_returns_no_migrate() {
    let coordinator = MigrationCoordinator::<NoopCloudProvider>::new()
        .await
        .expect("coordinator");
    let rec = coordinator
        .should_migrate("wl", &[Constraint::RequiresGPU])
        .await
        .expect("should_migrate");
    assert!(!rec.reason.is_empty());
    assert!(rec.reason.contains("No cloud providers") || rec.reason.contains("optimal"));
}

// ─── Local workload branches ─────────────────────────────────────────────────

#[tokio::test]
async fn planner_local_minimize_cost_stays_local() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
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
async fn planner_local_max_cost_per_hour_stays_local() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
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

#[tokio::test]
async fn planner_local_no_constraints_sufficient() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
    coordinator
        .track_workload(
            "local-wl",
            WorkloadLocation::Local {
                hostname: "host".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate("local-wl", &[])
        .await
        .expect("should_migrate");
    assert!(!rec.should_migrate);
    assert!(
        rec.reason.contains("sufficient")
            || rec.reason.contains("optimal")
            || rec.reason.contains("local")
    );
}

#[tokio::test]
async fn planner_local_requires_gpu_may_migrate() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
    coordinator
        .track_workload(
            "gpu-wl",
            WorkloadLocation::Local {
                hostname: "host".to_string(),
            },
        )
        .await;
    let rec = coordinator
        .should_migrate("gpu-wl", &[Constraint::RequiresGPU])
        .await
        .expect("should_migrate");
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}

// ─── Cloud workload branches ────────────────────────────────────────────────

#[tokio::test]
async fn planner_cloud_minimize_cost() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
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
async fn planner_cloud_max_cost_per_hour() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
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
        .should_migrate("cloud-wl", &[Constraint::MaxCostPerHour(0.1)])
        .await
        .expect("should_migrate");
    assert!(!rec.reason.is_empty());
}

#[tokio::test]
async fn planner_cloud_no_cost_constraint() {
    let coordinator = MigrationCoordinator::<MockProvider>::new()
        .await
        .expect("coordinator");
    coordinator
        .register_provider(std::sync::Arc::new(MockProvider))
        .await;
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
        .should_migrate("cloud-wl", &[Constraint::RequiresGPU])
        .await
        .expect("should_migrate");
    assert!(!rec.reason.is_empty());
    assert!(rec.confidence >= 0.0 && rec.confidence <= 1.0);
}
