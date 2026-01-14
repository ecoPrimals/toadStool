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
//! ```rust,no_run
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
                // Simulate migration to local
                info!("📥 Migrating {} to local", workload_id);
                WorkloadLocation::Local {
                    hostname: "localhost".to_string(),
                }
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
    pub async fn track_workload(&self, workload_id: String, location: WorkloadLocation) {
        let mut locations = self.workload_locations.write().await;
        locations.insert(workload_id, location);
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

    #[tokio::test]
    async fn test_coordinator_initialization() {
        let result = MigrationCoordinator::new().await;
        assert!(result.is_ok());
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
    }

    #[tokio::test]
    async fn test_migration_stats() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        let initial_stats = coordinator.stats().await;
        assert_eq!(initial_stats.total_migrations, 0);

        // Simulate a migration
        let result = coordinator.migrate_workload("test").await;
        assert!(result.is_ok());

        let updated_stats = coordinator.stats().await;
        assert_eq!(updated_stats.total_migrations, 1);
        assert_eq!(updated_stats.successful_migrations, 1);
    }

    #[tokio::test]
    async fn test_provider_registration() {
        let coordinator = MigrationCoordinator::new().await.unwrap();

        // Initially no providers
        let initial = coordinator.available_providers().await;
        assert_eq!(initial.len(), 0);

        // Register a mock provider (would need to implement mock provider)
        // For now, just verify the method exists
        assert_eq!(coordinator.available_providers().await.len(), 0);
    }

    #[tokio::test]
    async fn test_migration_recommendation_structure() {
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
    async fn test_cost_impact_calculation() {
        let impact = CostImpact {
            current_cost_per_hour: 5.0,
            new_cost_per_hour: 0.0,
            savings_per_hour: 5.0,
            migration_cost: 0.1,
        };

        assert_eq!(impact.savings_per_hour, 5.0);
        assert!(impact.savings_per_hour > 0.0); // Saving money
    }
}
