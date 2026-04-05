// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for public CLI command handlers.

use super::super::*;
use crate::CliContext;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

async fn create_valid_manifest_file(name: &str) -> (PathBuf, TempDir) {
    let temp_dir = TempDir::new().expect("temp dir");
    let manifest_path = temp_dir.path().join("biome.toml");

    let now = std::time::SystemTime::now();
    let created_secs = now.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    let content = format!(
        r#"
[metadata]
name = "{}"
version = "1.0.0"
created = {}
updated = {}
tags = []

[primals.test-primal]
version = "latest"
enabled = true
config = {{}}
dependencies = []

[primals.test-primal.source]
type = "Container"
registry = "registry.example.com"
image = "test-image"
tag = "latest"

[services]

[resources]
cpu_limit = 1.0

[security]
isolation_level = "standard"
trust_level = "medium"
beardog_required = false
crypto_policies = []
allowed_networks = []
forbidden_syscalls = []

[networking]
mode = "bridge"
dns_servers = []
port_mappings = []
network_policies = []

[storage]
datasets = []
volumes = []
"#,
        name, created_secs, created_secs
    );

    fs::write(&manifest_path, content)
        .await
        .expect("write manifest");
    (manifest_path, temp_dir)
}

#[tokio::test]
async fn test_up_biome_success() {
    let (manifest_path, _temp) = create_valid_manifest_file("up-test-biome").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path: manifest_path.clone(),
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    let result = executor.up_biome(&ctx, opts).await;
    assert!(result.is_ok(), "up_biome should succeed: {:?}", result);

    let _ = executor.down_biome("up-test-biome", true, 5, false).await;
    let _ = executor.purge_biome_data("up-test-biome").await;
}

#[tokio::test]
async fn test_up_biome_with_name_override() {
    let (manifest_path, _temp) = create_valid_manifest_file("manifest-name").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path,
        detach: true,
        name: Some("custom-name-override".to_string()),
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    let result = executor.up_biome(&ctx, opts).await;
    assert!(result.is_ok());
    let info = executor.list_biomes(false, "table", false, None).await;
    assert!(info.is_ok());

    let _ = executor
        .down_biome("custom-name-override", true, 5, false)
        .await;
    let _ = executor.purge_biome_data("custom-name-override").await;
}

#[tokio::test]
async fn test_up_biome_already_running_returns_err() {
    let (manifest_path, _temp) = create_valid_manifest_file("already-running").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path: manifest_path.clone(),
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    executor.up_biome(&ctx, opts.clone()).await.unwrap();

    let result = executor.up_biome(&ctx, opts).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already running"));

    let _ = executor.down_biome("already-running", true, 5, false).await;
    let _ = executor.purge_biome_data("already-running").await;
}

#[tokio::test]
async fn test_down_biome_with_purge() {
    let (manifest_path, _temp) = create_valid_manifest_file("purge-test").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path,
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    executor.up_biome(&ctx, opts).await.unwrap();

    let result = executor.down_biome("purge-test", false, 10, true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_biomes_with_running_biome() {
    let (manifest_path, _temp) = create_valid_manifest_file("list-test").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path,
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    executor.up_biome(&ctx, opts).await.unwrap();

    let result = executor.list_biomes(false, "table", true, None).await;
    assert!(result.is_ok());

    let _ = executor.down_biome("list-test", true, 5, false).await;
    let _ = executor.purge_biome_data("list-test").await;
}

#[tokio::test]
async fn test_show_logs_service_not_found() {
    let (manifest_path, _temp) = create_valid_manifest_file("logs-service-test").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path,
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    executor.up_biome(&ctx, opts).await.unwrap();

    let result = executor
        .show_logs(
            "logs-service-test.nonexistent-service",
            false,
            10,
            false,
            None,
            None,
        )
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    let _ = executor
        .down_biome("logs-service-test", true, 5, false)
        .await;
    let _ = executor.purge_biome_data("logs-service-test").await;
}

#[tokio::test]
async fn test_show_logs_biome_service() {
    let (manifest_path, _temp) = create_valid_manifest_file("logs-biome-svc").await;
    let executor = BiomeExecutor::new().await.expect("executor");
    let ctx = CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    };

    let opts = UpBiomeOptions {
        manifest_path,
        detach: true,
        name: None,
        env: vec![],
        restart: false,
        health_interval: 30,
    };

    executor.up_biome(&ctx, opts).await.unwrap();

    // Create log file (lifecycle stores path but doesn't create file)
    let env = toadstool_common::platform_paths::PathEnv::from_env();
    let paths = toadstool_common::platform_paths::PlatformPaths::new(&env);
    let log_path = paths
        .toadstool_log_dir()
        .join("logs-biome-svc")
        .join("test-primal.log");
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    let _ = fs::write(&log_path, "test log line\n").await;

    let result = executor
        .show_logs("logs-biome-svc.test-primal", false, 10, false, None, None)
        .await;
    assert!(
        result.is_ok(),
        "show_logs for primal should succeed: {:?}",
        result
    );

    let _ = executor.down_biome("logs-biome-svc", true, 5, false).await;
    let _ = executor.purge_biome_data("logs-biome-svc").await;
}
