// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Compute Operations - Advanced Substrate Management
//!
//! Advanced operations for universal compute platform management:
//! - Substrate detection and testing
//! - Performance benchmarking
//! - Workload migration between platforms
//! - Federation with other `ToadStool` instances
//!
//! ## Module Structure
//!
//! - `types`: All type definitions for platforms, benchmarks, federation, and migration
//! - `manager`: Core manager implementation with all operations

// Type definitions
pub mod types;

// Operation traits
pub mod operations;

// Re-export all types for backward compatibility
pub use types::{
    BenchmarkResult, BenchmarkTest, BenchmarkType, DetectedPlatform, FederationPeer,
    FederationStatus, GpuInfo, HardwareInfo, MigrationPlan, MigrationType, PlatformStatus,
    ReplicationHandle, SystemInfo, TrustLevel, WorkloadCheckpoint, WorkloadExport,
    WorkloadSnapshot,
};

// Core imports for manager
use crate::{CliContextExt, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tracing::{error, info, warn};

use toadstool_distributed::substrate_detection::{SubstrateCapabilities, SubstrateDetector};

/// Universal compute operations manager
pub struct UniversalComputeManager {
    /// Substrate detector
    detector: SubstrateDetector,
    /// Detected platforms
    platforms: HashMap<String, DetectedPlatform>,
    /// Benchmark results
    benchmarks: HashMap<String, BenchmarkResult>,
    /// Federation connections
    federation_peers: HashMap<String, FederationPeer>,
}

// Include the implementation from manager_impl.rs
// This keeps the file size manageable while preserving all functionality
include!("manager_impl.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_universal_compute_manager_new() {
        let manager = UniversalComputeManager::new().await;
        assert!(manager.is_ok());
        let manager = manager.unwrap();
        assert!(manager.platforms.is_empty());
        assert!(manager.benchmarks.is_empty());
        assert!(manager.federation_peers.is_empty());
    }

    #[tokio::test]
    async fn test_detect_platforms_with_output_file() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let temp_dir = TempDir::new().expect("temp dir");
        let output_path = temp_dir.path().join("detection.json");

        let result = manager
            .detect_platforms(
                vec!["traditional".to_string()],
                false,
                Some(output_path.clone()),
            )
            .await;

        assert!(result.is_ok());
        if output_path.exists() {
            let content = tokio::fs::read_to_string(&output_path).await.unwrap();
            assert!(content.contains("platforms") || content.contains("timestamp"));
        }
    }

    #[tokio::test]
    async fn test_detect_platforms_unknown_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["unknown_category_xyz".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_benchmarks_empty_platforms() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .run_benchmarks("basic".to_string(), vec![], "table".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_migrate_workload_source_not_found() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .migrate_workload(
                "nonexistent-source".to_string(),
                "nonexistent-target".to_string(),
                false,
                false,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Source platform"));
    }

    #[tokio::test]
    async fn test_establish_federation_invalid_endpoint() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .establish_federation(
                "not-valid-address".to_string(),
                "standard".to_string(),
                vec![],
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }

    #[tokio::test]
    async fn test_show_capabilities_empty() {
        let manager = UniversalComputeManager::new().await.unwrap();
        let result = manager.show_capabilities("json", false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_capabilities_table() {
        let manager = UniversalComputeManager::new().await.unwrap();
        let result = manager.show_capabilities("table", true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_capabilities_yaml() {
        let manager = UniversalComputeManager::new().await.unwrap();
        let result = manager.show_capabilities("yaml", false).await;
        assert!(result.is_ok());
    }
}
