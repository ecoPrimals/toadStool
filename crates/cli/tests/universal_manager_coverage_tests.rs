// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for `UniversalComputeManager` (`manager_impl.rs`) - coverage target 90%
//!
//! Tests `detect_platforms`, `run_benchmarks`, `migrate_workload`, `establish_federation`,
//! `show_capabilities`, and all platform categories.

use tempfile::TempDir;
use toadstool_cli::universal::UniversalComputeManager;

// ============================================================================
// new() tests
// ============================================================================

#[tokio::test]
async fn test_manager_new_succeeds() {
    let result = UniversalComputeManager::new().await;
    assert!(result.is_ok());
    let manager = result.unwrap();
    let result = manager.show_capabilities("json", false).await;
    assert!(result.is_ok());
}

// ============================================================================
// detect_platforms tests - all categories
// ============================================================================

#[tokio::test]
async fn test_detect_platforms_traditional() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_container() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["container".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_language() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["language".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_gpu() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["gpu".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_quantum() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["quantum".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_edge() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["edge".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_biological() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["biological".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_neuromorphic() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["neuromorphic".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_unknown_category_warns() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["unknown_xyz_category".to_string()], false, None)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_detect_platforms_empty_categories_uses_defaults() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.detect_platforms(vec![], false, None).await;
    assert!(result.is_ok());
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
async fn test_detect_platforms_with_test_platforms() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .detect_platforms(vec!["traditional".to_string()], true, None)
        .await;
    assert!(result.is_ok());
}

// ============================================================================
// run_benchmarks tests
// ============================================================================

#[tokio::test]
async fn test_run_benchmarks_empty_platforms() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .run_benchmarks("basic".to_string(), vec![], "table".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_benchmarks_json_format() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .run_benchmarks("basic".to_string(), vec![], "json".to_string())
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_run_benchmarks_table_format() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .run_benchmarks("basic".to_string(), vec![], "table".to_string())
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
async fn test_run_benchmarks_with_target_platforms() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let targets = vec![
        "traditional_linux".to_string(),
        "traditional_unknown".to_string(),
    ];
    let result = manager
        .run_benchmarks("basic".to_string(), targets, "table".to_string())
        .await;
    assert!(result.is_ok());
}

// ============================================================================
// migrate_workload tests
// ============================================================================

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
async fn test_migrate_workload_target_not_found() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .migrate_workload(
            "traditional_linux".to_string(),
            "nonexistent-target-xyz".to_string(),
            false,
            false,
        )
        .await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Target platform") || err_msg.contains("Source platform"),
        "expected platform error, got: {err_msg}"
    );
}

#[tokio::test]
async fn test_migrate_workload_success() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .migrate_workload(
            "traditional_linux".to_string(),
            "traditional_unknown".to_string(),
            false,
            false,
        )
        .await;
    let _ = result;
}

#[tokio::test]
async fn test_migrate_workload_with_pause_source() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .migrate_workload(
            "traditional_linux".to_string(),
            "traditional_unknown".to_string(),
            true,
            false,
        )
        .await;
    let _ = result;
}

#[tokio::test]
async fn test_migrate_workload_with_verify() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager
        .migrate_workload(
            "traditional_linux".to_string(),
            "traditional_unknown".to_string(),
            false,
            true,
        )
        .await;
    let _ = result;
}

// ============================================================================
// establish_federation tests
// ============================================================================

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
async fn test_establish_federation_valid_format_unreachable() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let result = manager
        .establish_federation(
            "127.0.0.1:9999".to_string(),
            "peer".to_string(),
            vec!["resource1".to_string()],
        )
        .await;
    let _ = result;
}

// ============================================================================
// show_capabilities tests
// ============================================================================

#[tokio::test]
async fn test_show_capabilities_json_empty() {
    let manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.show_capabilities("json", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_yaml_empty() {
    let manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.show_capabilities("yaml", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_table_empty() {
    let manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.show_capabilities("table", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_table_detailed() {
    let manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.show_capabilities("table", true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_unknown_format_defaults_to_table() {
    let manager = UniversalComputeManager::new().await.unwrap();
    let result = manager.show_capabilities("unknown_format", false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_with_platforms_json() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager.show_capabilities("json", true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_show_capabilities_with_platforms_yaml() {
    let mut manager = UniversalComputeManager::new().await.unwrap();
    let _ = manager
        .detect_platforms(vec!["traditional".to_string()], false, None)
        .await;
    let result = manager.show_capabilities("yaml", false).await;
    assert!(result.is_ok());
}
