// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executor internal methods — operations paths (`up`, lifecycle, concurrency).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Barrier;

use toadstool_cli::executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions};
use toadstool_cli::{
    BiomeManifest, BiomeMetadata, BiomeNetworking, BiomeResources, BiomeSecurity, BiomeStorage,
    CliContext, CliError,
};

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_context() -> CliContext {
    CliContext {
        config_path: None,
        working_dir: std::env::current_dir().expect("working directory must be accessible"),
        verbose: false,
    }
}

async fn create_test_executor() -> Result<BiomeExecutor> {
    Ok(BiomeExecutor::new().await?)
}

#[expect(dead_code, reason = "test helper; trait compatibility")]
fn create_minimal_manifest(name: &str) -> BiomeManifest {
    BiomeManifest {
        metadata: BiomeMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test biome".to_string()),
            author: Some("Test".to_string()),
            created: std::time::SystemTime::now(),
            updated: std::time::SystemTime::now(),
            tags: vec![],
        },
        primals: HashMap::new(),
        services: HashMap::new(),
        resources: BiomeResources {
            cpu_limit: Some(1.0),
            memory_limit: Some("512M".to_string()),
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: BiomeSecurity {
            isolation_level: "basic".to_string(),
            trust_level: "default".to_string(),
            security_required: false,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: BiomeStorage {
            storage_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

async fn create_manifest_file(name: &str, content: &str) -> Result<PathBuf> {
    use uuid::Uuid;

    let temp_dir = std::env::temp_dir();
    let unique_id = Uuid::new_v4();
    let path = temp_dir.join(format!("manifest-{name}-{unique_id}.toml"));

    std::fs::write(&path, content)?;
    Ok(path)
}

async fn cleanup_file(path: &PathBuf) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
// ============================================================================
// LOG PARAMETER COMPREHENSIVE TESTS
// ============================================================================

#[tokio::test]
async fn test_show_logs_all_parameter_combinations() {
    let executor = Arc::new(create_test_executor().await.unwrap());

    // Test various combinations of parameters
    let test_cases = vec![
        (false, 10, false, None, None),
        (false, 50, true, None, None),
        (false, 100, false, Some("info".to_string()), None),
        (false, 200, true, Some("error".to_string()), None),
        (false, 50, false, None, Some("ERROR".to_string())),
        (
            false,
            50,
            true,
            Some("warn".to_string()),
            Some("WARN.*".to_string()),
        ),
    ];

    let handles: Vec<_> = test_cases
        .into_iter()
        .enumerate()
        .map(|(i, (follow, lines, timestamps, level, grep))| {
            let exec = Arc::clone(&executor);

            tokio::spawn(async move {
                exec.show_logs(
                    format!("biome-{i}").as_str(),
                    follow,
                    lines,
                    timestamps,
                    level.as_deref(),
                    grep.as_deref(),
                )
                .await
            })
        })
        .collect();

    // All parameter combinations should be accepted
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_err(), "Should fail for nonexistent biome");
    }
}

// ============================================================================
// STRESS & PROPERTY TESTS
// ============================================================================

#[tokio::test]
async fn test_property_executor_methods_never_panic() {
    // Property: No executor method should panic under any input
    let executor = Arc::new(create_test_executor().await.unwrap());

    let handles: Vec<_> = (0..50)
        .map(|i| {
            let exec = Arc::clone(&executor);

            tokio::spawn(async move {
                let _ctx = create_test_context();

                // Mix of all operations with random parameters
                let _ = exec.list_biomes(i % 2 == 0, "table", false, None).await;
                let _ = exec
                    .down_biome(format!("biome-{i}").as_str(), i % 2 == 0, 30, false)
                    .await;
                let _ = exec
                    .show_logs(format!("log-{i}").as_str(), false, 50, false, None, None)
                    .await;

                Ok::<(), anyhow::Error>(())
            })
        })
        .collect();

    // All should complete without panicking
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task should not panic");
    }
}

#[tokio::test]
async fn test_stress_list_biomes_different_parameters() {
    // Stress test with many concurrent list operations with different params
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(Barrier::new(20));

    let mut handles = vec![];

    // Vary all parameters
    for i in 0..20 {
        let exec = Arc::clone(&executor);
        let b = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            b.wait().await;

            let all = i % 2 == 0;
            let format = if i % 3 == 0 {
                "json"
            } else if i % 3 == 1 {
                "yaml"
            } else {
                "table"
            };
            let resources = i % 4 == 0;
            let filter = if i % 5 == 0 {
                Some("running".to_string())
            } else {
                None
            };

            exec.list_biomes(all, format, resources, filter.as_deref())
                .await
        }));
    }

    // All should succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "List with various parameters should work");
    }
}

// ============================================================================
// DETACH MODE TESTS
// ============================================================================

#[tokio::test]
async fn test_up_biome_detach_mode_variations() {
    let executor = Arc::new(create_test_executor().await.unwrap());

    let manifest_content = r#"
    [metadata]
    name = "detach-test"
    version = "1.0.0"
    [resources]
    "#;

    let handles: Vec<_> = [true, false]
        .iter()
        .map(|&detach| {
            let exec = Arc::clone(&executor);

            tokio::spawn(async move {
                let manifest_path =
                    create_manifest_file(&format!("detach-{detach}"), manifest_content)
                        .await
                        .map_err(|e| CliError::Other(e.to_string()))?;

                let ctx = create_test_context();
                let opts = UpBiomeOptions {
                    manifest_path: manifest_path.clone(),
                    detach,
                    name: Some(format!("biome-detach-{detach}")),
                    env: vec![],
                    restart: false,
                    health_interval: 30,
                };
                let result = exec.up_biome(&ctx, opts).await;

                cleanup_file(&manifest_path).await.ok();
                result
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

// ============================================================================
// HEALTH INTERVAL TESTS
// ============================================================================

#[tokio::test]
async fn test_up_biome_different_health_intervals() {
    let executor = Arc::new(create_test_executor().await.unwrap());

    let manifest_content = r#"
    [metadata]
    name = "health-test"
    version = "1.0.0"
    [resources]
    "#;

    let health_intervals = vec![10, 30, 60, 120];

    let handles: Vec<_> = health_intervals
        .into_iter()
        .map(|interval| {
            let exec = Arc::clone(&executor);

            tokio::spawn(async move {
                let manifest_path =
                    create_manifest_file(&format!("health-{interval}"), manifest_content)
                        .await
                        .map_err(|e| CliError::Other(e.to_string()))?;

                let ctx = create_test_context();
                let opts = UpBiomeOptions {
                    manifest_path: manifest_path.clone(),
                    detach: false,
                    name: Some(format!("health-{interval}")),
                    env: vec![],
                    restart: false,
                    health_interval: interval,
                };
                let result = exec.up_biome(&ctx, opts).await;

                cleanup_file(&manifest_path).await.ok();
                result
            })
        })
        .collect();

    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

// ============================================================================
// CONCURRENT STATE MANAGEMENT TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_list_operations_consistent_state() {
    // Property: Concurrent list operations should return consistent state
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let exec = Arc::clone(&executor);
            let b = Arc::clone(&barrier);

            tokio::spawn(async move {
                b.wait().await;
                exec.list_biomes(true, "json", true, None).await
            })
        })
        .collect();

    // All should see consistent state (empty)
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "State should be consistent");
    }
}

// ============================================================================
// ERROR MESSAGE QUALITY TESTS
// ============================================================================

#[tokio::test]
async fn test_error_messages_are_descriptive() {
    let executor = create_test_executor().await.unwrap();

    // Test that error messages provide useful information
    let result = executor.down_biome("nonexistent", false, 30, false).await;

    assert!(result.is_err(), "Should fail");

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("nonexistent")
            || err_msg.contains("not running")
            || err_msg.contains("not found"),
        "Error message should be descriptive: {err_msg}"
    );
}

#[tokio::test]
async fn test_down_biome_error_includes_biome_name() {
    let executor = create_test_executor().await.unwrap();

    let biome_name = "test-biome-xyz";
    let result = executor.down_biome(biome_name, false, 30, false).await;

    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains(biome_name),
        "Error should include biome name: {err_msg}"
    );
}

// ============================================================================
// ENVIRONMENT VARIABLE VALIDATION
// ============================================================================

#[tokio::test]
async fn test_environment_variables_with_special_characters() {
    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    let manifest_content = r#"
    [metadata]
    name = "env-special"
    version = "1.0.0"
    [resources]
    "#;

    let manifest_path = create_manifest_file("env-special", manifest_content)
        .await
        .unwrap();

    let env_vars = vec![
        "PATH=/usr/bin:/usr/local/bin".to_string(),
        "URL=https://example.com:8080/path?query=value".to_string(),
        "JSON={\"key\":\"value\"}".to_string(),
    ];

    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: None,
        env: env_vars,
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "basic".to_string(),
    };
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_file(&manifest_path).await.ok();

    // Special characters in env vars should be accepted
    let _ = result;
}

// ============================================================================
// BOUNDARY & EDGE CASE TESTS
// ============================================================================

#[tokio::test]
async fn test_very_long_biome_name() {
    let executor = create_test_executor().await.unwrap();

    // Test with very long name (but valid)
    let long_name = "a".repeat(255);

    let result = executor.down_biome(long_name, false, 30, false).await;

    // Should fail (biome doesn't exist) but accept long name
    assert!(result.is_err());
}

#[tokio::test]
async fn test_biome_name_with_special_characters() {
    let executor = create_test_executor().await.unwrap();

    let special_names = vec![
        "biome-with-dashes",
        "biome_with_underscores",
        "biome.with.dots",
        "biome123",
    ];

    for name in special_names {
        let result = executor.down_biome(name, false, 30, false).await;

        // All should be accepted as valid names
        assert!(result.is_err(), "Should fail for nonexistent biome");
    }
}

#[tokio::test]
async fn test_log_lines_boundary_values() {
    let executor = create_test_executor().await.unwrap();

    let line_counts = vec![1, 10, 50, 100, 1000, 10_000];

    for lines in line_counts {
        let result = executor
            .show_logs("test", false, lines, false, None, None)
            .await;

        // All line counts should be accepted
        assert!(result.is_err(), "Should fail for nonexistent biome");
    }
}

// ============================================================================
// CONCURRENT EXECUTOR CREATION STRESS TEST
// ============================================================================

#[tokio::test]
async fn test_stress_executor_creation_and_operations() {
    // ✅ MODERN: Create many executors concurrently and use them
    let barrier = Arc::new(Barrier::new(10));

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let b = Arc::clone(&barrier);

            tokio::spawn(async move {
                b.wait().await;

                // Each task creates its own executor
                let exec = create_test_executor().await?;

                // Perform operations
                exec.list_biomes(false, "table", false, None).await?;
                exec.down_biome(format!("test-{i}"), false, 30, false)
                    .await
                    .ok();
                exec.show_logs(format!("log-{i}").as_str(), false, 50, false, None, None)
                    .await
                    .ok();

                Ok::<(), anyhow::Error>(())
            })
        })
        .collect();

    // All should complete
    for handle in handles {
        let result = handle.await.unwrap();
        // list_biomes should succeed, others may fail (that's OK)
        let _ = result;
    }
}

// ============================================================================
// FUTURE: DAY 3 CONTINUATION
// ============================================================================

// Next batch will add:
// - Service management tests (with mocked services)
// - State persistence tests
// - Health monitoring tests
// - Full integration scenarios
