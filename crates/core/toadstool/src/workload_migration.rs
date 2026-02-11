// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Workload Migration Coordinator
//!
//! This module implements seamless workload migration between local and cloud,
//! and between different cloud providers.
//!
//! # Philosophy
//!
//! **Seamless Mobility**: Workloads should move transparently based on constraints,
//! not be locked to where they started. Migration is constraint-driven, not manual.
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool::workload_migration::MigrationCoordinator;
//! use toadstool::composition_constraints::Constraint;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let coordinator = MigrationCoordinator::new().await?;
//!
//! // Evaluate if workload should migrate
//! let should_migrate = coordinator.should_migrate(
//!     "my-workload",
//!     &[Constraint::max_cost_per_hour(1.0), Constraint::requires_gpu()]
//! ).await?;
//!
//! if should_migrate {
//!     // Migrate to optimal location
//!     let new_location = coordinator.migrate_workload("my-workload").await?;
//!     println!("Migrated to: {:?}", new_location);
//! }
//! # Ok(())
//! # }
//! ```

use crate::cloud_provider_trait::*;
use crate::composition_constraints::*;
use crate::composition_engine::CompositionEngine;
use crate::fractal_integration::FractalRuntime;
use crate::ToadStoolResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Migration coordinator
///
/// Manages workload migrations between local and cloud environments.
pub struct MigrationCoordinator {
    /// Current runtime
    runtime: Arc<FractalRuntime>,

    /// Composition engine for constraint evaluation
    engine: Arc<CompositionEngine>,

    /// Cloud provider registry
    providers: Arc<RwLock<CloudProviderRegistry>>,

    /// Active workload locations
    workload_locations: Arc<RwLock<HashMap<String, WorkloadLocation>>>,

    /// Migration statistics
    stats: Arc<RwLock<MigrationStats>>,
}

/// Migration statistics
#[derive(Debug, Default, Clone)]
pub struct MigrationStats {
    /// Total migrations performed
    pub total_migrations: u64,

    /// Successful migrations
    pub successful_migrations: u64,

    /// Failed migrations
    pub failed_migrations: u64,

    /// Migrations to cloud
    pub migrations_to_cloud: u64,

    /// Migrations to local
    pub migrations_to_local: u64,

    /// Average migration time (seconds)
    pub avg_migration_time_secs: f64,
}

/// Migration recommendation
#[derive(Debug, Clone)]
pub struct MigrationRecommendation {
    /// Should migrate?
    pub should_migrate: bool,

    /// Reason for recommendation
    pub reason: String,

    /// Target location (if migration recommended)
    pub target: Option<MigrationTarget>,

    /// Estimated cost impact
    pub cost_impact: Option<CostImpact>,

    /// Confidence level (0.0-1.0)
    pub confidence: f64,
}

/// Migration target
#[derive(Debug, Clone)]
pub enum MigrationTarget {
    /// Stay local
    Local,

    /// Move to cloud
    Cloud {
        provider: String,
        region: String,
        estimated_cost_per_hour: f64,
    },

    /// Move to different cloud
    DifferentCloud {
        from_provider: String,
        to_provider: String,
        to_region: String,
        estimated_cost_per_hour: f64,
    },
}

/// Cost impact of migration
#[derive(Debug, Clone)]
pub struct CostImpact {
    /// Current cost per hour
    pub current_cost_per_hour: f64,

    /// New cost per hour
    pub new_cost_per_hour: f64,

    /// Savings per hour (negative if more expensive)
    pub savings_per_hour: f64,

    /// Estimated migration cost (one-time)
    pub migration_cost: f64,
}

impl MigrationCoordinator {
    /// Create a new migration coordinator
    pub async fn new() -> ToadStoolResult<Self> {
        let runtime = Arc::new(FractalRuntime::init().await?);
        let engine = Arc::new(CompositionEngine::new(Arc::clone(&runtime))?);

        Ok(Self {
            runtime,
            engine,
            providers: Arc::new(RwLock::new(CloudProviderRegistry::new())),
            workload_locations: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MigrationStats::default())),
        })
    }

    /// Register a cloud provider
    pub async fn register_provider(&self, provider: Box<dyn CloudProvider>) {
        let name = provider.name().to_string();
        let mut providers = self.providers.write().await;
        providers.register(provider);
        info!("📦 Registered cloud provider: {}", name);
    }

    /// Get available providers
    pub async fn available_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.available_providers()
    }

    /// Evaluate if workload should migrate
    pub async fn should_migrate(
        &self,
        workload_id: &str,
        constraints: &[Constraint],
    ) -> ToadStoolResult<MigrationRecommendation> {
        info!("🔍 Evaluating migration for workload: {}", workload_id);

        // Get current location
        let current_location = self.get_workload_location(workload_id).await;

        // Create composition request from constraints
        let request = CompositionRequest {
            name: workload_id.to_string(),
            constraints: constraints.to_vec(),
            priority: ConstraintPriority::Normal,
            metadata: HashMap::new(),
        };

        // Evaluate current location
        let current_eval = self.engine.evaluate(&request).await?;

        debug!(
            "Current location feasibility: {}, score: {}",
            current_eval.is_feasible, current_eval.overall_score
        );

        // If current location is perfect, no need to migrate
        if current_eval.is_feasible && current_eval.overall_score >= 0.9 {
            return Ok(MigrationRecommendation {
                should_migrate: false,
                reason: "Current location optimal".to_string(),
                target: None,
                cost_impact: None,
                confidence: 1.0,
            });
        }

        // Check if migration could improve
        let recommendation = self
            .evaluate_migration_targets(workload_id, constraints, &current_location)
            .await?;

        Ok(recommendation)
    }

    /// Evaluate potential migration targets
    async fn evaluate_migration_targets(
        &self,
        _workload_id: &str,
        constraints: &[Constraint],
        current_location: &Option<WorkloadLocation>,
    ) -> ToadStoolResult<MigrationRecommendation> {
        // Check cloud providers
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

        // For now, simple logic: if local is struggling and we have cloud, recommend cloud
        // In full implementation, this would evaluate each provider/region combination

        let has_cost_constraint = constraints
            .iter()
            .any(|c| matches!(c, Constraint::MaxCostPerHour(_) | Constraint::MinimizeCost));

        let requires_gpu = constraints
            .iter()
            .any(|c| matches!(c, Constraint::RequiresGPU));

        // Simple recommendation logic
        match current_location {
            None | Some(WorkloadLocation::Local { .. }) => {
                if requires_gpu && !self.runtime.has_direct_gpu_access() {
                    // Local doesn't have GPU, cloud might
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
                    // Cost-sensitive workload - prefer local if possible
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
                    // Running in cloud but cost-sensitive - consider moving to local
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

    /// Migrate workload based on recommendation
    pub async fn migrate_workload(&self, workload_id: &str) -> ToadStoolResult<WorkloadLocation> {
        info!("🚀 Migrating workload: {}", workload_id);

        let start = std::time::Instant::now();

        // Get current location
        let current = self.get_workload_location(workload_id).await;

        // For now, simple implementation: if local, "move" to cloud; if cloud, "move" to local
        let new_location = match current {
            None | Some(WorkloadLocation::Local { .. }) => {
                // Simulate cloud deployment
                info!("📤 Migrating {} to cloud", workload_id);
                WorkloadLocation::Cloud {
                    provider: "SimulatedCloud".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: format!("instance-{}", workload_id),
                }
            }
            Some(WorkloadLocation::Cloud { .. }) => {
                // EVOLVED: Discover local node's actual hostname from environment
                info!("📥 Migrating {} to local", workload_id);
                let hostname = std::env::var("HOSTNAME")
                    .or_else(|_| std::env::var("HOST"))
                    .or_else(|_| std::env::var("COMPUTERNAME")) // Windows
                    .unwrap_or_else(|_| {
                        // Fallback: use node ID (self-knowledge)
                        format!("node-{}", uuid::Uuid::new_v4())
                    });
                WorkloadLocation::Local { hostname }
            }
        };

        // Update location
        let mut locations = self.workload_locations.write().await;
        locations.insert(workload_id.to_string(), new_location.clone());

        // Update stats
        let duration = start.elapsed();
        self.update_migration_stats(true, &new_location, duration.as_secs_f64())
            .await;

        info!("✅ Migration complete: {:?}", new_location);

        Ok(new_location)
    }

    /// Get workload location
    pub async fn get_workload_location(&self, workload_id: &str) -> Option<WorkloadLocation> {
        let locations = self.workload_locations.read().await;
        locations.get(workload_id).cloned()
    }

    /// Track workload location
    pub async fn track_workload(&self, workload_id: impl Into<String>, location: WorkloadLocation) {
        let mut locations = self.workload_locations.write().await;
        locations.insert(workload_id.into(), location);
    }

    /// Update migration statistics
    async fn update_migration_stats(
        &self,
        success: bool,
        new_location: &WorkloadLocation,
        duration_secs: f64,
    ) {
        let mut stats = self.stats.write().await;

        stats.total_migrations += 1;

        if success {
            stats.successful_migrations += 1;
        } else {
            stats.failed_migrations += 1;
        }

        match new_location {
            WorkloadLocation::Local { .. } => stats.migrations_to_local += 1,
            WorkloadLocation::Cloud { .. } => stats.migrations_to_cloud += 1,
        }

        // Update running average
        let total = stats.total_migrations as f64;
        stats.avg_migration_time_secs =
            ((stats.avg_migration_time_secs * (total - 1.0)) + duration_secs) / total;
    }

    /// Get migration statistics
    pub async fn stats(&self) -> MigrationStats {
        self.stats.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_provider_trait::{
        CloudCapabilities, CloudError, CloudProvider, CostEstimate, GpuType, WorkloadHealth,
        WorkloadLocation, WorkloadSpec,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;

    // ─── Mock Cloud Provider ─────────────────────────────────────────────────

    struct MockCloudProvider {
        name: String,
        supports_gpu: bool,
    }

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
            Ok(format!("instance-{}", workload_id))
        }

        async fn migrate_workload(
            &self,
            workload_id: &str,
            _source: WorkloadLocation,
            _target_region: &str,
        ) -> Result<String, CloudError> {
            Ok(format!("migrated-{}", workload_id))
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

    // ─── MigrationStats tests ────────────────────────────────────────────────

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
    fn test_migration_stats_debug() {
        let stats = MigrationStats::default();
        let _ = format!("{:?}", stats);
    }

    // ─── MigrationTarget enum tests ──────────────────────────────────────────

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
    fn test_migration_target_clone() {
        let target = MigrationTarget::Cloud {
            provider: "Azure".to_string(),
            region: "eastus".to_string(),
            estimated_cost_per_hour: 4.0,
        };
        let cloned = target.clone();
        assert!(matches!(cloned, MigrationTarget::Cloud { .. }));
    }

    // ─── CostImpact tests ────────────────────────────────────────────────────

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
    fn test_cost_impact_negative_savings() {
        let impact = CostImpact {
            current_cost_per_hour: 0.0,
            new_cost_per_hour: 5.0,
            savings_per_hour: -5.0,
            migration_cost: 0.5,
        };
        assert_eq!(impact.savings_per_hour, -5.0);
        assert_eq!(impact.migration_cost, 0.5);
    }

    #[test]
    fn test_cost_impact_zero_savings() {
        let impact = CostImpact {
            current_cost_per_hour: 3.0,
            new_cost_per_hour: 3.0,
            savings_per_hour: 0.0,
            migration_cost: 0.0,
        };
        assert_eq!(impact.savings_per_hour, 0.0);
    }

    #[test]
    fn test_cost_impact_clone() {
        let impact = CostImpact {
            current_cost_per_hour: 1.0,
            new_cost_per_hour: 2.0,
            savings_per_hour: -1.0,
            migration_cost: 0.2,
        };
        let cloned = impact.clone();
        assert_eq!(cloned.current_cost_per_hour, impact.current_cost_per_hour);
        assert_eq!(cloned.new_cost_per_hour, impact.new_cost_per_hour);
    }

    // ─── MigrationRecommendation tests ───────────────────────────────────────

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

    #[test]
    fn test_migration_recommendation_no_migrate() {
        let rec = MigrationRecommendation {
            should_migrate: false,
            reason: "Current location optimal".to_string(),
            target: None,
            cost_impact: None,
            confidence: 1.0,
        };
        assert!(!rec.should_migrate);
        assert_eq!(rec.confidence, 1.0);
    }

    #[test]
    fn test_migration_recommendation_with_cost_impact() {
        let rec = MigrationRecommendation {
            should_migrate: true,
            reason: "Cost savings available".to_string(),
            target: Some(MigrationTarget::Local),
            cost_impact: Some(CostImpact {
                current_cost_per_hour: 5.0,
                new_cost_per_hour: 0.0,
                savings_per_hour: 5.0,
                migration_cost: 0.1,
            }),
            confidence: 0.9,
        };
        assert!(rec.should_migrate);
        let impact = rec.cost_impact.unwrap();
        assert_eq!(impact.savings_per_hour, 5.0);
    }

    #[test]
    fn test_migration_recommendation_cloud_target() {
        let rec = MigrationRecommendation {
            should_migrate: true,
            reason: "GPU required".to_string(),
            target: Some(MigrationTarget::Cloud {
                provider: "AWS".to_string(),
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
        };
        assert!(rec.should_migrate);
        assert!(matches!(rec.target, Some(MigrationTarget::Cloud { .. })));
    }

    // ─── Coordinator initialization ───────────────────────────────────────────

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

    // ─── Provider registration tests ──────────────────────────────────────────

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
    async fn test_register_multiple_providers() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        coordinator
            .register_provider(Box::new(MockCloudProvider {
                name: "AWS".to_string(),
                supports_gpu: true,
            }))
            .await;
        coordinator
            .register_provider(Box::new(MockCloudProvider {
                name: "GCP".to_string(),
                supports_gpu: false,
            }))
            .await;

        let providers = coordinator.available_providers().await;
        assert!(providers.len() >= 2);
        assert!(providers.contains(&"AWS".to_string()));
        assert!(providers.contains(&"GCP".to_string()));
    }

    // ─── should_migrate evaluation tests ────────────────────────────────────

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
    async fn test_should_migrate_no_providers() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        // No providers registered

        let rec = coordinator
            .should_migrate("workload", &[Constraint::requires_gpu()])
            .await
            .unwrap();

        // Either "Current location optimal" (engine gave high score) or "No cloud providers"
        assert!(
            rec.reason.contains("No cloud providers")
                || rec.reason.contains("optimal")
                || rec.reason.contains("sufficient")
        );
    }

    #[tokio::test]
    async fn test_should_migrate_with_provider_cost_constraint_local() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        coordinator
            .register_provider(Box::new(MockCloudProvider {
                name: "TestCloud".to_string(),
                supports_gpu: true,
            }))
            .await;

        coordinator
            .track_workload(
                "cost-workload",
                WorkloadLocation::Local {
                    hostname: "local-node".to_string(),
                },
            )
            .await;

        let rec = coordinator
            .should_migrate(
                "cost-workload",
                &[Constraint::MaxCostPerHour(1.0), Constraint::MinimizeCost],
            )
            .await
            .unwrap();

        // Cost-sensitive workload on local: prefer stay local (or engine may deem current optimal)
        assert!(
            !rec.should_migrate
                || rec.reason.contains("optimal")
                || rec.reason.contains("Cost-sensitive")
                || rec.reason.contains("local")
        );
        assert!(!rec.reason.is_empty());
    }

    #[tokio::test]
    async fn test_should_migrate_with_provider_cost_constraint_cloud() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        coordinator
            .register_provider(Box::new(MockCloudProvider {
                name: "TestCloud".to_string(),
                supports_gpu: true,
            }))
            .await;

        coordinator
            .track_workload(
                "cloud-cost-workload",
                WorkloadLocation::Cloud {
                    provider: "TestCloud".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: "inst-123".to_string(),
                },
            )
            .await;

        let rec = coordinator
            .should_migrate(
                "cloud-cost-workload",
                &[Constraint::MaxCostPerHour(1.0), Constraint::MinimizeCost],
            )
            .await
            .unwrap();

        // Cost-sensitive workload on cloud: recommend migrate to local, or engine may deem optimal
        if rec.should_migrate {
            assert!(matches!(rec.target, Some(MigrationTarget::Local)));
            assert!(rec.cost_impact.is_some());
            let impact = rec.cost_impact.unwrap();
            assert!(impact.savings_per_hour > 0.0);
        } else {
            assert!(rec.reason.contains("optimal") || rec.reason.contains("Cloud"));
        }
    }

    #[tokio::test]
    async fn test_should_migrate_cloud_no_cost_constraint() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        coordinator
            .register_provider(Box::new(MockCloudProvider {
                name: "MyCloud".to_string(),
                supports_gpu: true,
            }))
            .await;

        coordinator
            .track_workload(
                "cloud-workload",
                WorkloadLocation::Cloud {
                    provider: "MyCloud".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: "inst-456".to_string(),
                },
            )
            .await;

        let rec = coordinator
            .should_migrate("cloud-workload", &[Constraint::requires_gpu()])
            .await
            .unwrap();

        // Cloud workload without cost constraint: stay or optimal
        assert!(!rec.should_migrate);
        assert!(!rec.reason.is_empty());
    }

    #[tokio::test]
    async fn test_should_migrate_empty_constraints() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        let rec = coordinator.should_migrate("workload", &[]).await.unwrap();
        assert!(!rec.reason.is_empty());
    }

    // ─── Workload tracking tests ──────────────────────────────────────────────

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
    async fn test_get_workload_location_nonexistent() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        let location = coordinator.get_workload_location("nonexistent").await;
        assert!(location.is_none());
    }

    #[tokio::test]
    async fn test_track_workload_overwrites() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        coordinator
            .track_workload(
                "w1",
                WorkloadLocation::Local {
                    hostname: "host-a".to_string(),
                },
            )
            .await;

        coordinator
            .track_workload(
                "w1",
                WorkloadLocation::Cloud {
                    provider: "AWS".to_string(),
                    region: "us-east-1".to_string(),
                    instance_id: "i-123".to_string(),
                },
            )
            .await;

        let loc = coordinator.get_workload_location("w1").await.unwrap();
        assert!(matches!(loc, WorkloadLocation::Cloud { .. }));
    }

    #[tokio::test]
    async fn test_track_workload_accepts_into_string() {
        let coordinator = MigrationCoordinator::new().await.unwrap();
        coordinator
            .track_workload(
                String::from("dynamic-id"),
                WorkloadLocation::Local {
                    hostname: "x".to_string(),
                },
            )
            .await;
        assert!(coordinator
            .get_workload_location("dynamic-id")
            .await
            .is_some());
    }

    // ─── migrate_workload transition tests ────────────────────────────────────

    #[tokio::test]
    async fn test_migration_stats() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        let initial_stats = coordinator.stats().await;
        assert_eq!(initial_stats.total_migrations, 0);

        let result = coordinator.migrate_workload("test").await;
        assert!(result.is_ok());

        let updated_stats = coordinator.stats().await;
        assert_eq!(updated_stats.total_migrations, 1);
        assert_eq!(updated_stats.successful_migrations, 1);
        assert_eq!(updated_stats.migrations_to_cloud, 1);
    }

    #[tokio::test]
    async fn test_migrate_from_untracked_to_cloud() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        let result = coordinator.migrate_workload("new-workload").await;
        assert!(result.is_ok());

        let loc = result.unwrap();
        assert!(matches!(loc, WorkloadLocation::Cloud { .. }));
        match &loc {
            WorkloadLocation::Cloud {
                provider,
                region,
                instance_id,
            } => {
                assert_eq!(provider, "SimulatedCloud");
                assert_eq!(region, "us-west-1");
                assert!(instance_id.starts_with("instance-new-workload"));
            }
            _ => {}
        }

        let stored = coordinator.get_workload_location("new-workload").await;
        assert!(stored.is_some());
        match (&stored.unwrap(), &loc) {
            (
                WorkloadLocation::Cloud {
                    provider: pa,
                    region: ra,
                    instance_id: ia,
                },
                WorkloadLocation::Cloud {
                    provider: pb,
                    region: rb,
                    instance_id: ib,
                },
            ) => {
                assert_eq!(pa, pb);
                assert_eq!(ra, rb);
                assert_eq!(ia, ib);
            }
            _ => panic!("Expected matching Cloud variant"),
        }
    }

    #[tokio::test]
    async fn test_migrate_from_local_to_cloud() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        coordinator
            .track_workload(
                "local-workload",
                WorkloadLocation::Local {
                    hostname: "my-machine".to_string(),
                },
            )
            .await;

        let result = coordinator.migrate_workload("local-workload").await;
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
        match &loc {
            WorkloadLocation::Local { hostname } => {
                assert!(
                    hostname.starts_with("node-")
                        || !std::env::var("HOSTNAME").unwrap_or_default().is_empty()
                        || !std::env::var("HOST").unwrap_or_default().is_empty()
                );
            }
            _ => {}
        }
    }

    #[tokio::test]
    async fn test_migration_stats_migrations_to_local() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        coordinator
            .track_workload(
                "cloud-workload",
                WorkloadLocation::Cloud {
                    provider: "AWS".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: "i-abc".to_string(),
                },
            )
            .await;

        let _ = coordinator.migrate_workload("cloud-workload").await;

        let stats = coordinator.stats().await;
        assert_eq!(stats.migrations_to_local, 1);
        assert_eq!(stats.migrations_to_cloud, 0);
    }

    #[tokio::test]
    async fn test_migration_stats_avg_time_updates() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        coordinator.migrate_workload("a").await.unwrap();
        let stats1 = coordinator.stats().await;
        assert!(stats1.avg_migration_time_secs >= 0.0);

        coordinator
            .track_workload(
                "b",
                WorkloadLocation::Cloud {
                    provider: "x".to_string(),
                    region: "y".to_string(),
                    instance_id: "z".to_string(),
                },
            )
            .await;
        coordinator.migrate_workload("b").await.unwrap();

        let stats2 = coordinator.stats().await;
        assert_eq!(stats2.total_migrations, 2);
        assert!(stats2.avg_migration_time_secs >= 0.0);
    }

    #[tokio::test]
    async fn test_multiple_migrations_preserve_state() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        for i in 0..3 {
            let id = format!("workload-{}", i);
            coordinator
                .track_workload(
                    &id,
                    WorkloadLocation::Cloud {
                        provider: "A".to_string(),
                        region: "r".to_string(),
                        instance_id: format!("i-{}", i),
                    },
                )
                .await;
            let loc = coordinator.migrate_workload(&id).await.unwrap();
            assert!(matches!(loc, WorkloadLocation::Local { .. }));
        }

        let stats = coordinator.stats().await;
        assert_eq!(stats.total_migrations, 3);
        assert_eq!(stats.successful_migrations, 3);
    }
}
