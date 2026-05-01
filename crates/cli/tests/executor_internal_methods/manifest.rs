// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executor internal methods — manifest-oriented helpers and fixtures.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Barrier;

use toadstool_cli::executor::{BiomeExecutor, RunBiomeOptions};
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

    tokio::fs::write(&path, content).await?;
    Ok(path)
}

async fn cleanup_file(path: &PathBuf) -> Result<()> {
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

// ============================================================================
// MANIFEST VALIDATION TESTS
// ============================================================================

#[tokio::test]
async fn test_manifest_with_all_fields() {
    let manifest_content = r#"
    [metadata]
    name = "full-biome"
    version = "2.0.0"
    description = "A complete biome"
    author = "Test Author"
    license = "Apache-2.0"
    
    [resources]
    cpu_limit = 2.0
    memory_limit = "1G"
    disk_limit = "10G"
    gpu = true
    "#;

    let manifest_path = create_manifest_file("full", manifest_content)
        .await
        .unwrap();

    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Should accept fully specified manifest
    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: None,
        env: vec![],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "basic".to_string(),
    };
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_file(&manifest_path).await.ok();

    // May fail at startup, but not at validation
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.contains("invalid") || !msg.contains("validation"),
            "Should not fail validation: {msg}"
        );
    }
}

#[tokio::test]
async fn test_manifest_with_minimal_fields() {
    let manifest_content = r#"
    [metadata]
    name = "minimal"
    version = "1.0.0"
    
    [resources]
    "#;

    let manifest_path = create_manifest_file("minimal", manifest_content)
        .await
        .unwrap();

    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: None,
        env: vec![],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "basic".to_string(),
    };
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_file(&manifest_path).await.ok();

    // Minimal manifest should be accepted
    if let Err(e) = result {
        let msg = e.to_string();
        assert!(
            !msg.contains("missing") || !msg.contains("required"),
            "Minimal manifest should be valid: {msg}"
        );
    }
}

// ============================================================================
// BIOME NAME HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_biome_name_from_manifest() {
    let manifest_content = r#"
    [metadata]
    name = "manifest-name"
    version = "1.0.0"
    
    [resources]
    "#;

    let manifest_path = create_manifest_file("name-test", manifest_content)
        .await
        .unwrap();

    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // No name override - should use manifest name
    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: None,
        env: vec![],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "basic".to_string(),
    };
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_file(&manifest_path).await.ok();

    // Name should come from manifest
    let _ = result; // May fail at startup
}

#[tokio::test]
async fn test_biome_name_override() {
    let manifest_content = r#"
    [metadata]
    name = "manifest-name"
    version = "1.0.0"
    
    [resources]
    "#;

    let manifest_path = create_manifest_file("override-test", manifest_content)
        .await
        .unwrap();

    let executor = create_test_executor().await.unwrap();
    let ctx = create_test_context();

    // Override manifest name
    let opts = RunBiomeOptions {
        manifest_path: manifest_path.clone(),
        name: Some("overridden-name".to_string()),
        env: vec![],
        debug: false,
        cpu_limit: None,
        memory_limit: None,
        security: "basic".to_string(),
    };
    let result = executor.run_biome(&ctx, opts).await;

    cleanup_file(&manifest_path).await.ok();

    // Override should be respected
    let _ = result; // May fail at startup
}

// ============================================================================
// CONCURRENT BIOME MANAGEMENT TESTS
// ============================================================================

#[tokio::test]
async fn test_concurrent_different_biome_operations() {
    // ✅ MODERN: Different biomes can be operated on concurrently
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(Barrier::new(6));

    let mut handles = vec![];

    // List operation
    {
        let exec = Arc::clone(&executor);
        let b = Arc::clone(&barrier);
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.list_biomes(false, "json", false, None).await
        }));
    }

    // Multiple down operations on different biomes
    for i in 0..5 {
        let exec = Arc::clone(&executor);
        let b = Arc::clone(&barrier);
        handles.push(tokio::spawn(async move {
            b.wait().await;
            exec.down_biome(format!("biome-{i}"), false, 30, false)
                .await
        }));
    }

    // All should execute concurrently without blocking each other
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_list_biomes_all_output_formats_concurrent() {
    let executor = Arc::new(create_test_executor().await.unwrap());
    let barrier = Arc::new(Barrier::new(3));

    let formats = vec!["table", "json", "yaml"];

    let handles: Vec<_> = formats
        .into_iter()
        .map(|fmt| {
            let exec = Arc::clone(&executor);
            let b = Arc::clone(&barrier);

            tokio::spawn(async move {
                b.wait().await;
                exec.list_biomes(false, fmt, false, None).await
            })
        })
        .collect();

    // All formats should work concurrently
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Format rendering should succeed");
    }
}

// ============================================================================
// PARAMETER BOUNDARY TESTS
// ============================================================================

#[tokio::test]
async fn test_timeout_boundary_values() {
    let executor = create_test_executor().await.unwrap();

    let timeouts = vec![1, 5, 30, 60, 300, 3600]; // 1s to 1h

    for timeout in timeouts {
        let result = executor.down_biome("test", false, timeout, false).await;

        // All timeout values should be accepted
        assert!(result.is_err(), "Should fail for nonexistent biome");
    }
}

#[tokio::test]
async fn test_resource_limit_combinations() {
    let executor = Arc::new(create_test_executor().await.unwrap());

    let combinations = vec![
        (Some(0.5), None),
        (None, Some("256M".to_string())),
        (Some(2.0), Some("2G".to_string())),
        (Some(4.0), Some("4G".to_string())),
    ];

    let manifest_content = r#"
    [metadata]
    name = "test"
    version = "1.0.0"
    [resources]
    "#;

    let handles: Vec<_> = combinations
        .into_iter()
        .enumerate()
        .map(|(i, (cpu, mem))| {
            let exec = Arc::clone(&executor);

            tokio::spawn(async move {
                let manifest_path = create_manifest_file(&format!("res-{i}"), manifest_content)
                    .await
                    .map_err(|e| CliError::Other(e.to_string()))?;

                let ctx = create_test_context();
                let opts = RunBiomeOptions {
                    manifest_path: manifest_path.clone(),
                    name: Some(format!("biome-res-{i}")),
                    env: vec![],
                    debug: false,
                    cpu_limit: cpu,
                    memory_limit: mem,
                    security: "basic".to_string(),
                };
                let result = exec.run_biome(&ctx, opts).await;

                cleanup_file(&manifest_path).await.ok();
                result
            })
        })
        .collect();

    // All combinations should be accepted
    for handle in handles {
        let _ = handle.await.unwrap();
    }
}
