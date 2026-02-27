// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

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
//! let should_migrate = coordinator.should_migrate(
//!     "my-workload",
//!     &[Constraint::max_cost_per_hour(1.0), Constraint::requires_gpu()]
//! ).await?;
//!
//! if should_migrate.should_migrate {
//!     let new_location = coordinator.migrate_workload("my-workload").await?;
//!     println!("Migrated to: {:?}", new_location);
//! }
//! # Ok(())
//! # }
//! ```

mod executor;
mod planner;
mod tests;
mod validation;

pub use validation::{
    validate_migration, validate_preflight, validate_recommendation, PreMigrationSnapshot,
    PreflightOutcome, ResourceRequirements,
};

use crate::cloud_provider_trait::*;
use crate::composition_engine::CompositionEngine;
use crate::fractal_integration::FractalRuntime;
use crate::ToadStoolResult;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Migration coordinator
///
/// Manages workload migrations between local and cloud environments.
pub struct MigrationCoordinator {
    /// Current runtime
    pub(super) runtime: Arc<FractalRuntime>,

    /// Composition engine for constraint evaluation
    pub(super) engine: Arc<CompositionEngine>,

    /// Cloud provider registry
    pub(super) providers: Arc<RwLock<CloudProviderRegistry>>,

    /// Active workload locations
    pub(super) workload_locations: Arc<RwLock<HashMap<String, WorkloadLocation>>>,

    /// Migration statistics
    pub(super) stats: Arc<RwLock<MigrationStats>>,
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

    /// Get migration statistics
    pub async fn stats(&self) -> MigrationStats {
        self.stats.read().await.clone()
    }
}
