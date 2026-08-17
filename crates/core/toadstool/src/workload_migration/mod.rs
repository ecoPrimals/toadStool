// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

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
//! use toadstool::cloud_provider_trait::NoopCloudProvider;
//! use toadstool::composition_constraints::Constraint;
//! use toadstool::workload_migration::MigrationCoordinator;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let coordinator = MigrationCoordinator::<NoopCloudProvider>::new().await?;
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
    PreMigrationSnapshot, PreflightOutcome, ResourceRequirements, validate_migration,
    validate_preflight, validate_recommendation,
};

use crate::ToadStoolResult;
use crate::cloud_provider_trait::{
    CloudProvider, CloudProviderRegistry, NoopCloudProvider, WorkloadLocation,
};
use crate::composition_engine::CompositionEngine;
use crate::fractal_integration::FractalRuntime;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::info;

/// Migration coordinator
///
/// Manages workload migrations between local and cloud environments.
pub struct MigrationCoordinator<P: CloudProvider = NoopCloudProvider> {
    /// Current runtime
    pub(super) runtime: Arc<FractalRuntime>,

    /// Composition engine for constraint evaluation
    pub(super) engine: Arc<CompositionEngine>,

    /// Cloud provider registry
    pub(super) providers: Arc<RwLock<CloudProviderRegistry<P>>>,

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
        /// Cloud provider name.
        provider: String,
        /// Target region.
        region: String,
        /// Estimated cost per hour.
        estimated_cost_per_hour: f64,
    },

    /// Move to different cloud
    DifferentCloud {
        /// Current cloud provider.
        from_provider: String,
        /// Target cloud provider.
        to_provider: String,
        /// Target region.
        to_region: String,
        /// Estimated cost per hour.
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

impl<P: CloudProvider> MigrationCoordinator<P> {
    /// Create a new migration coordinator
    ///
    /// # Errors
    ///
    /// Returns error if fractal runtime or composition engine initialization fails.
    #[cfg(feature = "runtime")]
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
    pub async fn register_provider(&self, provider: Box<P>) {
        let name = provider.name().to_string();
        self.providers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .register(provider);
        info!("📦 Registered cloud provider: {}", name);
    }

    /// Get available providers
    pub async fn available_providers(&self) -> Vec<String> {
        let providers = self.providers.read().unwrap_or_else(|e| e.into_inner());
        providers.available_providers()
    }

    /// Get workload location
    pub async fn get_workload_location(&self, workload_id: &str) -> Option<WorkloadLocation> {
        let locations = self
            .workload_locations
            .read()
            .unwrap_or_else(|e| e.into_inner());
        locations.get(workload_id).cloned()
    }

    /// Track workload location
    pub async fn track_workload(&self, workload_id: impl Into<String>, location: WorkloadLocation) {
        let mut locations = self
            .workload_locations
            .write()
            .unwrap_or_else(|e| e.into_inner());
        locations.insert(workload_id.into(), location);
    }

    /// Get migration statistics
    pub async fn stats(&self) -> MigrationStats {
        self.stats.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
}
