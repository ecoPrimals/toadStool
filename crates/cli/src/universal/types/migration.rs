//! Workload migration types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::time::Duration;

/// Plan for migrating a workload between platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub source_platform: String,
    pub target_platform: String,
    pub workload_id: String,
    pub migration_type: MigrationType,
    pub estimated_duration: Duration,
    pub risks: Vec<String>,
    pub requirements: Vec<String>,
    pub cleanup_source: bool,
}

/// Type of migration strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    LiveMigration,  // No downtime
    ColdMigration,  // Planned downtime
    HotMigration,   // Minimal downtime
    CloneMigration, // Create copy then switch
}

/// Workload checkpoint for migration
#[derive(Debug, Clone)]
pub struct WorkloadCheckpoint {
    pub biome_name: String,
    pub timestamp: std::time::SystemTime,
    pub data_path: PathBuf,
}

/// Exported workload data
#[derive(Debug, Clone)]
pub struct WorkloadExport {
    pub biome_name: String,
    pub export_path: PathBuf,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Handle for replication process
#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    pub id: uuid::Uuid,
    pub source: String,
    pub target: String,
}

/// Snapshot of workload state
#[derive(Debug, Clone)]
pub struct WorkloadSnapshot {
    pub biome_name: String,
    pub snapshot_id: String,
    pub created_at: std::time::SystemTime,
}
