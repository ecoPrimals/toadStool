// SPDX-License-Identifier: AGPL-3.0-or-later
//! Migration Operations
//!
//! Extension trait for workload migration operations between platforms.
//!
//! ## Preview status
//!
//! Most platform-specific steps still return [`crate::CliError::NotImplemented`] until
//! the corresponding capability providers are wired (pause, transfer, restore, etc.).
//! The `universal migrate` CLI subcommand is gated behind the `migration-preview` Cargo
//! feature so default builds do not expose a command that always fails at runtime.
//! Library callers (tests, internal orchestration) may invoke [`MigrationOps`] directly.

use crate::Result;
use std::future::Future;
use tokio::time::Duration;
use tracing::info;

use crate::universal::types::{
    MigrationPlan, MigrationType, ReplicationHandle, WorkloadCheckpoint, WorkloadExport,
    WorkloadSnapshot,
};

/// Migration operations trait
pub trait MigrationOps {
    /// Create migration plan
    fn create_migration_plan(
        &self,
        source: &str,
        target: &str,
    ) -> impl Future<Output = Result<MigrationPlan>> + Send;

    /// Execute live migration
    fn execute_live_migration(
        &self,
        plan: &MigrationPlan,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Execute cold migration
    fn execute_cold_migration(
        &self,
        plan: &MigrationPlan,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Execute hot migration
    fn execute_hot_migration(
        &self,
        plan: &MigrationPlan,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Execute clone migration
    fn execute_clone_migration(
        &self,
        plan: &MigrationPlan,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Pause workload
    fn pause_workload(&self, platform: &str) -> impl Future<Output = Result<()>> + Send;

    /// Verify migration success
    fn verify_migration_success(
        &self,
        plan: &MigrationPlan,
    ) -> impl Future<Output = Result<bool>> + Send;

    /// Prepare target platform for migration
    fn prepare_target_platform(&self, target: &str) -> impl Future<Output = Result<()>> + Send;
    /// Create a checkpoint of the workload state
    fn create_workload_checkpoint(
        &self,
        biome: &str,
    ) -> impl Future<Output = Result<WorkloadCheckpoint>> + Send;
    /// Transfer checkpoint data to target platform
    fn transfer_checkpoint(
        &self,
        checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Restore workload from checkpoint on target
    fn restore_from_checkpoint(
        &self,
        checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Remove workload from source after migration
    fn cleanup_source_workload(&self, biome: &str) -> impl Future<Output = Result<()>> + Send;
    /// Stop the workload on source platform
    fn stop_source_workload(&self, biome: &str) -> impl Future<Output = Result<()>> + Send;
    /// Export workload state for transfer
    fn export_workload_state(
        &self,
        biome: &str,
    ) -> impl Future<Output = Result<WorkloadExport>> + Send;
    /// Transfer exported data to target
    fn transfer_workload_data(
        &self,
        export: &WorkloadExport,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Import export and start workload on target
    fn import_and_start_workload(
        &self,
        export: &WorkloadExport,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Start continuous replication from source to target
    fn start_continuous_replication(
        &self,
        source: &str,
        target: &str,
    ) -> impl Future<Output = Result<ReplicationHandle>> + Send;
    /// Wait for replication to sync
    fn wait_for_replication_sync(
        &self,
        handle: &ReplicationHandle,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Perform quick switchover from source to target
    fn perform_quick_switchover(
        &self,
        source: &str,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Stop replication
    fn stop_replication(
        &self,
        handle: &ReplicationHandle,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Create a snapshot of workload state
    fn create_workload_snapshot(
        &self,
        biome: &str,
    ) -> impl Future<Output = Result<WorkloadSnapshot>> + Send;
    /// Deploy snapshot to target platform
    fn deploy_snapshot_to_target(
        &self,
        snapshot: &WorkloadSnapshot,
        target: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    /// Start the cloned workload on target
    fn start_cloned_workload(&self, target: &str) -> impl Future<Output = Result<()>> + Send;
}

/// Implementation of migration operations
impl crate::universal::UniversalComputeManager {
    const DEFAULT_MIGRATION_ESTIMATE_SECS: u64 = 60;
}

impl MigrationOps for crate::universal::UniversalComputeManager {
    async fn create_migration_plan(&self, source: &str, target: &str) -> Result<MigrationPlan> {
        // Pending: real plan generation requires capability discovery for
        // source/target platform types, workload introspection, and risk analysis.
        // Gate behind `migration-preview` feature before production use.
        Ok(MigrationPlan {
            source_platform: source.to_string(),
            target_platform: target.to_string(),
            workload_id: "workload-1".to_string(),
            migration_type: MigrationType::ColdMigration,
            estimated_duration: Duration::from_secs(Self::DEFAULT_MIGRATION_ESTIMATE_SECS),
            risks: vec!["Data loss".to_string(), "Downtime".to_string()],
            requirements: vec!["Target platform availability".to_string()],
            cleanup_source: false,
        })
    }

    async fn execute_live_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🔄 Executing live migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        self.prepare_target_platform(&plan.target_platform).await?;
        let checkpoint = self
            .create_workload_checkpoint(&plan.source_platform)
            .await?;
        self.transfer_checkpoint(&checkpoint, &plan.target_platform)
            .await?;
        self.restore_from_checkpoint(&checkpoint, &plan.target_platform)
            .await?;

        if self.verify_migration_success(plan).await? {
            info!("✅ Live migration completed successfully");
            if plan.cleanup_source {
                self.cleanup_source_workload(&plan.source_platform).await?;
            }
        } else {
            return Err(crate::CliError::Other(
                "Migration verification failed".to_string(),
            ));
        }

        Ok(())
    }

    async fn execute_cold_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "❄️ Executing cold migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        self.stop_source_workload(&plan.source_platform).await?;
        let export_data = self.export_workload_state(&plan.source_platform).await?;
        self.transfer_workload_data(&export_data, &plan.target_platform)
            .await?;
        self.import_and_start_workload(&export_data, &plan.target_platform)
            .await?;

        info!("✅ Cold migration completed successfully");
        Ok(())
    }

    async fn execute_hot_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🔥 Executing hot migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        let replication_handle = self
            .start_continuous_replication(&plan.source_platform, &plan.target_platform)
            .await?;
        self.wait_for_replication_sync(&replication_handle).await?;
        self.perform_quick_switchover(&plan.source_platform, &plan.target_platform)
            .await?;
        self.stop_replication(&replication_handle).await?;

        info!("✅ Hot migration completed successfully");
        Ok(())
    }

    async fn execute_clone_migration(&self, plan: &MigrationPlan) -> Result<()> {
        info!(
            "🧬 Executing clone migration: {} → {}",
            plan.source_platform, plan.target_platform
        );

        let snapshot = self.create_workload_snapshot(&plan.source_platform).await?;
        self.deploy_snapshot_to_target(&snapshot, &plan.target_platform)
            .await?;
        self.start_cloned_workload(&plan.target_platform).await?;

        info!("✅ Clone migration completed successfully");
        Ok(())
    }

    async fn pause_workload(&self, platform: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "pause_workload({platform}): requires platform.workload.pause capability"
        )))
    }

    async fn verify_migration_success(&self, plan: &MigrationPlan) -> Result<bool> {
        Err(crate::CliError::NotImplemented(format!(
            "migration verification for {} → {}: requires platform.health.check capability",
            plan.source_platform, plan.target_platform
        )))
    }

    async fn prepare_target_platform(&self, target: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "prepare_target_platform({target}): requires platform.provision capability"
        )))
    }

    async fn create_workload_checkpoint(&self, biome: &str) -> Result<WorkloadCheckpoint> {
        info!("📸 Creating workload checkpoint: {}", biome);
        Ok(WorkloadCheckpoint {
            biome_name: biome.to_string(),
            timestamp: std::time::SystemTime::now(),
            // Use runtime-discovered temp directory (Deep Debt compliant)
            data_path: {
                let mut path = super::constants::paths::checkpoint_prefix();
                path.set_file_name(format!("toadstool_checkpoint_{}", uuid::Uuid::new_v4()));
                path
            },
        })
    }

    async fn transfer_checkpoint(
        &self,
        _checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "transfer_checkpoint({target}): requires platform.transfer capability"
        )))
    }

    async fn restore_from_checkpoint(
        &self,
        _checkpoint: &WorkloadCheckpoint,
        target: &str,
    ) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "restore_from_checkpoint({target}): requires platform.restore capability"
        )))
    }

    async fn cleanup_source_workload(&self, biome: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "cleanup_source_workload({biome}): requires platform.workload.cleanup capability"
        )))
    }

    async fn stop_source_workload(&self, biome: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "stop_source_workload({biome}): requires platform.workload.stop capability"
        )))
    }

    async fn export_workload_state(&self, biome: &str) -> Result<WorkloadExport> {
        info!("📤 Exporting workload state: {}", biome);
        Ok(WorkloadExport {
            biome_name: biome.to_string(),
            // Use runtime-discovered temp directory (Deep Debt compliant)
            export_path: {
                let mut path = super::constants::paths::export_prefix();
                path.set_file_name(format!("toadstool_export_{}", uuid::Uuid::new_v4()));
                path
            },
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn transfer_workload_data(&self, _export: &WorkloadExport, target: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "transfer_workload_data({target}): requires platform.transfer capability"
        )))
    }

    async fn import_and_start_workload(
        &self,
        _export: &WorkloadExport,
        target: &str,
    ) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "import_and_start_workload({target}): requires platform.workload.import capability"
        )))
    }

    async fn start_continuous_replication(
        &self,
        source: &str,
        target: &str,
    ) -> Result<ReplicationHandle> {
        info!(
            "🔄 Starting continuous replication: {} → {}",
            source, target
        );
        Ok(ReplicationHandle {
            id: uuid::Uuid::new_v4(),
            source: source.to_string(),
            target: target.to_string(),
        })
    }

    async fn wait_for_replication_sync(&self, handle: &ReplicationHandle) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "wait_for_replication_sync({}): requires replication.sync capability",
            handle.id
        )))
    }

    async fn perform_quick_switchover(&self, source: &str, target: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "perform_quick_switchover({source} -> {target}): requires platform.switchover capability"
        )))
    }

    async fn stop_replication(&self, handle: &ReplicationHandle) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "stop_replication({}): requires replication.control capability",
            handle.id
        )))
    }

    async fn create_workload_snapshot(&self, biome: &str) -> Result<WorkloadSnapshot> {
        info!("📷 Creating workload snapshot: {}", biome);
        Ok(WorkloadSnapshot {
            biome_name: biome.to_string(),
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            created_at: std::time::SystemTime::now(),
        })
    }

    async fn deploy_snapshot_to_target(
        &self,
        _snapshot: &WorkloadSnapshot,
        target: &str,
    ) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "deploy_snapshot_to_target({target}): requires platform.snapshot.deploy capability"
        )))
    }

    async fn start_cloned_workload(&self, target: &str) -> Result<()> {
        Err(crate::CliError::NotImplemented(format!(
            "start_cloned_workload({target}): requires platform.workload.start capability"
        )))
    }
}
