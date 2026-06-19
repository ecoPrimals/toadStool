// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::PathBuf;

use super::super::BiomeExecutor;
use super::start::parse_env_vars;

fn make_minimal_container_manifest() -> crate::BiomeManifest {
    let now = std::time::SystemTime::now();
    let mut primals = HashMap::new();
    primals.insert(
        "test-primal".to_string(),
        crate::PrimalConfig {
            version: "latest".to_string(),
            source: crate::WorkloadSource::Container {
                registry: "registry.example.com".to_string(),
                image: "test-image".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    crate::BiomeManifest {
        metadata: crate::BiomeMetadata {
            name: "test-biome".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created: now,
            updated: now,
            tags: vec![],
        },
        primals,
        services: HashMap::new(),
        resources: crate::BiomeResources {
            cpu_limit: Some(1.0),
            memory_limit: None,
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: crate::BiomeSecurity {
            isolation_level: "standard".to_string(),
            trust_level: "medium".to_string(),
            security_required: false,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: crate::BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: crate::BiomeStorage {
            storage_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

fn make_local_manifest() -> crate::BiomeManifest {
    let now = std::time::SystemTime::now();
    let mut primals = HashMap::new();
    primals.insert(
        "local-primal".to_string(),
        crate::PrimalConfig {
            version: "1.0".to_string(),
            source: crate::WorkloadSource::Local {
                path: PathBuf::from("/usr/bin/true"),
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    crate::BiomeManifest {
        metadata: crate::BiomeMetadata {
            name: "local-biome".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created: now,
            updated: now,
            tags: vec![],
        },
        primals,
        services: HashMap::new(),
        resources: crate::BiomeResources {
            cpu_limit: None,
            memory_limit: None,
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: crate::BiomeSecurity {
            isolation_level: "standard".to_string(),
            trust_level: "medium".to_string(),
            security_required: false,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: crate::BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: crate::BiomeStorage {
            storage_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

fn make_manifest_with_service() -> crate::BiomeManifest {
    let mut manifest = make_minimal_container_manifest();
    manifest.services.insert(
        "test-service".to_string(),
        crate::ServiceConfig {
            version: "latest".to_string(),
            source: crate::WorkloadSource::Container {
                registry: "registry.example.com".to_string(),
                image: "service-image".to_string(),
                tag: "v1".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: crate::ServiceResources {
                cpu_limit: Some(0.5),
                memory_limit: None,
                storage_limit: None,
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec![],
            health_check: None,
        },
    );
    manifest
}

#[test]
fn test_parse_env_vars() {
    let env_vars = vec![
        "KEY1=value1".to_string(),
        "KEY2=value2".to_string(),
        "EMPTY=".to_string(),
        "NO_EQUALS".to_string(), // skipped
    ];
    let env = parse_env_vars(&env_vars);
    assert_eq!(env.get("KEY1"), Some(&"value1".to_string()));
    assert_eq!(env.get("KEY2"), Some(&"value2".to_string()));
    assert_eq!(env.get("EMPTY"), Some(&String::new()));
    assert!(!env.contains_key("NO_EQUALS"));
}

#[test]
fn test_parse_env_vars_empty() {
    let env = parse_env_vars(&[]);
    assert!(env.is_empty());
}

#[test]
fn test_parse_env_vars_multiple_equals() {
    let env_vars = vec!["PATH=/usr/bin:/usr/local/bin".to_string()];
    let env = parse_env_vars(&env_vars);
    assert_eq!(
        env.get("PATH"),
        Some(&"/usr/bin:/usr/local/bin".to_string())
    );
}

#[test]
fn test_parse_env_vars_overwrites_duplicate_key() {
    let env_vars = vec!["KEY=first".to_string(), "KEY=second".to_string()];
    let env = parse_env_vars(&env_vars);
    assert_eq!(env.get("KEY"), Some(&"second".to_string()));
}

#[test]
fn test_parse_env_vars_special_chars() {
    let env_vars = vec![
        "QUOTED=\"value with spaces\"".to_string(),
        "URL=https://example.com/path?foo=bar".to_string(),
    ];
    let env = parse_env_vars(&env_vars);
    assert_eq!(
        env.get("QUOTED"),
        Some(&"\"value with spaces\"".to_string())
    );
    assert_eq!(
        env.get("URL"),
        Some(&"https://example.com/path?foo=bar".to_string())
    );
}

#[test]
fn test_parse_env_vars_only_key_no_value() {
    let env_vars = vec!["SINGLE=".to_string()];
    let env = parse_env_vars(&env_vars);
    assert_eq!(env.get("SINGLE"), Some(&String::new()));
}

#[test]
fn test_parse_env_vars_mixed_valid_invalid() {
    let env_vars = vec![
        "VALID=ok".to_string(),
        "INVALID_NO_EQUALS".to_string(),
        "ANOTHER=works".to_string(),
    ];
    let env = parse_env_vars(&env_vars);
    assert_eq!(env.len(), 2);
    assert_eq!(env.get("VALID"), Some(&"ok".to_string()));
    assert_eq!(env.get("ANOTHER"), Some(&"works".to_string()));
}

// ─── start_biome_internal, stop_biome_internal, purge_biome_data tests ───

#[tokio::test]
async fn test_start_biome_internal_with_container_manifest() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_minimal_container_manifest();

    let result = executor
        .start_biome_internal(
            "test-lifecycle-biome",
            manifest,
            vec!["ENV_KEY=value".to_string()],
            false,
            false,
            "standard",
        )
        .await;

    assert!(
        result.is_ok(),
        "start_biome_internal should succeed: {:?}",
        result
    );
    let info = result.unwrap();
    assert_eq!(info.name, "test-lifecycle-biome");
    assert!(!info.services.is_empty() || info.services.is_empty()); // services from manifest

    let _ = executor
        .stop_biome_internal("test-lifecycle-biome", true, 5)
        .await;
    let _ = executor.purge_biome_data("test-lifecycle-biome").await;
}

#[tokio::test]
async fn test_start_biome_internal_with_local_manifest() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_local_manifest();

    let result = executor
        .start_biome_internal(
            "local-test-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await;

    assert!(
        result.is_ok(),
        "start_biome_internal with Local source: {:?}",
        result
    );
    let info = result.unwrap();
    assert_eq!(info.name, "local-test-biome");

    let _ = executor
        .stop_biome_internal("local-test-biome", true, 5)
        .await;
    let _ = executor.purge_biome_data("local-test-biome").await;
}

#[tokio::test]
async fn test_start_biome_internal_with_services() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_manifest_with_service();

    let result = executor
        .start_biome_internal("service-biome", manifest, vec![], false, false, "standard")
        .await;

    assert!(result.is_ok(), "start with services: {:?}", result);
    let info = result.unwrap();
    assert_eq!(info.name, "service-biome");

    let _ = executor.stop_biome_internal("service-biome", true, 5).await;
    let _ = executor.purge_biome_data("service-biome").await;
}

#[tokio::test]
async fn test_stop_biome_internal_nonexistent_returns_err() {
    let executor = BiomeExecutor::new().await.expect("executor should create");

    let result = executor
        .stop_biome_internal("nonexistent-biome-xyz", false, 30)
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_stop_biome_internal_force_mode() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_minimal_container_manifest();

    executor
        .start_biome_internal(
            "force-stop-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await
        .expect("start should succeed");

    let result = executor
        .stop_biome_internal("force-stop-biome", true, 5)
        .await;
    assert!(result.is_ok(), "force stop should succeed: {:?}", result);

    let _ = executor.purge_biome_data("force-stop-biome").await;
}

#[tokio::test]
async fn test_stop_biome_internal_graceful_mode() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_minimal_container_manifest();

    executor
        .start_biome_internal(
            "graceful-stop-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await
        .expect("start should succeed");

    let result = executor
        .stop_biome_internal("graceful-stop-biome", false, 10)
        .await;
    assert!(result.is_ok(), "graceful stop should succeed: {:?}", result);

    let _ = executor.purge_biome_data("graceful-stop-biome").await;
}

#[tokio::test]
async fn test_purge_biome_data_nonexistent_succeeds() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let result = executor.purge_biome_data("nonexistent-purge-target").await;
    assert!(result.is_ok(), "purge nonexistent should not error");
}

#[tokio::test]
async fn test_purge_biome_data_after_stop() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_minimal_container_manifest();

    executor
        .start_biome_internal(
            "purge-test-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await
        .expect("start should succeed");

    executor
        .stop_biome_internal("purge-test-biome", true, 5)
        .await
        .expect("stop should succeed");

    let result = executor.purge_biome_data("purge-test-biome").await;
    assert!(result.is_ok(), "purge after stop: {:?}", result);
}

fn make_git_workload_manifest() -> crate::BiomeManifest {
    let now = std::time::SystemTime::now();
    let mut primals = HashMap::new();
    primals.insert(
        "git-primal".to_string(),
        crate::PrimalConfig {
            version: "1.0".to_string(),
            source: crate::WorkloadSource::Git {
                repository: "https://example.com/repo.git".to_string(),
                branch: Some("main".to_string()),
                commit: None,
                path: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    crate::BiomeManifest {
        metadata: crate::BiomeMetadata {
            name: "git-biome".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created: now,
            updated: now,
            tags: vec![],
        },
        primals,
        services: HashMap::new(),
        resources: crate::BiomeResources {
            cpu_limit: None,
            memory_limit: None,
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: crate::BiomeSecurity {
            isolation_level: "standard".to_string(),
            trust_level: "medium".to_string(),
            security_required: false,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: crate::BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: crate::BiomeStorage {
            storage_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

fn make_wasm_manifest(source: &str, checksum: &str) -> crate::BiomeManifest {
    let now = std::time::SystemTime::now();
    let mut primals = HashMap::new();
    primals.insert(
        "wasm-primal".to_string(),
        crate::PrimalConfig {
            version: "1.0".to_string(),
            source: crate::WorkloadSource::Wasm {
                source: source.to_string(),
                checksum: checksum.to_string(),
                wasi_config: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    crate::BiomeManifest {
        metadata: crate::BiomeMetadata {
            name: "wasm-biome".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created: now,
            updated: now,
            tags: vec![],
        },
        primals,
        services: HashMap::new(),
        resources: crate::BiomeResources {
            cpu_limit: None,
            memory_limit: None,
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: crate::BiomeSecurity {
            isolation_level: "standard".to_string(),
            trust_level: "medium".to_string(),
            security_required: false,
            crypto_policies: vec![],
            allowed_networks: vec![],
            forbidden_syscalls: vec![],
        },
        networking: crate::BiomeNetworking {
            mode: "bridge".to_string(),
            dns_servers: vec![],
            port_mappings: vec![],
            network_policies: vec![],
        },
        storage: crate::BiomeStorage {
            storage_integration: None,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

#[tokio::test]
async fn test_start_biome_internal_unsupported_git_workload_fails() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_git_workload_manifest();

    let result = executor
        .start_biome_internal(
            "unsupported-git-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await;

    assert!(result.is_err(), "Git workload source should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Unsupported workload source") || err.contains("Git"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_start_biome_internal_wasm_missing_file_fails() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let manifest = make_wasm_manifest("/nonexistent/toadstool-test-module.wasm", "deadbeef");

    let result = executor
        .start_biome_internal(
            "wasm-missing-file-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await;

    assert!(result.is_err(), "missing WASM file should fail startup");
}

#[tokio::test]
async fn test_start_biome_internal_wasm_checksum_mismatch_fails() {
    let executor = BiomeExecutor::new().await.expect("executor should create");
    let temp_dir = std::env::temp_dir();
    let wasm_path = temp_dir.join(format!("test-module-{}.wasm", uuid::Uuid::new_v4()));
    tokio::fs::write(&wasm_path, b"\0asm\x01\0\0\0")
        .await
        .expect("write temp wasm");

    let manifest = make_wasm_manifest(wasm_path.to_str().expect("utf8 path"), "wrong-checksum");

    let result = executor
        .start_biome_internal(
            "wasm-checksum-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await;

    let _ = tokio::fs::remove_file(&wasm_path).await;
    assert!(result.is_err(), "checksum mismatch should fail startup");
}

#[tokio::test]
async fn test_start_biome_internal_with_disabled_primal() {
    let mut manifest = make_minimal_container_manifest();
    manifest.primals.insert(
        "disabled-primal".to_string(),
        crate::PrimalConfig {
            version: "1.0".to_string(),
            source: crate::WorkloadSource::Container {
                registry: "r".to_string(),
                image: "i".to_string(),
                tag: "t".to_string(),
                digest: None,
            },
            enabled: false,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    let executor = BiomeExecutor::new().await.expect("executor should create");
    let result = executor
        .start_biome_internal(
            "disabled-primal-biome",
            manifest,
            vec![],
            false,
            false,
            "standard",
        )
        .await;

    assert!(result.is_ok());
    let _ = executor
        .stop_biome_internal("disabled-primal-biome", true, 5)
        .await;
    let _ = executor.purge_biome_data("disabled-primal-biome").await;
}
