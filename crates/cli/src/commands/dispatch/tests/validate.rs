// SPDX-License-Identifier: AGPL-3.0-or-later
use std::path::PathBuf;

use crate::commands::definitions::Commands;
use crate::commands::dispatch::execute_command;
use crate::{Cli, CliContext};
use tempfile::TempDir;
use tokio::fs;

fn valid_manifest_yaml() -> &'static str {
    r#"
metadata:
  name: test-biome
  version: "1.0.0"
  created: 1735689600
  updated: 1735689600
  tags: []

primals: {}
services: {}

resources:
  cpu_limit: 2.0
  memory_limit: "2GB"

security:
  isolation_level: "high"
  trust_level: "medium"
  security_required: false
  crypto_policies: []
  allowed_networks: []
  forbidden_syscalls: []

networking:
  mode: "bridge"
  dns_servers: []
  port_mappings: []
  network_policies: []

storage:
  storage_integration: false
  datasets: []
  volumes: []
"#
}

#[tokio::test(flavor = "current_thread")]
async fn test_execute_command_validate_valid_manifest() {
    let temp_dir = TempDir::new().expect("temp dir");
    let manifest_path = temp_dir.path().join("biome.yaml");
    fs::write(&manifest_path, valid_manifest_yaml())
        .await
        .expect("write manifest");

    let cli = Cli {
        command: Commands::Validate {
            manifest: manifest_path,
            check_resources: false,
            check_security: false,
            format: "text".to_string(),
        },
        verbose: false,
        config: None,
        directory: None,
    };
    let ctx = CliContext::new(&cli).expect("context");

    let result = execute_command(&cli, &ctx).await;
    assert!(
        result.is_ok(),
        "validate should succeed: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_execute_command_validate_json_format() {
    let temp_dir = TempDir::new().expect("temp dir");
    let manifest_path = temp_dir.path().join("biome.yaml");
    fs::write(&manifest_path, valid_manifest_yaml())
        .await
        .expect("write manifest");

    let cli = Cli {
        command: Commands::Validate {
            manifest: manifest_path,
            check_resources: true,
            check_security: true,
            format: "json".to_string(),
        },
        verbose: false,
        config: None,
        directory: None,
    };
    let ctx = CliContext::new(&cli).expect("context");

    let result = execute_command(&cli, &ctx).await;
    assert!(
        result.is_ok(),
        "validate json should succeed: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_execute_command_validate_nonexistent_manifest() {
    let cli = Cli {
        command: Commands::Validate {
            manifest: PathBuf::from("/nonexistent/path/biome.yaml"),
            check_resources: false,
            check_security: false,
            format: "text".to_string(),
        },
        verbose: false,
        config: None,
        directory: None,
    };
    let ctx = CliContext::new(&cli).expect("context");

    let result = execute_command(&cli, &ctx).await;
    assert!(result.is_err(), "validate nonexistent should fail");
}
