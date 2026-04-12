// SPDX-License-Identifier: AGPL-3.0-or-later
//! Socket path format and biomeOS directory layout
//!
//! Defines the standard socket path resolution and directory structure
//! for ecoBin/UniBin deployment.

use std::path::{Path, PathBuf};

use tracing::info;

use crate::errors::{ServerError, ServerResult};

/// Ensure biomeos directory exists with proper permissions
///
/// # Errors
///
/// Returns an error if directory creation or permission setting fails.
pub fn ensure_biomeos_directory(runtime_dir: &Path) -> ServerResult<PathBuf> {
    let biomeos_dir = runtime_dir.join("biomeos");

    std::fs::create_dir_all(&biomeos_dir).map_err(|e| ServerError::Internal(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&biomeos_dir, perms)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
    }

    info!("✅ biomeos directory ensured: {}", biomeos_dir.display());
    Ok(biomeos_dir)
}

/// Get socket filename based on family ID.
///
/// Uses the **capability domain** stem per `PRIMAL_SELF_KNOWLEDGE_STANDARD.md`
/// v1.1 — sockets are named for what the primal *does*, not what it *is*.
///
/// - If family ID is "default" or not set: `compute.sock`
/// - If family ID is set and not "default": `compute-{family_id}.sock`
#[must_use]
pub fn socket_filename_for_family(family_id: &str) -> String {
    let domain = toadstool_common::constants::primal_identity::CAPABILITY_DOMAIN;
    if family_id.is_empty() || family_id == "default" {
        format!("{domain}.sock")
    } else {
        format!("{domain}-{family_id}.sock")
    }
}

/// tarpc socket filename (separate from JSON-RPC to avoid bind collision).
///
/// JSON-RPC is the primary protocol on `{domain}.sock`; tarpc uses
/// `{domain}-tarpc.sock` so both listeners can coexist.
#[must_use]
pub fn tarpc_socket_filename_for_family(family_id: &str) -> String {
    let domain = toadstool_common::constants::primal_identity::CAPABILITY_DOMAIN;
    if family_id.is_empty() || family_id == "default" {
        format!("{domain}-tarpc.sock")
    } else {
        format!("{domain}-{family_id}-tarpc.sock")
    }
}

/// Legacy primal-named socket filename (for backward-compatible symlink).
///
/// During migration from primal-named to domain-named sockets, a symlink
/// `toadstool.sock → compute.sock` is maintained for callers that haven't
/// updated to domain-based discovery.
#[must_use]
pub fn legacy_socket_filename_for_family(family_id: &str) -> String {
    let name = toadstool_common::constants::primal_identity::PRIMAL_NAME;
    if family_id.is_empty() || family_id == "default" {
        format!("{name}.sock")
    } else {
        format!("{name}-{family_id}.sock")
    }
}

/// Get socket path following biomeOS-standardized fallback
///
/// # Errors
///
/// Returns an error if biomeOS directory creation or permission setting fails.
///
/// Priority order:
/// 1. TOADSTOOL_SOCKET env var
/// 2. PRIMAL_SOCKET env var (with family suffix)
/// 3. BIOMEOS_SOCKET_PATH env var
/// 4. XDG runtime directory
/// 5. /tmp fallback
pub fn get_socket_path(family_id: &str, _node_id: &str) -> ServerResult<PathBuf> {
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("✅ Using socket path from TOADSTOOL_SOCKET: {}", socket);
        return Ok(PathBuf::from(socket));
    }

    if let Ok(socket) = std::env::var("PRIMAL_SOCKET") {
        let socket_with_family = format!("{socket}-{family_id}");
        info!(
            "✅ Using socket path from PRIMAL_SOCKET: {}",
            socket_with_family
        );
        return Ok(PathBuf::from(socket_with_family));
    }

    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        info!("✅ Using socket path from BIOMEOS_SOCKET_PATH: {}", socket);
        return Ok(PathBuf::from(socket));
    }

    let runtime_dir = if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime)
    } else if let Ok(uid_str) = std::fs::read_to_string("/proc/self/loginuid") {
        if let Ok(uid) = uid_str.trim().parse::<u32>() {
            PathBuf::from(format!("/run/user/{uid}"))
        } else {
            std::env::var("USER")
                .ok()
                .and_then(|user| {
                    std::fs::read_to_string("/etc/passwd")
                        .ok()
                        .and_then(|passwd| {
                            passwd
                                .lines()
                                .find(|line| line.starts_with(&format!("{user}:")))
                                .and_then(|line| {
                                    line.split(':')
                                        .nth(2)
                                        .and_then(|uid| uid.parse::<u32>().ok())
                                })
                                .map(|uid| PathBuf::from(format!("/run/user/{uid}")))
                        })
                })
                .unwrap_or_else(std::env::temp_dir)
        }
    } else {
        std::env::temp_dir()
    };

    if runtime_dir.exists() {
        let biomeos_dir = ensure_biomeos_directory(&runtime_dir)?;
        let socket_filename = socket_filename_for_family(family_id);
        let socket_path = biomeos_dir.join(socket_filename);
        info!(
            "✅ Using biomeOS standard socket path: {}",
            socket_path.display()
        );
        return Ok(socket_path);
    }

    let tmp_biomeos = std::env::temp_dir().join("biomeos");
    std::fs::create_dir_all(&tmp_biomeos).map_err(|e| ServerError::Internal(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&tmp_biomeos, perms)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
    }

    let socket_filename = socket_filename_for_family(family_id);
    let tmp_path = tmp_biomeos.join(socket_filename);
    info!("⚠️  Using /tmp fallback for dev/testing deployment");
    Ok(tmp_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_biomeos_directory_creates_in_runtime_dir() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let result = ensure_biomeos_directory(temp_dir.path());
        assert!(result.is_ok());
        let biomeos = result.unwrap();
        assert!(biomeos.ends_with("biomeos"));
        assert!(biomeos.exists());
        assert!(biomeos.is_dir());
    }

    #[test]
    fn socket_filename_for_family_empty() {
        assert_eq!(socket_filename_for_family(""), "compute.sock");
    }

    #[test]
    fn socket_filename_for_family_default() {
        assert_eq!(socket_filename_for_family("default"), "compute.sock");
    }

    #[test]
    fn socket_filename_for_family_custom() {
        assert_eq!(socket_filename_for_family("nat0"), "compute-nat0.sock");
    }

    #[test]
    fn tarpc_socket_filename_for_family_empty() {
        assert_eq!(tarpc_socket_filename_for_family(""), "compute-tarpc.sock");
    }

    #[test]
    fn tarpc_socket_filename_for_family_default() {
        assert_eq!(
            tarpc_socket_filename_for_family("default"),
            "compute-tarpc.sock"
        );
    }

    #[test]
    fn tarpc_socket_filename_for_family_custom() {
        assert_eq!(
            tarpc_socket_filename_for_family("nat0"),
            "compute-nat0-tarpc.sock"
        );
    }

    #[test]
    fn legacy_socket_filename_for_family_empty() {
        assert_eq!(legacy_socket_filename_for_family(""), "toadstool.sock");
    }

    #[test]
    fn legacy_socket_filename_for_family_custom() {
        assert_eq!(
            legacy_socket_filename_for_family("nat0"),
            "toadstool-nat0.sock"
        );
    }

    #[test]
    fn get_socket_path_tmp_fallback_when_xdg_not_exists() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_SOCKET", None::<&str>),
                ("PRIMAL_SOCKET", None::<&str>),
                ("BIOMEOS_SOCKET_PATH", None::<&str>),
                ("XDG_RUNTIME_DIR", Some("/nonexistent-path-12345-abcd")),
            ],
            || {
                let result = get_socket_path("custom", "node1");
                assert!(result.is_ok());
                let path = result.unwrap();
                assert!(path.ends_with("biomeos/compute-custom.sock"));
            },
        );
    }

    #[test]
    fn ensure_biomeos_directory_fails_when_parent_is_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let file_path = temp_dir.path().join("not_a_dir");
        std::fs::File::create(&file_path).expect("create file");

        let result = ensure_biomeos_directory(&file_path);
        assert!(
            result.is_err(),
            "create_dir_all on path with file parent should fail"
        );
    }

    #[test]
    fn get_socket_path_from_toadstool_socket_env() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("custom-toadstool.sock");
        let path_str = socket_path.to_string_lossy().to_string();
        temp_env::with_var("TOADSTOOL_SOCKET", Some(path_str.as_str()), || {
            let result = get_socket_path("any-family", "any-node");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), socket_path);
        });
    }

    #[test]
    fn get_socket_path_from_primal_socket_with_family_suffix() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_SOCKET", None::<&str>),
                ("BIOMEOS_SOCKET_PATH", None::<&str>),
                ("PRIMAL_SOCKET", Some("/run/primal")),
            ],
            || {
                let result = get_socket_path("family-x", "node1");
                assert!(result.is_ok());
                assert_eq!(
                    result.unwrap(),
                    std::path::PathBuf::from("/run/primal-family-x")
                );
            },
        );
    }

    #[test]
    fn get_socket_path_temp_dir_fallback_no_xdg() {
        temp_env::with_vars_unset(
            [
                "TOADSTOOL_SOCKET",
                "PRIMAL_SOCKET",
                "BIOMEOS_SOCKET_PATH",
                "XDG_RUNTIME_DIR",
            ],
            || {
                let result = get_socket_path("default", "node1");
                assert!(result.is_ok());
                let path = result.unwrap();
                assert!(path.ends_with("biomeos/compute.sock"));
            },
        );
    }
}
