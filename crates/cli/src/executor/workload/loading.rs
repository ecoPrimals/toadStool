// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload file loading and parsing.
//!
//! Reads workload specification files from disk (TOML or JSON format).
//! Supports `${VAR}` and `$VAR` environment variable expansion in values
//! before deserialization, making TOMLs portable across gates and hosts.

use crate::{CliContextExt, Result};
use std::path::PathBuf;

use super::spec::WorkloadFile;

/// Expand environment variable references in a string.
///
/// Handles two forms:
/// - `${VAR_NAME}` — braced, unambiguous
/// - `$VAR_NAME`   — bare, terminated by non-alphanumeric/non-underscore
///
/// Undefined variables expand to the empty string with a debug log.
/// Literal `$$` escapes to a single `$`.
fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] != b'$' {
            result.push(bytes[i] as char);
            i += 1;
            continue;
        }

        if i + 1 < len && bytes[i + 1] == b'$' {
            result.push('$');
            i += 2;
            continue;
        }

        if i + 1 < len
            && bytes[i + 1] == b'{'
            && let Some(close) = input[i + 2..].find('}')
        {
            let var_name = &input[i + 2..i + 2 + close];
            match std::env::var(var_name) {
                Ok(val) => result.push_str(&val),
                Err(_) => {
                    tracing::debug!(var = var_name, "Undefined env var in workload file");
                }
            }
            i += 2 + close + 1;
            continue;
        }

        if i + 1 < len && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
            let start = i + 1;
            let mut end = start;
            while end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            let var_name = &input[start..end];
            match std::env::var(var_name) {
                Ok(val) => result.push_str(&val),
                Err(_) => {
                    tracing::debug!(var = var_name, "Undefined env var in workload file");
                }
            }
            i = end;
            continue;
        }

        result.push('$');
        i += 1;
    }

    result
}

/// Load and parse a workload specification file from disk.
/// Supports TOML (`.toml`) and JSON (`.json`) formats.
/// Environment variables (`${VAR}`, `$VAR`) are expanded before parsing.
pub(super) async fn load_workload_file(path: &PathBuf) -> Result<WorkloadFile> {
    let raw = std::fs::read_to_string(path)
        .context(format!("Failed to read workload file: {}", path.display()))?;

    let content = expand_env_vars(&raw);

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

    #[test]
    fn expand_braced_var() {
        temp_env::with_vars([("WETSPRING_DIR", Some("/opt/wetspring"))], || {
            assert_eq!(
                expand_env_vars("working_dir = \"${WETSPRING_DIR}/data\""),
                "working_dir = \"/opt/wetspring/data\""
            );
        });
    }

    #[test]
    fn expand_bare_var() {
        temp_env::with_vars([("HOME", Some("/home/user"))], || {
            assert_eq!(
                expand_env_vars("path = \"$HOME/workloads\""),
                "path = \"/home/user/workloads\""
            );
        });
    }

    #[test]
    fn expand_undefined_var_becomes_empty() {
        temp_env::with_vars([("CERTAINLY_UNDEFINED_XYZ", Option::<&str>::None)], || {
            assert_eq!(expand_env_vars("${CERTAINLY_UNDEFINED_XYZ}/data"), "/data");
        });
    }

    #[test]
    fn expand_double_dollar_escapes() {
        assert_eq!(expand_env_vars("cost = $$100"), "cost = $100");
    }

    #[test]
    fn expand_no_vars_passthrough() {
        let input = "name = \"hello world\"";
        assert_eq!(expand_env_vars(input), input);
    }

    #[test]
    fn expand_mixed_vars() {
        use toadstool_common::constants::primal_identity::PRIMAL_NAME;

        temp_env::with_vars(
            [("APP_DIR", Some("/app")), ("APP_USER", Some(PRIMAL_NAME))],
            || {
                assert_eq!(
                    expand_env_vars("dir = \"${APP_DIR}\" user = \"$APP_USER\""),
                    format!("dir = \"/app\" user = \"{PRIMAL_NAME}\"")
                );
            },
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn load_toml_with_env_expansion() {
        temp_env::async_with_vars([("TOADSTOOL_TEST_CMD", Some("/usr/bin/env"))], async {
            let content = r#"
[metadata]
name = "env-test"

[execution]
type = "native"
command = "${TOADSTOOL_TEST_CMD}"
"#;
            let mut f = NamedTempFile::with_suffix(".toml").unwrap();
            write!(f, "{}", content).unwrap();
            let wf = load_workload_file(&f.path().to_path_buf()).await.unwrap();
            if let super::super::spec::ExecutionSpec::Native { command, .. } = &wf.execution {
                assert_eq!(command, "/usr/bin/env");
            } else {
                panic!("Expected Native execution spec");
            }
        })
        .await;
    }
}
