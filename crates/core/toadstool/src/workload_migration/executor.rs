// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-only

//! Migration execution and state transfer logic.

use crate::cloud_provider_trait::WorkloadLocation;

use super::MigrationCoordinator;
use crate::ToadStoolResult;
use tracing::info;

impl MigrationCoordinator {
    /// Migrate workload based on recommendation
    pub async fn migrate_workload(&self, workload_id: &str) -> ToadStoolResult<WorkloadLocation> {
        info!("🚀 Migrating workload: {}", workload_id);

        let start = std::time::Instant::now();

        let current = self.get_workload_location(workload_id).await;

        let new_location = match current {
            None | Some(WorkloadLocation::Local { .. }) => {
                info!("📤 Migrating {} to cloud", workload_id);
                WorkloadLocation::Cloud {
                    provider: "SimulatedCloud".to_string(),
                    region: "us-west-1".to_string(),
                    instance_id: format!("instance-{workload_id}"),
                }
            }
            Some(WorkloadLocation::Cloud { .. }) => {
                info!("📥 Migrating {} to local", workload_id);
                let hostname = std::env::var("HOSTNAME")
                    .or_else(|_| std::env::var("HOST"))
                    .or_else(|_| std::env::var("COMPUTERNAME"))
                    .unwrap_or_else(|_| format!("node-{}", uuid::Uuid::new_v4()));
                WorkloadLocation::Local { hostname }
            }
        };

        let mut locations = self.workload_locations.write().await;
        locations.insert(workload_id.to_string(), new_location.clone());

        let duration = start.elapsed();
        self.update_migration_stats(true, &new_location, duration.as_secs_f64())
            .await;

        info!("✅ Migration complete: {:?}", new_location);

        Ok(new_location)
    }

    /// Update migration statistics
    pub(super) async fn update_migration_stats(
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

        #[allow(clippy::cast_precision_loss)]
        let total = stats.total_migrations as f64;
        stats.avg_migration_time_secs =
            ((stats.avg_migration_time_secs * (total - 1.0)) + duration_secs) / total;
    }
}
