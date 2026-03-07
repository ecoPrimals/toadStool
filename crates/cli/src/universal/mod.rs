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

    #[tokio::test]
    async fn test_detect_platforms_empty_categories_uses_defaults() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager.detect_platforms(vec![], false, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_with_test_platforms() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["traditional".to_string()], true, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_container_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["container".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_language_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["language".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_gpu_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["gpu".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_quantum_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["quantum".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_edge_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["edge".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_biological_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["biological".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_detect_platforms_neuromorphic_category() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .detect_platforms(vec!["neuromorphic".to_string()], false, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_benchmarks_json_format() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .run_benchmarks("basic".to_string(), vec![], "json".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_benchmarks_with_target_platforms() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        // Use specific platform IDs - may not exist but exercises the code path
        let result = manager
            .run_benchmarks(
                "basic".to_string(),
                vec![
                    "traditional_linux".to_string(),
                    "traditional_unknown".to_string(),
                ],
                "table".to_string(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_benchmarks_unknown_format_defaults_to_table() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let result = manager
            .run_benchmarks("basic".to_string(), vec![], "unknown".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_migrate_workload_target_not_found() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        let platforms: Vec<String> = manager.platforms.keys().cloned().collect();
        let source = platforms
            .first()
            .cloned()
            .unwrap_or_else(|| "traditional_linux".to_string());
        let result = manager
            .migrate_workload(source, "nonexistent-target-xyz".to_string(), false, false)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Target platform"));
    }

    #[tokio::test]
    async fn test_migrate_workload_success_cold_migration() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        let platforms: Vec<String> = manager.platforms.keys().cloned().collect();
        if platforms.len() >= 2 {
            let result = manager
                .migrate_workload(platforms[0].clone(), platforms[1].clone(), false, false)
                .await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_migrate_workload_with_pause_source() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        let platforms: Vec<String> = manager.platforms.keys().cloned().collect();
        if platforms.len() >= 2 {
            let result = manager
                .migrate_workload(platforms[0].clone(), platforms[1].clone(), true, false)
                .await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_migrate_workload_with_verify() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        let platforms: Vec<String> = manager.platforms.keys().cloned().collect();
        if platforms.len() >= 2 {
            let result = manager
                .migrate_workload(platforms[0].clone(), platforms[1].clone(), false, true)
                .await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_show_capabilities_default_format() {
        let manager = UniversalComputeManager::new().await.unwrap();
        let result = manager.show_capabilities("unknown_format", false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_capabilities_with_platforms() {
        let mut manager = UniversalComputeManager::new().await.unwrap();
        let _ = manager
            .detect_platforms(vec!["traditional".to_string()], false, None)
            .await;
        let result = manager.show_capabilities("json", true).await;
        assert!(result.is_ok());
    }
}
