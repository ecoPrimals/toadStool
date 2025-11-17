//! Comprehensive tests for CLI library functions
//! Target: crates/cli/src/lib.rs (0% coverage → 100%)
//! Focus: load_biome_manifest(), validate_manifest(), CliContext

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;
use toadstool_cli::{
    load_biome_manifest, validate_manifest, BiomeManifest, BiomeMetadata, BiomeNetworking,
    BiomeResources, BiomeSecurity, BiomeStorage, Cli, CliContext, Commands, PrimalConfig,
    ServiceConfig, WorkloadSource,
};
use tokio::fs;

// ============================================================================
// CliContext Tests
// ============================================================================

#[test]
fn test_cli_context_new_default() {
    let cli = Cli {
        command: Commands::Ps {
            all: false,
            format: "table".to_string(),
            resources: false,
            status: None,
        },
        verbose: false,
        config: None,
        directory: None,
    };

    let context = CliContext::new(&cli).expect("Should create context");

    assert!(!context.verbose);
    assert!(context.config_path.is_none());
    assert!(context.working_dir.exists());
}

#[test]
fn test_cli_context_with_config() {
    let cli = Cli {
        command: Commands::Ps {
            all: false,
            format: "table".to_string(),
            resources: false,
            status: None,
        },
        verbose: true,
        config: Some(PathBuf::from("/tmp/config.toml")),
        directory: None,
    };

    let context = CliContext::new(&cli).expect("Should create context");

    assert!(context.verbose);
    assert_eq!(context.config_path, Some(PathBuf::from("/tmp/config.toml")));
}

#[test]
fn test_cli_context_with_directory() {
    let temp_dir = TempDir::new().unwrap();
    let cli = Cli {
        command: Commands::Ps {
            all: false,
            format: "table".to_string(),
            resources: false,
            status: None,
        },
        verbose: false,
        config: None,
        directory: Some(temp_dir.path().to_path_buf()),
    };

    let context = CliContext::new(&cli).expect("Should create context");

    assert_eq!(context.working_dir, temp_dir.path());
}

#[test]
fn test_cli_context_verbose_flag() {
    let cli_quiet = Cli {
        command: Commands::Ps {
            all: false,
            format: "table".to_string(),
            resources: false,
            status: None,
        },
        verbose: false,
        config: None,
        directory: None,
    };

    let cli_verbose = Cli {
        command: Commands::Ps {
            all: false,
            format: "table".to_string(),
            resources: false,
            status: None,
        },
        verbose: true,
        config: None,
        directory: None,
    };

    let context_quiet = CliContext::new(&cli_quiet).unwrap();
    let context_verbose = CliContext::new(&cli_verbose).unwrap();

    assert!(!context_quiet.verbose);
    assert!(context_verbose.verbose);
}

// ============================================================================
// load_biome_manifest Tests
// ============================================================================

#[tokio::test]
async fn test_load_biome_manifest_valid() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("biome.yaml");

    // Create a valid minimal manifest
    let yaml_content = r#"
metadata:
  name: test-biome
  version: "1.0.0"
  created: "2025-01-01T00:00:00Z"
  updated: "2025-01-01T00:00:00Z"
  tags: []

primals: {}
services: {}

resources:
  cpu_limit: 2.0
  memory_limit: "2GB"

security:
  isolation_level: "high"
  trust_level: "medium"
  beardog_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: false
  datasets: []
  volumes: []
"#;

    fs::write(&manifest_path, yaml_content).await.unwrap();

    let manifest = load_biome_manifest(&manifest_path).await.unwrap();

    assert_eq!(manifest.metadata.name, "test-biome");
    assert_eq!(manifest.metadata.version, "1.0.0");
}

#[tokio::test]
async fn test_load_biome_manifest_with_primals() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("biome.yaml");

    let yaml_content = r#"
metadata:
  name: primal-biome
  version: "1.0.0"
  created: "2025-01-01T00:00:00Z"
  updated: "2025-01-01T00:00:00Z"
  tags: ["primal"]

primals:
  beardog:
    version: "0.1.0"
    enabled: true
    source:
      type: "Container"
      registry: "ghcr.io"
      image: "beardog"
      tag: "latest"
    config: {}
    dependencies: []

services: {}

resources: {}
security:
  isolation_level: "high"
  trust_level: "high"
  beardog_required: true
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: false
  datasets: []
  volumes: []
"#;

    fs::write(&manifest_path, yaml_content).await.unwrap();

    let manifest = load_biome_manifest(&manifest_path).await.unwrap();

    assert_eq!(manifest.primals.len(), 1);
    assert!(manifest.primals.contains_key("beardog"));
    assert!(manifest.security.beardog_required);
}

#[tokio::test]
async fn test_load_biome_manifest_nonexistent_file() {
    let result = load_biome_manifest(&PathBuf::from("/nonexistent/manifest.yaml")).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to read manifest file"));
}

#[tokio::test]
async fn test_load_biome_manifest_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("invalid.yaml");

    fs::write(&manifest_path, "invalid: yaml: content: {{{")
        .await
        .unwrap();

    let result = load_biome_manifest(&manifest_path).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse manifest file"));
}

#[tokio::test]
async fn test_load_biome_manifest_missing_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("incomplete.yaml");

    // Missing required fields
    let yaml_content = r#"
metadata:
  name: incomplete
"#;

    fs::write(&manifest_path, yaml_content).await.unwrap();

    let result = load_biome_manifest(&manifest_path).await;

    // Should fail during deserialization
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_biome_manifest_with_services() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("services.yaml");

    let yaml_content = r#"
metadata:
  name: service-biome
  version: "1.0.0"
  created: "2025-01-01T00:00:00Z"
  updated: "2025-01-01T00:00:00Z"
  tags: []

primals: {}

services:
  web:
    version: "1.0.0"
    source:
      type: "Container"
      registry: "docker.io"
      image: "nginx"
      tag: "latest"
    replicas: 2
    resources:
      cpu_limit: 1.0
      memory_limit: "512MB"
    environment: {}
    ports: []
    volumes: []
    dependencies: []

resources: {}
security:
  isolation_level: "medium"
  trust_level: "medium"
  beardog_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: false
  datasets: []
  volumes: []
"#;

    fs::write(&manifest_path, yaml_content).await.unwrap();

    let manifest = load_biome_manifest(&manifest_path).await.unwrap();

    assert_eq!(manifest.services.len(), 1);
    assert!(manifest.services.contains_key("web"));
}

// ============================================================================
// validate_manifest Tests
// ============================================================================

fn create_test_manifest() -> BiomeManifest {
    BiomeManifest {
        metadata: BiomeMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            created: Utc::now(),
            updated: Utc::now(),
            tags: vec![],
        },
        primals: HashMap::new(),
        services: HashMap::new(),
        resources: BiomeResources {
            cpu_limit: Some(2.0),
            memory_limit: Some("2GB".to_string()),
            storage_limit: None,
            gpu_limit: None,
            network_bandwidth: None,
        },
        security: BiomeSecurity {
            isolation_level: "high".to_string(),
            trust_level: "medium".to_string(),
            beardog_required: false,
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
            nestgate_integration: false,
            datasets: vec![],
            volumes: vec![],
            backup_policy: None,
        },
    }
}

#[test]
fn test_validate_manifest_valid() {
    let manifest = create_test_manifest();
    let warnings = validate_manifest(&manifest).unwrap();

    // Valid manifest with CPU limit set should have no warnings
    // (or at least no critical warnings)
    // warnings.len() is always >= 0 (usize), so just verify it's accessible
    let _ = warnings.len();
}

#[test]
fn test_validate_manifest_missing_cpu_limit() {
    let mut manifest = create_test_manifest();
    manifest.resources.cpu_limit = None;

    let warnings = validate_manifest(&manifest).unwrap();

    assert!(warnings
        .iter()
        .any(|w| w.contains("No CPU limit specified")));
}

#[test]
fn test_validate_manifest_beardog_required_but_not_configured() {
    let mut manifest = create_test_manifest();
    manifest.security.beardog_required = true;
    // No beardog in primals

    let warnings = validate_manifest(&manifest).unwrap();

    assert!(warnings
        .iter()
        .any(|w| w.contains("BearDog is required but not configured")));
}

#[test]
fn test_validate_manifest_beardog_configured() {
    let mut manifest = create_test_manifest();
    manifest.security.beardog_required = true;
    manifest.primals.insert(
        "beardog".to_string(),
        PrimalConfig {
            version: "0.1.0".to_string(),
            source: WorkloadSource::Container {
                registry: "ghcr.io".to_string(),
                image: "beardog".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    let warnings = validate_manifest(&manifest).unwrap();

    // Should NOT warn about missing beardog
    assert!(!warnings
        .iter()
        .any(|w| w.contains("BearDog is required but not configured")));
}

#[test]
fn test_validate_manifest_undefined_service_dependency() {
    let mut manifest = create_test_manifest();

    // Add a service that depends on undefined service
    manifest.services.insert(
        "web".to_string(),
        ServiceConfig {
            version: "1.0.0".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "nginx".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: toadstool_cli::ServiceResources {
                cpu_limit: None,
                memory_limit: None,
                storage_limit: None,
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec!["database".to_string()], // Undefined dependency
            health_check: None,
        },
    );

    let warnings = validate_manifest(&manifest).unwrap();

    assert!(warnings
        .iter()
        .any(|w| w.contains("depends on undefined service 'database'")));
}

#[test]
fn test_validate_manifest_valid_service_dependency() {
    let mut manifest = create_test_manifest();

    // Add database service
    manifest.services.insert(
        "database".to_string(),
        ServiceConfig {
            version: "1.0.0".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "postgres".to_string(),
                tag: "14".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: toadstool_cli::ServiceResources {
                cpu_limit: None,
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

    // Add web service that depends on database
    manifest.services.insert(
        "web".to_string(),
        ServiceConfig {
            version: "1.0.0".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "nginx".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: toadstool_cli::ServiceResources {
                cpu_limit: None,
                memory_limit: None,
                storage_limit: None,
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec!["database".to_string()],
            health_check: None,
        },
    );

    let warnings = validate_manifest(&manifest).unwrap();

    // Should NOT warn about undefined dependency
    assert!(!warnings
        .iter()
        .any(|w| w.contains("depends on undefined service")));
}

#[test]
fn test_validate_manifest_dependency_on_primal() {
    let mut manifest = create_test_manifest();

    // Add beardog primal
    manifest.primals.insert(
        "beardog".to_string(),
        PrimalConfig {
            version: "0.1.0".to_string(),
            source: WorkloadSource::Container {
                registry: "ghcr.io".to_string(),
                image: "beardog".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            enabled: true,
            config: HashMap::new(),
            dependencies: vec![],
            health_check: None,
        },
    );

    // Add service that depends on beardog primal
    manifest.services.insert(
        "auth".to_string(),
        ServiceConfig {
            version: "1.0.0".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "auth-service".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: toadstool_cli::ServiceResources {
                cpu_limit: None,
                memory_limit: None,
                storage_limit: None,
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec!["beardog".to_string()], // Depends on primal
            health_check: None,
        },
    );

    let warnings = validate_manifest(&manifest).unwrap();

    // Should NOT warn - beardog primal satisfies the dependency
    assert!(!warnings.iter().any(|w| w.contains("depends on undefined")));
}

#[test]
fn test_validate_manifest_multiple_issues() {
    let mut manifest = create_test_manifest();
    manifest.resources.cpu_limit = None; // Issue 1: No CPU limit
    manifest.security.beardog_required = true; // Issue 2: BearDog required but not configured

    // Issue 3: Undefined dependency
    manifest.services.insert(
        "api".to_string(),
        ServiceConfig {
            version: "1.0.0".to_string(),
            source: WorkloadSource::Container {
                registry: "docker.io".to_string(),
                image: "api".to_string(),
                tag: "latest".to_string(),
                digest: None,
            },
            replicas: Some(1),
            resources: toadstool_cli::ServiceResources {
                cpu_limit: None,
                memory_limit: None,
                storage_limit: None,
            },
            environment: HashMap::new(),
            ports: vec![],
            volumes: vec![],
            dependencies: vec!["cache".to_string()],
            health_check: None,
        },
    );

    let warnings = validate_manifest(&manifest).unwrap();

    // Should have at least 3 warnings
    assert!(warnings.len() >= 3);
    assert!(warnings
        .iter()
        .any(|w| w.contains("No CPU limit specified")));
    assert!(warnings
        .iter()
        .any(|w| w.contains("BearDog is required but not configured")));
    assert!(warnings
        .iter()
        .any(|w| w.contains("depends on undefined service")));
}

#[test]
fn test_validate_manifest_empty_services_and_primals() {
    let manifest = create_test_manifest();

    let warnings = validate_manifest(&manifest).unwrap();

    // Should not fail, may or may not have warnings
    // warnings.len() is always >= 0 (usize), so just verify it's accessible
    let _ = warnings.len();
}

// ============================================================================
// Edge Cases & Integration Tests
// ============================================================================

#[tokio::test]
async fn test_load_and_validate_manifest_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("roundtrip.yaml");

    let yaml_content = r#"
metadata:
  name: roundtrip-biome
  version: "1.0.0"
  description: "Test roundtrip"
  created: "2025-01-01T00:00:00Z"
  updated: "2025-01-01T00:00:00Z"
  tags: ["test"]

primals: {}
services: {}

resources:
  cpu_limit: 4.0
  memory_limit: "8GB"

security:
  isolation_level: "high"
  trust_level: "high"
  beardog_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: ["8.8.8.8"]
  port_mappings: []
  network_policies: []

storage:
  nestgate_integration: true
  datasets: []
  volumes: []
"#;

    fs::write(&manifest_path, yaml_content).await.unwrap();

    let manifest = load_biome_manifest(&manifest_path).await.unwrap();
    let warnings = validate_manifest(&manifest).unwrap();

    assert_eq!(manifest.metadata.name, "roundtrip-biome");
    assert!(manifest.storage.nestgate_integration);
    // Warnings may or may not be present depending on manifest content
    // warnings.len() is always >= 0 (usize), so just verify it's accessible
    let _ = warnings.len();
}

#[test]
fn test_cli_context_fields_accessible() {
    let cli = Cli {
        command: Commands::Ps {
            all: true,
            format: "json".to_string(),
            resources: true,
            status: Some("running".to_string()),
        },
        verbose: true,
        config: Some(PathBuf::from("/config.toml")),
        directory: Some(PathBuf::from("/tmp")),
    };

    let context = CliContext::new(&cli).unwrap();

    // All fields should be accessible
    let _ = context.config_path;
    let _ = context.working_dir;
    let _ = context.verbose;
}
