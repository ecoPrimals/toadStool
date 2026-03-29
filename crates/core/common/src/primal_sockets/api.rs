// SPDX-License-Identifier: AGPL-3.0-only
//! Public API - thin wrappers with single env snapshot at call site

use std::path::PathBuf;

#[allow(deprecated)]
use crate::constants::ecosystem::well_known::BIOMEOS;
use crate::constants::primal_identity::PRIMAL_NAME;

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

/// Get beardog (crypto) socket path. Prefer [`get_socket_path_for_capability`]("crypto").
#[deprecated(
    since = "0.2.0",
    note = "Use get_socket_path_for_capability(\"crypto\") or discover_crypto_socket().await"
)]
#[must_use]
pub fn get_beardog_socket_path() -> PathBuf {
    get_socket_path_for_capability("crypto")
}

/// Get songbird (coordination) socket path. Prefer [`get_socket_path_for_capability`]("coordination").
#[deprecated(
    since = "0.2.0",
    note = "Use get_socket_path_for_capability(\"coordination\") or discover_coordination_socket().await"
)]
#[must_use]
pub fn get_songbird_socket_path() -> PathBuf {
    get_socket_path_for_capability("coordination")
}

/// Get nestgate (storage) socket path. Prefer [`get_socket_path_for_capability`]("storage").
#[deprecated(
    since = "0.2.0",
    note = "Use get_socket_path_for_capability(\"storage\") or discover_storage_socket().await"
)]
#[must_use]
pub fn get_nestgate_socket_path() -> PathBuf {
    get_socket_path_for_capability("storage")
}

/// Get squirrel (AI) socket path.
#[allow(deprecated)]
#[must_use]
pub fn get_squirrel_socket_path() -> PathBuf {
    paths::resolve_routing_socket(&SocketPathEnv::from_env())
}

/// Get Nucleus (biomeOS) socket path.
#[must_use]
pub fn get_nucleus_socket_path() -> PathBuf {
    paths::resolve_nucleus_socket(&SocketPathEnv::from_env())
}

/// Get ToadStool main socket path.
#[must_use]
pub fn get_toadstool_socket_path() -> PathBuf {
    paths::resolve_toadstool_socket(&SocketPathEnv::from_env())
}

/// Resolve socket path by primal service name. Prefer [`get_socket_path_for_capability`] for capability-based discovery.
#[deprecated(
    since = "0.92.0",
    note = "Use `get_socket_path_for_capability()` for sovereignty-compliant discovery"
)]
#[allow(deprecated)]
pub fn get_socket_path_for_service(service_name: &str) -> PathBuf {
    let normalized = service_name.to_lowercase();
    let s = normalized.as_str();

    if let Some(cap) = paths::service_label_to_capability_id(s) {
        return get_socket_path_for_capability(cap);
    }

    if s == PRIMAL_NAME || s == "toad-stool" {
        return get_toadstool_socket_path();
    }
    if s == "nucleus" || s == BIOMEOS {
        return get_nucleus_socket_path();
    }

    let env = SocketPathEnv::from_env();
    let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
    let override_path = std::env::var(&env_var).ok().map(PathBuf::from);
    paths::resolve_socket_path_for_service(service_name, &env, override_path)
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
                dir.contains("toadstool-runtime") || dir.starts_with("/run/user/"),
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
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_biomeos_alias() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("biomeos");
            assert!(path.to_string_lossy().contains("nucleus"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_song_bird_alias() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("song-bird");
            assert!(path.to_string_lossy().contains("coordination"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_nest_gate_alias() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("nest-gate");
            assert!(path.to_string_lossy().contains("storage"));
        });
    }

    // ═══════════════════════════════════════════════════════════════════
    // Additional tests for socket path functions and error handling
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    #[allow(deprecated)]
    fn test_get_beardog_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/9999"), || {
            temp_env::with_var_unset("BEARDOG_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_CRYPTO_SOCKET", || {
                    let path = get_beardog_socket_path();
                    assert!(path.to_string_lossy().contains("crypto"));
                    assert!(path.to_string_lossy().contains("biomeos"));
                });
            });
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_songbird_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/8888"), || {
            temp_env::with_var_unset("SONGBIRD_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_COORDINATION_SOCKET", || {
                    let path = get_songbird_socket_path();
                    assert!(path.to_string_lossy().contains("coordination"));
                });
            });
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_nestgate_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/7777"), || {
            temp_env::with_var_unset("NESTGATE_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_STORAGE_SOCKET", || {
                    let path = get_nestgate_socket_path();
                    assert!(path.to_string_lossy().contains("storage"));
                });
            });
        });
    }

    #[test]
    fn test_get_squirrel_socket_path_with_xdg() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/squirrel-runtime"), || {
            temp_env::with_var_unset("SQUIRREL_SOCKET", || {
                temp_env::with_var_unset("BIOMEOS_ROUTING_SOCKET", || {
                    let path = get_squirrel_socket_path();
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
                    assert!(path.to_string_lossy().contains("toadstool"));
                });
            });
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_beardog() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("beardog");
            assert!(path.to_string_lossy().contains("crypto"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_bear_dog_alias() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("bear-dog");
            assert!(path.to_string_lossy().contains("crypto"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_squirrel() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("squirrel");
            assert!(path.to_string_lossy().contains("routing"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_toadstool() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("toadstool");
            assert!(path.to_string_lossy().contains("toadstool"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_toad_stool_alias() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            let path = get_socket_path_for_service("toad-stool");
            assert!(path.to_string_lossy().contains("toadstool"));
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_custom_with_env_override() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            temp_env::with_var("CUSTOM_SVC_SOCKET", Some("/custom/path.sock"), || {
                let path = get_socket_path_for_service("custom-svc");
                assert_eq!(path, PathBuf::from("/custom/path.sock"));
            });
        });
    }

    #[test]
    #[allow(deprecated)]
    fn test_get_socket_path_for_service_unknown_service_uses_convention() {
        temp_env::with_var("XDG_RUNTIME_DIR", Some("/run/user/1000"), || {
            temp_env::with_var_unset("UNKNOWN_SVC_SOCKET", || {
                let path = get_socket_path_for_service("unknown-svc");
                assert!(path.to_string_lossy().contains("unknown-svc"));
                assert!(path.to_string_lossy().ends_with(".sock"));
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
    fn test_get_socket_path_for_capability_routing() {
        temp_env::with_var_unset("BIOMEOS_ROUTING_SOCKET", || {
            temp_env::with_var("XDG_RUNTIME_DIR", Some("/tmp/test-rt"), || {
                let path = get_socket_path_for_capability("routing");
                assert_eq!(path, PathBuf::from("/tmp/test-rt/biomeos/routing.sock"));
            });
        });
    }
}
