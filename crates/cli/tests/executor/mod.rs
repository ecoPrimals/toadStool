// SPDX-License-Identifier: AGPL-3.0-only
//! Shared test helpers for executor tests
//!
//! **Testing Philosophy**:
//! - **Concurrent-Safe**: All tests use isolated state, no global pollution
//! - **Event-Driven**: Use Notify/Barrier for coordination, NO sleeps
//! - **TDD Approach**: Write test first, make it pass, refactor
//!
//! ## Module Organization
//!
//! Tests are organized by domain:
//! - `lifecycle`: Executor creation, initialization, basic operations
//! - `manifest_handling`: Manifest parsing, validation, error handling
//! - `resource_management`: CPU/memory limits, quotas, overrides
//! - `concurrent_operations`: Concurrency tests, race conditions, stress tests
//! - `biome_operations`: up, down, restart, status operations
//! - `error_handling`: Error paths, timeouts, boundary conditions
//! - `parameter_validation`: Input validation, security levels
//! - `environment`: Environment variables, configuration
//! - `logging`: Log management and output tests

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Barrier;

use toadstool_cli::executor::{BiomeExecutor, RunBiomeOptions, UpBiomeOptions};
use toadstool_cli::CliContext;

// Re-export common dependencies for submodules
pub use anyhow;
pub use std;
pub use tokio;
pub use toadstool_cli;

// ============================================================================
// Shared Test Helpers (Concurrent-Safe)
// ============================================================================

/// Create isolated test executor with minimal dependencies
pub async fn create_test_executor() -> Result<BiomeExecutor> {
    // Each test gets its own isolated executor
    BiomeExecutor::new().await
}

/// Create default run_biome options for testing
pub fn run_biome_opts(
    manifest_path: std::path::PathBuf,
    name: Option<String>,
    env: Vec<String>,
    debug: bool,
    cpu_limit: Option<f64>,
    memory_limit: Option<String>,
    security: String,
) -> RunBiomeOptions {
    RunBiomeOptions {
        manifest_path,
        name,
        env,
        debug,
        cpu_limit,
        memory_limit,
        security,
    }
}

/// Create default up_biome options for testing
pub fn up_biome_opts(
    manifest_path: std::path::PathBuf,
    detach: bool,
    name: Option<String>,
    env: Vec<String>,
    restart: bool,
    health_interval: u64,
) -> UpBiomeOptions {
    UpBiomeOptions {
        manifest_path,
        detach,
        name,
        env,
        restart,
        health_interval,
    }
}

/// Create test CLI context (isolated per test)
pub fn create_test_context() -> CliContext {
    CliContext {
        config_path: None,
        working_dir: std::env::current_dir().unwrap(),
        verbose: false,
    }
}

/// Create minimal valid manifest for testing
pub fn create_test_manifest_content() -> String {
    r#"
    [metadata]
    name = "test-biome"
    version = "1.0.0"
    
    [resources]
    cpu_limit = 1.0
    memory_limit = "512M"
    "#
    .to_string()
}

/// Create test manifest file (unique per test to avoid conflicts)
pub async fn create_test_manifest_file(test_name: &str) -> Result<PathBuf> {
    use tokio::fs;
    use uuid::Uuid;

    let temp_dir = std::env::temp_dir();
    let unique_id = Uuid::new_v4();
    let manifest_path = temp_dir.join(format!("test-{}-{}.toml", test_name, unique_id));

    fs::write(&manifest_path, create_test_manifest_content()).await?;

    Ok(manifest_path)
}

/// Cleanup test manifest (async drop)
pub async fn cleanup_test_manifest(path: &PathBuf) -> Result<()> {
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

// ============================================================================
// Test Modules (Domain-Driven Organization)
// ============================================================================

mod lifecycle;
mod manifest_handling;
mod resource_management;
mod concurrent_operations;
mod biome_operations;
mod error_handling;
mod parameter_validation;
mod environment;
mod logging;

