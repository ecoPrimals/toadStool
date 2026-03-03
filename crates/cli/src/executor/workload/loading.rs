// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload file loading and parsing.
//!
//! Reads workload specification files from disk (TOML or JSON format).

use crate::{CliContextExt, Result};
use std::path::PathBuf;

use super::spec::WorkloadFile;

/// Load and parse a workload specification file from disk.
/// Supports TOML (`.toml`) and JSON (`.json`) formats.
pub(super) async fn load_workload_file(path: &PathBuf) -> Result<WorkloadFile> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context(format!("Failed to read workload file: {}", path.display()))?;

    // Try TOML first, then JSON
    if path.extension().and_then(|s| s.to_str()) == Some("toml") {
        toml::from_str(&content).context(format!(
            "Failed to parse TOML workload file: {}",
            path.display()
        ))
    } else {
        serde_json::from_str(&content).context(format!(
            "Failed to parse JSON workload file: {}",
            path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_toml() {
        let content = r#"
[metadata]
name = "test-workload"
description = "Test"
version = "1.0"

[execution]
type = "native"
command = "/bin/echo"
"#;

        let mut temp_file = NamedTempFile::with_suffix(".toml").unwrap();
        write!(temp_file, "{}", content).unwrap();
        let path = temp_file.path().to_path_buf();

        let result = load_workload_file(&path).await;
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert_eq!(workload.metadata.name, "test-workload");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_json() {
        let content = r#"
{
  "metadata": {"name": "json-workload"},
  "execution": {"type": "python", "file": "script.py"}
}
"#;

        let mut temp_file = NamedTempFile::with_suffix(".json").unwrap();
        write!(temp_file, "{}", content).unwrap();
        let path = temp_file.path().to_path_buf();

        let result = load_workload_file(&path).await;
        assert!(result.is_ok());
        let workload = result.unwrap();
        assert_eq!(workload.metadata.name, "json-workload");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_load_workload_file_not_found() {
        let path = PathBuf::from("/nonexistent/workload.toml");
        let result = load_workload_file(&path).await;
        assert!(result.is_err());
    }
}
