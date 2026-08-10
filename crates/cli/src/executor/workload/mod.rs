// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct workload execution without biome.yaml
//!
//! This module implements the `toadstool execute` command for running
//! workloads directly using ToadStool's runtime engines.
//!
//! ## Module Structure (Refactored by Domain)
//!
//! - `spec`: Workload file format types (WorkloadFile, ExecutionSpec, etc.)
//! - `loading`: Loading and parsing workload files from disk
//! - `conversion`: Converting to ToadStool WorkloadSpec
//! - `runtime`: Runtime type selection and engine registration
//! - `reporting`: Output formatting (text/JSON)

mod conversion;
mod loading;
mod reporting;
mod runtime;
mod spec;

use crate::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use toadstool::{
    execution::ExecutionRequest,
    runtime::{RuntimeOrchestrator, RuntimeSelectionStrategy},
};
use toadstool_server::RuntimeEngineDispatch;

// Re-export public types for external consumers
pub use spec::{ExecutionSpec, ResourceSpec, SecuritySpec, WorkloadFile, WorkloadMetadata};

use conversion::{
    convert_resource_requirements, convert_security_context, convert_to_workload_spec,
};
use loading::load_workload_file;
use reporting::{print_json_output, print_text_output};
pub(crate) use runtime::infer_runtime_type;
use runtime::{parse_runtime_hint, register_runtime_engines};

/// Execute a workload from a specification file
pub async fn execute_workload(
    workload_path: &PathBuf,
    runtime_hint: Option<&str>,
    env_overrides: &[String],
    timeout_secs: u64,
    output_format: &str,
) -> Result<()> {
    info!(
        "📖 Loading workload specification: {}",
        workload_path.display()
    );

    // Load workload file
    let workload_file = load_workload_file(workload_path).await?;

    info!("✅ Loaded workload: {}", workload_file.metadata.name);
    if let Some(desc) = &workload_file.metadata.description {
        info!("   Description: {}", desc);
    }

    // Parse environment overrides (zero-copy: only allocate when inserting into HashMap)
    let env_map: HashMap<String, String> = env_overrides
        .iter()
        .filter_map(|env_pair| {
            env_pair
                .split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect();

    // Create runtime orchestrator
    info!("🔧 Initializing runtime orchestrator");
    let orchestrator = RuntimeOrchestrator::<RuntimeEngineDispatch>::create(
        RuntimeSelectionStrategy::FirstAvailable,
    );

    // Register available runtime engines
    register_runtime_engines(&orchestrator).await?;

    // Validate data dependencies before dispatch
    validate_data_dependencies(&workload_file).await?;

    // Convert workload spec to ToadStool WorkloadSpec
    let workload_spec = convert_to_workload_spec(&workload_file, env_map)?;

    // Determine runtime type
    let runtime_type = if let Some(hint) = runtime_hint {
        parse_runtime_hint(hint)?
    } else {
        infer_runtime_type(&workload_spec)
    };

    // Create execution request
    let request = ExecutionRequest {
        execution_id: Uuid::new_v4(),
        workload: workload_spec,
        runtime_hint: Some(runtime_type),
        resources: convert_resource_requirements(&workload_file.resources),
        security_context: convert_security_context(&workload_file.security),
        timeout: Some(Duration::from_secs(timeout_secs)),
        environment: HashMap::new(),
        input_data: Default::default(),
        callback_config: None,
        encryption_config: None,
    };

    info!("🚀 Executing workload: {}", workload_file.metadata.name);
    debug!("   Execution ID: {}", request.execution_id);
    debug!("   Runtime: {:?}", request.runtime_hint);

    // Execute!
    let start_time = std::time::Instant::now();
    let response = orchestrator.execute(request).await?;
    let duration = start_time.elapsed();

    // Display results
    match output_format {
        "json" => print_json_output(&response, duration)?,
        _ => print_text_output(&workload_file, &response, duration)?,
    }

    Ok(())
}

/// Validate declared data dependencies before workload dispatch.
///
/// For each dependency with a local-path source:
/// - Required deps must exist on disk (error if missing)
/// - Optional deps log a warning if missing
/// - BLAKE3 hash is verified when declared
async fn validate_data_dependencies(workload: &spec::WorkloadFile) -> Result<()> {
    let deps = match &workload.data_dependencies {
        Some(d) if !d.is_empty() => d,
        _ => return Ok(()),
    };

    info!(
        "Validating {} data dependenc{}",
        deps.len(),
        if deps.len() == 1 { "y" } else { "ies" }
    );

    for dep in deps {
        // Skip non-local sources (nestgate://, http://, etc.) — staging TBD
        if dep.source.contains("://") {
            debug!(
                name = dep.name,
                source = dep.source,
                "Skipping remote dependency (staging not yet implemented)"
            );
            continue;
        }

        let path = Path::new(&dep.source);
        if !path.exists() {
            if dep.required {
                return Err(crate::CliError::Other(format!(
                    "Required data dependency '{}' not found at: {}",
                    dep.name, dep.source
                )));
            }
            warn!(
                name = dep.name,
                source = dep.source,
                "Optional data dependency not found — workload may degrade"
            );
            continue;
        }

        // BLAKE3 integrity check when hash is declared
        if let Some(expected_hash) = &dep.blake3 {
            let source = dep.source.clone();
            let expected = expected_hash.clone();
            let name = dep.name.clone();

            let actual = tokio::task::spawn_blocking(move || -> Result<String> {
                let data = std::fs::read(&source).map_err(|e| {
                    crate::CliError::Other(format!(
                        "Failed to read dependency '{}' for integrity check: {e}",
                        name
                    ))
                })?;
                Ok(blake3::hash(&data).to_hex().to_string())
            })
            .await
            .map_err(|e| crate::CliError::Other(format!("BLAKE3 task failed: {e}")))??;

            if actual != expected {
                return Err(crate::CliError::Other(format!(
                    "BLAKE3 mismatch for dependency '{}': expected {expected}, got {actual}",
                    dep.name
                )));
            }
            debug!(name = dep.name, "BLAKE3 integrity verified");
        }

        debug!(name = dep.name, source = dep.source, "Dependency validated");
    }

    info!("All data dependencies validated");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Writes a `.toml` workload fixture and returns the temp file (kept alive) and path.
    fn write_toml_fixture(content: &str) -> (NamedTempFile, PathBuf) {
        let mut tmp = NamedTempFile::with_suffix(".toml").expect("temp toml file");
        write!(tmp, "{content}").expect("write workload toml");
        tmp.flush().expect("flush");
        let path = tmp.path().to_path_buf();
        (tmp, path)
    }

    /// Writes raw bytes to a temp file for dependency validation tests.
    fn write_bytes_fixture(content: &[u8]) -> (NamedTempFile, PathBuf) {
        let mut tmp = NamedTempFile::new().expect("temp file");
        tmp.write_all(content).expect("write bytes");
        tmp.flush().expect("flush");
        let path = tmp.path().to_path_buf();
        (tmp, path)
    }

    #[tokio::test]
    async fn test_execute_workload_file_not_found() {
        let path = PathBuf::from("/nonexistent/workload-12345.toml");
        let result = execute_workload(&path, None, &[], 60, "text").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_workload_invalid_toml() {
        let (_tmp, path) = write_toml_fixture("invalid toml [unclosed");

        let result = execute_workload(&path, None, &[], 60, "text").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_workload_native_echo() {
        let content = r#"
[metadata]
name = "echo-test"
description = "Native echo"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
args = ["hello"]
"#;
        let (_tmp, path) = write_toml_fixture(content);

        let result = execute_workload(&path, None, &[], 10, "text").await;
        assert!(result.is_ok(), "execute_workload failed: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_workload_with_runtime_hint() {
        let content = r#"
[metadata]
name = "hint-test"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
"#;
        let (_tmp, path) = write_toml_fixture(content);

        let result = execute_workload(&path, Some("native"), &[], 10, "text").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_workload_json_output() {
        let content = r#"
[metadata]
name = "json-output-test"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
args = ["test"]
"#;
        let (_tmp, path) = write_toml_fixture(content);

        let result = execute_workload(&path, None, &[], 10, "json").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_execute_workload_with_env_overrides() {
        let content = r#"
[metadata]
name = "env-test"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
"#;
        let (_tmp, path) = write_toml_fixture(content);

        let env = vec!["CUSTOM_VAR=value".to_string()];
        let result = execute_workload(&path, None, &env, 10, "text").await;
        assert!(result.is_ok());
    }

    // ── data_dependencies validation tests ──

    fn make_workload(deps: Option<Vec<spec::DataDependency>>) -> spec::WorkloadFile {
        spec::WorkloadFile {
            metadata: spec::WorkloadMetadata {
                name: "dep-test".into(),
                description: None,
                version: None,
            },
            execution: spec::ExecutionSpec::Native {
                command: "/bin/echo".into(),
                args: None,
                working_dir: None,
                env: None,
            },
            resources: None,
            security: None,
            data_dependencies: deps,
        }
    }

    #[tokio::test]
    async fn test_validate_no_deps() {
        let wl = make_workload(None);
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_empty_deps() {
        let wl = make_workload(Some(vec![]));
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_required_dep_missing() {
        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "missing".into(),
            source: "/nonexistent/data-78901.bin".into(),
            blake3: None,
            required: true,
        }]));
        let err = validate_data_dependencies(&wl)
            .await
            .expect_err("required dep missing");
        assert!(err.to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_validate_optional_dep_missing_ok() {
        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "optional".into(),
            source: "/nonexistent/data-78902.bin".into(),
            blake3: None,
            required: false,
        }]));
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_dep_exists() {
        let (_tmp, path) = write_bytes_fixture(b"");
        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "present".into(),
            source: path.to_string_lossy().into_owned(),
            blake3: None,
            required: true,
        }]));
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_blake3_match() {
        let (_tmp, path) = write_bytes_fixture(b"hello world");

        let hash = blake3::hash(b"hello world").to_hex().to_string();
        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "hashed".into(),
            source: path.to_string_lossy().into_owned(),
            blake3: Some(hash),
            required: true,
        }]));
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_blake3_mismatch() {
        let (_tmp, path) = write_bytes_fixture(b"hello world");

        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "bad-hash".into(),
            source: path.to_string_lossy().into_owned(),
            blake3: Some("0000000000000000000000000000000000000000000000000000000000000000".into()),
            required: true,
        }]));
        let err = validate_data_dependencies(&wl)
            .await
            .expect_err("BLAKE3 mismatch");
        assert!(err.to_string().contains("BLAKE3 mismatch"));
    }

    #[tokio::test]
    async fn test_validate_remote_dep_skipped() {
        let wl = make_workload(Some(vec![spec::DataDependency {
            name: "remote".into(),
            source: "nestgate://artifact-12345".into(),
            blake3: None,
            required: true,
        }]));
        assert!(validate_data_dependencies(&wl).await.is_ok());
    }
}
