// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public API - thin wrappers with single env snapshot at call site

use std::path::PathBuf;

use super::env::SocketPathEnv;
use super::paths;

/// Get runtime directory for socket files
#[must_use]
pub fn get_runtime_dir() -> String {
    paths::resolve_runtime_dir(&SocketPathEnv::from_env())
}

/// Get biomeos directory path
#[must_use]
pub fn get_biomeos_dir() -> PathBuf {
    paths::resolve_biomeos_dir(&SocketPathEnv::from_env())
}

/// Ensure biomeos directory exists with proper permissions
///
/// # Errors
///
/// Returns [`std::io::Error`] if directory creation or permission setting fails.
pub fn ensure_biomeos_dir() -> std::io::Result<PathBuf> {
    let biomeos_dir = get_biomeos_dir();
    std::fs::create_dir_all(&biomeos_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&biomeos_dir, perms)?;
    }
    Ok(biomeos_dir)
}

/// Get family ID from environment
#[must_use]
pub fn get_family_id() -> String {
    paths::resolve_family_id(&SocketPathEnv::from_env())
}

/// BTSP insecure guard: refuse if `FAMILY_ID` + `BIOMEOS_INSECURE=1`.
///
/// Call at server startup before binding sockets.
///
/// # Errors
///
/// Returns a description of the security conflict.
pub fn check_insecure_guard() -> Result<(), String> {
    paths::validate_insecure_guard(&SocketPathEnv::from_env())
}

/// Get routing (AI / intelligence) socket path.
#[must_use]
pub fn get_routing_socket_path() -> PathBuf {
    paths::resolve_routing_socket(&SocketPathEnv::from_env())
}

/// Get Nucleus (biomeOS) socket path.
#[must_use]
pub fn get_nucleus_socket_path() -> PathBuf {
    paths::resolve_nucleus_socket(&SocketPathEnv::from_env())
}

/// Get ToadStool main (JSON-RPC) socket path.
#[must_use]
pub fn get_toadstool_socket_path() -> PathBuf {
    paths::resolve_toadstool_socket(&SocketPathEnv::from_env())
}

/// Get ToadStool tarpc (hot-path) socket path.
///
/// tarpc uses a separate socket from JSON-RPC to avoid bind collision.
/// Convention: `compute-tarpc.sock` or `compute-{family_id}-tarpc.sock`.
/// Override via `TOADSTOOL_TARPC_SOCKET`.
#[must_use]
pub fn get_toadstool_tarpc_socket_path() -> PathBuf {
    paths::resolve_toadstool_tarpc_socket(&SocketPathEnv::from_env())
}

/// Resolve socket path by capability rather than primal name.
///
/// This is the sovereignty-compliant API: ToadStool discovers peers by
/// what they *do*, not who they *are*. The actual socket path is resolved
/// from the environment variable `BIOMEOS_{CAPABILITY}_SOCKET`, falling back
/// to a conventional path under the biomeos runtime directory.
///
/// Supported capabilities: `coordination`, `crypto`, `storage`, `routing`, `compute`, …
#[must_use]
pub fn get_socket_path_for_capability(capability: &str) -> PathBuf {
    paths::resolve_capability_socket_fallback(capability, &SocketPathEnv::from_env())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_runtime_dir_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1234"), || {
            let dir = get_runtime_dir();
            assert_eq!(dir, "/run/user/1234");
        });
    }

    #[test]
    fn test_get_runtime_dir_fallback_without_xdg() {
        temp_env::with_var_unset("XDG_RUNTIME_DIR", || {
            let dir = get_runtime_dir();
            assert!(!dir.is_empty());
            assert!(
                dir.contains("toadstool-runtime")
                    || dir.starts_with("/run/user/")
                    || dir.starts_with("/run/membrane/"),
                "expected runtime dir pattern, got: {dir}"
            );
        });
    }

    #[test]
    fn test_get_biomeos_dir_contains_biomeos() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-runtime"), || {
            let dir = get_biomeos_dir();
            assert!(dir.ends_with("biomeos"));
            assert_eq!(dir, PathBuf::from("/tmp/test-runtime/biomeos"));
        });
    }

    #[test]
    fn test_get_family_id_from_env() {
        temp_env::with_var("BIOMEOS_FAMILY_ID", Some("my-family"), || {
            let id = get_family_id();
            assert_eq!(id, "my-family");
        });
    }

    #[test]
    fn test_get_family_id_default() {
        temp_env::with_var_unset("BIOMEOS_FAMILY_ID", || {
            temp_env::with_var_unset("TOADSTOOL_FAMILY", || {
                let id = get_family_id();
                assert_eq!(id, "default");
            });
        });
    }

    #[test]
    fn test_ensure_biomeos_dir_creates_dir_with_temp_env() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path_str = temp_dir.path().to_string_lossy().into_owned();

        let result = temp_env::with_var("XDG_RUNTIME_DIR", Some(path_str.as_str()), || {
            ensure_biomeos_dir()
        });

        assert!(result.is_ok());
        let biomeos_path = result.unwrap();
        assert!(biomeos_path.exists());
        assert!(biomeos_path.is_dir());
        assert_eq!(biomeos_path.file_name().unwrap(), "biomeos");
    }

    #[test]
    fn test_get_routing_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/routing-runtime"), || {
            temp_env::with_var_unset("SQUIRREL_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_ROUTING_SOCKET", || {
                    let path = get_routing_socket_path();
                    assert!(path.to_string_lossy().contains("routing"));
                    assert!(path.ends_with("routing.sock"));
                });
            });
        });
    }

    #[test]
    fn test_get_nucleus_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/nucleus-runtime"), || {
            temp_env::with_var_unset("NUCLEUS_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_SOCKET_PATH", || {
                    let path = get_nucleus_socket_path();
                    assert!(path.to_string_lossy().contains("nucleus"));
                });
            });
        });
    }

    #[test]
    fn test_get_toadstool_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/toadstool-runtime"), || {
            temp_env::with_var_unset("TOADSTOOL_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_SOCKET_PATH", || {
                    let path = get_toadstool_socket_path();
                    assert!(
                        path.to_string_lossy().ends_with("compute.sock"),
                        "Self-Knowledge v1.1: domain-based name, got: {}",
                        path.display()
                    );
                });
            });
        });
    }

    #[test]
    fn test_get_biomeos_dir_uses_xdg_runtime() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/12345"), || {
            let dir = get_biomeos_dir();
            assert_eq!(dir, PathBuf::from("/run/user/12345/biomeos"));
        });
    }

    #[test]
    fn test_ensure_biomeos_dir_idempotent() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let path_str = temp_dir.path().to_string_lossy().into_owned();

        let result1 = temp_env::with_var("XDG_RUNTIME_DIR", Some(path_str.as_str()), || {
            ensure_biomeos_dir()
        });
        assert!(result1.is_ok());

        let result2 = temp_env::with_var("XDG_RUNTIME_DIR", Some(path_str.as_str()), || {
            ensure_biomeos_dir()
        });
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[test]
    fn test_get_socket_path_for_capability_from_env() {
        temp_env::with_var("BIOMEOS_CRYPTO_SOCKET", Some("/tmp/crypto.sock"), || {
            let path = get_socket_path_for_capability("crypto");
            assert_eq!(path, PathBuf::from("/tmp/crypto.sock"));
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_fallback() {
        temp_env::with_var_unset("BIOMEOS_STORAGE_SOCKET", || {
            temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-rt"), || {
                let path = get_socket_path_for_capability("storage");
                assert_eq!(path, PathBuf::from("/tmp/test-rt/biomeos/storage.sock"));
            });
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_coordination() {
        temp_env::with_var_unset("BIOMEOS_COORDINATION_SOCKET", || {
            temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-rt"), || {
                let path = get_socket_path_for_capability("coordination");
                assert_eq!(
                    path,
                    PathBuf::from("/tmp/test-rt/biomeos/coordination.sock")
                );
            });
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_crypto_with_xdg_unset_env() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/9999"), || {
            temp_env::with_var_unset("BEARDOG_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_CRYPTO_SOCKET", || {
                    let path = get_socket_path_for_capability("crypto");
                    assert!(path.to_string_lossy().contains("crypto"));
                    assert!(path.to_string_lossy().contains("biomeos"));
                });
            });
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_coordination_with_xdg_unset_env() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/8888"), || {
            temp_env::with_var_unset("SONGBIRD_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_COORDINATION_SOCKET", || {
                    let path = get_socket_path_for_capability("coordination");
                    assert!(path.to_string_lossy().contains("coordination"));
                });
            });
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_storage_with_xdg_unset_env() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/7777"), || {
            temp_env::with_var_unset("NESTGATE_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_STORAGE_SOCKET", || {
                    let path = get_socket_path_for_capability("storage");
                    assert!(path.to_string_lossy().contains("storage"));
                });
            });
        });
    }

    #[test]
    fn test_get_socket_path_for_capability_routing() {
        temp_env::with_var_unset("BIOMEOS_ROUTING_SOCKET", || {
            temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-rt"), || {
                let path = get_socket_path_for_capability("routing");
                assert_eq!(path, PathBuf::from("/tmp/test-rt/biomeos/routing.sock"));
            });
        });
    }
}
