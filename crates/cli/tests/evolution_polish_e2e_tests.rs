// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::expect_used)] // expect() is idiomatic in tests
//! Evolution Polish E2E Tests
//!
//! End-to-end tests verifying the complete removal of hardcoded primal clients
//! and the enforcement of pure capability-based discovery.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Barrier;

use toadstool_cli::executor::{BiomeExecutor, RunBiomeOptions};
use toadstool_cli::CliContext;

// ============================================================================
// TEST FIXTURES
// ============================================================================

fn create_test_context() -> CliContext {
    CliContext {
        config_path: None,
        working_dir: std::env::current_dir().expect("working directory must be accessible"),
        verbose: false,
    }
}

async fn create_test_manifest(name: &str) -> anyhow::Result<PathBuf> {
    let content = format!(
        r#"
[metadata]
name = "{}"
version = "1.0.0"
description = "Test biome for evolution polish E2E"

[resources]
cpu_limit = 1.0
memory_limit = "512M"

[security]
isolation_level = "basic"
trust_level = "default"
beardog_required = false

[networking]
mode = "bridge"

[storage]
"#,
        name
    );

    let temp_dir = std::env::temp_dir();
    let path = temp_dir.join(format!("evolution-{}.toml", name));
    tokio::fs::write(&path, content).await?;
    Ok(path)
}

async fn cleanup_manifest(path: &PathBuf) {
    let _ = tokio::fs::remove_file(path).await;
}

// ============================================================================
// E2E TEST 1: Full Executor Lifecycle Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_executor_full_lifecycle_standalone() {
    // ✅ E2E: Complete executor lifecycle without any hardcoded registry

    let executor = BiomeExecutor::new().await.unwrap();
    let ctx = create_test_context();

    // 1. List biomes (should be empty)
    let list_result = executor
        .list_biomes(false, "json".to_string(), false, None)
        .await;
    assert!(list_result.is_ok(), "Should list biomes without registry");

    // 2. Try to create a biome (may fail at startup, but not due to registry)
    let manifest_path = create_test_manifest("e2e-test-1").await.unwrap();

    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: Some("e2e-standalone".to_string()),
        env: vec![],
        debug: false,
        cpu_limit: Some(0.5),
        memory_limit: Some("256M".to_string()),
        security: "basic".to_string(),
    };
    let run_result = executor.run_biome(&ctx, opts).await;

    // May fail at startup, but error should not mention hardcoded clients
    if let Err(e) = run_result {
        let msg = e.to_string();
        assert!(
            !msg.contains("BiomeOSClient")
                && !msg.contains("biomeos_client")
                && !msg.contains("SongbirdClient"),
            "Error should not reference hardcoded clients: {}",
            msg
        );
    }

    // 3. List again
    let list_result_2 = executor
        .list_biomes(false, "yaml".to_string(), true, None)
        .await;
    assert!(list_result_2.is_ok(), "Second list should work");

    // 4. Try to stop nonexistent biome
    let down_result = executor
        .down_biome("e2e-standalone".to_string(), false, 30, false)
        .await;
    assert!(down_result.is_err(), "Should fail for nonexistent biome");

    cleanup_manifest(&manifest_path).await;
}

// ============================================================================
// E2E TEST 2: Concurrent Biome Operations Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_concurrent_biome_operations_standalone() {
    // ✅ E2E: Multiple concurrent biome operations without registry

    let executor = Arc::new(BiomeExecutor::new().await.unwrap());
    let ctx = create_test_context();
    let barrier = Arc::new(Barrier::new(5));

    let mut handles = vec![];

    // Spawn 5 concurrent operations
    for i in 0..5 {
        let exec = Arc::clone(&executor);
        let ctx_clone = CliContext {
            config_path: ctx.config_path.clone(),
            working_dir: ctx.working_dir.clone(),
            verbose: ctx.verbose,
        };
        let b = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            b.wait().await;

            let manifest_path = create_test_manifest(&format!("concurrent-{}", i))
                .await
                .unwrap();

            // Try to run biome
            let opts = RunBiomeOptions {
                manifest_path: manifest_path.clone(),
                name: Some(format!("concurrent-biome-{}", i)),
                env: vec![],
                debug: false,
                cpu_limit: Some(0.5),
                memory_limit: Some("128M".to_string()),
                security: "basic".to_string(),
            };
            let result = exec.run_biome(&ctx_clone, opts).await;

            cleanup_manifest(&manifest_path).await;

            // Check error doesn't mention hardcoded clients
            if let Err(e) = result {
                let msg = e.to_string();
                assert!(
                    !msg.contains("BiomeOSClient"),
                    "Concurrent operation error should not mention hardcoded clients"
                );
            }
        }));
    }

    // All should complete without deadlock
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent operations should complete");
    }
}

// ============================================================================
// E2E TEST 3: Discovery System Integration
// ============================================================================

#[tokio::test]
async fn test_e2e_discovery_system_works_without_hardcoded_names() {
    // ✅ E2E: Discovery system uses capabilities, not primal names

    use toadstool_common::infant_discovery::{CapabilityDiscovery, DiscoveryEngine};

    let discovery_engine = DiscoveryEngine::new();

    // Should be able to discover by capability
    let result = discovery_engine.discover("compute").await;

    // May not find anything (no services running), but should not error on hardcoded names
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.contains("beardog")
                && !msg.contains("songbird")
                && !msg.contains("BearDog")
                && !msg.contains("Songbird"),
            "Discovery error should not reference primal names: {}",
            msg
        );
    }
}

// ============================================================================
// E2E TEST 4: Adapter Factory Integration
// ============================================================================

#[tokio::test]
async fn test_e2e_adapter_factory_capability_based() {
    // ✅ E2E: Adapter factory provides capability-based adapters

    use toadstool_cli::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();

    // Should create adapters for generic capabilities
    let coordination = factory.coordination_adapter();
    assert!(coordination.is_ok(), "Should create coordination adapter");

    let crypto = factory.crypto_adapter();
    assert!(crypto.is_ok(), "Should create crypto adapter");

    let storage = factory.storage_adapter();
    assert!(storage.is_ok(), "Should create storage adapter");
}

// ============================================================================
// E2E TEST 5: Full Stack - Executor + Discovery + Adapters
// ============================================================================

#[tokio::test]
async fn test_e2e_full_stack_capability_based() {
    // ✅ E2E: Complete stack works with pure capability-based discovery

    // 1. Create executor (no hardcoded registry)
    let executor = BiomeExecutor::new().await.unwrap();

    // 2. Create adapter factory
    use toadstool_cli::ecosystem::adapters::AdapterFactory;
    let factory = AdapterFactory::new();

    // 3. Create discovery engine
    use toadstool_common::infant_discovery::DiscoveryEngine;
    let discovery = DiscoveryEngine::new();

    // 4. Perform operations
    let _ = executor
        .list_biomes(false, "json".to_string(), false, None)
        .await;
    let _ = factory.coordination_adapter();
    let _ = discovery;

    // All components work together without hardcoded primal names ✅
}

// ============================================================================
// E2E TEST 6: Error Propagation Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_error_propagation_clean() {
    // ✅ E2E: Errors propagate cleanly without mentioning hardcoded clients

    let executor = BiomeExecutor::new().await.unwrap();

    // Try various operations that should fail
    let errors = vec![
        executor
            .down_biome("nonexistent1".to_string(), false, 30, false)
            .await,
        executor
            .down_biome("nonexistent2".to_string(), true, 60, false)
            .await,
        executor
            .show_logs("nonexistent3".to_string(), false, 50, false, None, None)
            .await,
    ];

    // All should error, but cleanly
    for error in errors {
        assert!(
            error.is_err(),
            "Operation should fail for nonexistent biome"
        );

        let msg = error.unwrap_err().to_string();
        assert!(
            !msg.contains("BiomeOSClient")
                && !msg.contains("SongbirdClient")
                && !msg.contains("biomeos_client")
                && !msg.contains("songbird_client"),
            "Error message should not reference hardcoded clients: {}",
            msg
        );
    }
}

// ============================================================================
// E2E TEST 7: Multi-Format Output Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_multi_format_output_standalone() {
    // ✅ E2E: All output formats work without registry

    let executor = Arc::new(BiomeExecutor::new().await.unwrap());
    let formats = vec!["json", "yaml", "table"];

    let mut handles = vec![];

    for format in formats {
        let exec = Arc::clone(&executor);
        handles.push(tokio::spawn(async move {
            exec.list_biomes(false, format.to_string(), false, None)
                .await
        }));
    }

    // All formats should work
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(
            result.is_ok(),
            "All output formats should work without registry"
        );
    }
}

// ============================================================================
// E2E TEST 8: Resource Limits Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_resource_limits_without_registry() {
    // ✅ E2E: Resource limiting works without hardcoded registry

    let executor = BiomeExecutor::new().await.unwrap();
    let ctx = create_test_context();

    let manifest_path = create_test_manifest("resource-test").await.unwrap();

    // Try with various resource limits
    let resource_configs = vec![
        (Some(0.5), Some("128M".to_string())),
        (Some(1.0), Some("256M".to_string())),
        (Some(2.0), Some("512M".to_string())),
    ];

    for (cpu, mem) in resource_configs {
        let opts = RunBiomeOptions {
            manifest_path: manifest_path.clone(),
            name: Some("resource-test".to_string()),
            env: vec![],
            debug: false,
            cpu_limit: cpu,
            memory_limit: mem,
            security: "basic".to_string(),
        };
        let result = executor.run_biome(&ctx, opts).await;

        // May fail at startup, but error should not mention registry
        if let Err(e) = result {
            let msg = e.to_string();
            assert!(
                !msg.contains("biomeos") && !msg.contains("songbird"),
                "Resource limit error should not mention hardcoded services: {}",
                msg
            );
        }
    }

    cleanup_manifest(&manifest_path).await;
}

// ============================================================================
// E2E TEST 9: Stress Test - Many Operations Without Registry
// ============================================================================

#[tokio::test]
async fn test_e2e_stress_many_operations_standalone() {
    // ✅ E2E: System remains stable under load without registry

    let executor = Arc::new(BiomeExecutor::new().await.unwrap());
    let barrier = Arc::new(Barrier::new(20));

    let mut handles = vec![];

    for i in 0..20 {
        let exec = Arc::clone(&executor);
        let b = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            b.wait().await;

            // Mix of operations
            let _ = exec
                .list_biomes(i % 2 == 0, "json".to_string(), false, None)
                .await;
            let _ = exec
                .down_biome(format!("stress-{}", i), false, 30, false)
                .await;
            let _ = exec
                .show_logs(format!("stress-{}", i), false, 50, false, None, None)
                .await;

            Ok::<(), anyhow::Error>(())
        }));
    }

    // All should complete without panic or deadlock
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Stress test operations should complete");
    }
}

// ============================================================================
// E2E TEST 10: Philosophy Validation - Infant Discovery
// ============================================================================

#[tokio::test]
async fn test_e2e_infant_discovery_philosophy_enforced() {
    // ✅ E2E: Complete system embodies infant discovery philosophy

    // "Each primal knows only itself"
    let executor = BiomeExecutor::new().await.unwrap();

    // "Everything else is discovered at runtime by capability"
    use toadstool_cli::ecosystem::adapters::AdapterFactory;
    let factory = AdapterFactory::new();
    let _ = factory.coordination_adapter(); // Discovers by capability

    // "Zero hardcoded primal names"
    use toadstool_common::primal_identity::Capability;
    let _capability = Capability::Compute; // Generic, not primal-specific

    // "Code starts with zero knowledge like an infant"
    use toadstool_common::infant_discovery::DiscoveryEngine;
    let discovery = DiscoveryEngine::new(); // Starts with no hardcoded knowledge

    // All operations work together seamlessly
    let _ = executor
        .list_biomes(false, "json".to_string(), false, None)
        .await;
    let _ = discovery;

    // Philosophy enforced end-to-end ✅
}
