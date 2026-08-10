// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload migration types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Plan for migrating a workload between platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Source platform (e.g. docker, native)
    pub source_platform: String,
    /// Target platform
    pub target_platform: String,
    /// Workload identifier
    pub workload_id: String,
    /// Migration strategy (live, cold, hot, clone)
    pub migration_type: MigrationType,
    /// Estimated migration duration
    pub estimated_duration: Duration,
    /// Identified risks
    pub risks: Vec<String>,
    /// Prerequisites for migration
    pub requirements: Vec<String>,
    /// Whether to remove workload from source after migration
    pub cleanup_source: bool,
}

/// Type of migration strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    /// No downtime (live migration)
    LiveMigration,
    /// Planned downtime (stop, move, start)
    ColdMigration,
    /// Minimal downtime (replication-based)
    HotMigration,
    /// Create copy then switch
    CloneMigration,
}

/// Workload checkpoint for migration
#[derive(Debug, Clone)]
pub struct WorkloadCheckpoint {
    /// Biome name
    pub biome_name: String,
    /// When the checkpoint was created
    pub timestamp: std::time::SystemTime,
    /// Path to checkpoint data
    pub data_path: PathBuf,
}

/// Exported workload data
#[derive(Debug, Clone)]
pub struct WorkloadExport {
    /// Biome name
    pub biome_name: String,
    /// Path to exported data
    pub export_path: PathBuf,
    /// Export metadata (version, checksum, etc.)
    pub metadata: std::collections::HashMap<String, String>,
}

/// Handle for replication process
#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    /// Replication job ID
    pub id: uuid::Uuid,
    /// Source platform or endpoint
    pub source: String,
    /// Target platform or endpoint
    pub target: String,
}

/// Snapshot of workload state
#[derive(Debug, Clone)]
pub struct WorkloadSnapshot {
    /// Biome name
    pub biome_name: String,
    /// Snapshot identifier
    pub snapshot_id: String,
    /// When the snapshot was created
    pub created_at: std::time::SystemTime,
}
