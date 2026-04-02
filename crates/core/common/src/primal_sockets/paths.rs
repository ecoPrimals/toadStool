// SPDX-License-Identifier: AGPL-3.0-only
//! Pure socket path resolution (no env access - inject `SocketPathEnv`)

use std::path::PathBuf;

use super::env::SocketPathEnv;
use crate::interned_strings::CapabilityDomain;

pub(super) const SELF_SOCKET: &str = "toadstool.sock";

/// Map a *peer* service label (legacy primal name or capability ID) to a canonical capability id.
///
/// Self-names ("toadstool", "toad-stool") and platform names ("nucleus", "biomeos")
/// are intentionally excluded — they have dedicated socket resolution paths.
#[must_use]
pub fn service_label_to_capability_id(label: &str) -> Option<&'static str> {
    let lower = label.to_ascii_lowercase();
    match lower.as_str() {
        "toadstool" | "toad-stool" | "nucleus" | "biomeos" => None,
        _ => CapabilityDomain::from_label(&lower).map(CapabilityDomain::as_str),
    }
}

/// Pure logic: resolve runtime dir from environment snapshot
#[must_use]
pub fn resolve_runtime_dir(env: &SocketPathEnv) -> String {
    if let Some(ref xdg) = env.xdg_runtime_dir {
        return xdg.clone();
    }
    let temp_dir = std::env::temp_dir();
    let username = env.user.as_deref().unwrap_or("default");
    temp_dir
        .join(format!("toadstool-runtime-{username}"))
        .to_string_lossy()
        .to_string()
}

/// Pure logic: resolve biomeos dir
#[must_use]
pub fn resolve_biomeos_dir(env: &SocketPathEnv) -> PathBuf {
    PathBuf::from(resolve_runtime_dir(env)).join("biomeos")
}

/// Pure logic: resolve family ID from environment snapshot
#[must_use]
pub fn resolve_family_id(env: &SocketPathEnv) -> String {
    env.biomeos_family_id
        .clone()
        .unwrap_or_else(|| "default".to_string())
}

/// Pure logic: resolve socket path for a capability id (`crypto`, `coordination`, …).
///
/// Precedence: `BIOMEOS_{CAP}_SOCKET` → legacy `BEARDOG_SOCKET` / `SONGBIRD_SOCKET` / … →
/// `{capability}.sock` under the biomeOS runtime directory (never primal-name filenames).
#[must_use]
pub fn resolve_capability_socket_fallback(capability: &str, env: &SocketPathEnv) -> PathBuf {
    let cap = capability.to_lowercase();
    let cap = cap.as_str();

    if let Some(p) = match cap {
        "crypto" | "security" => env.biomeos_crypto_socket.as_ref(),
        "coordination" => env.biomeos_coordination_socket.as_ref(),
        "storage" => env.biomeos_storage_socket.as_ref(),
        "routing" | "intelligence" | "ai" => env.biomeos_routing_socket.as_ref(),
        _ => None,
    } {
        return PathBuf::from(p);
    }

    if let Some(p) = match cap {
        "crypto" | "security" => env.beardog_socket.as_ref(),
        "coordination" => env.songbird_socket.as_ref(),
        "storage" => env.nestgate_socket.as_ref(),
        "routing" | "intelligence" | "ai" => env.squirrel_socket.as_ref(),
        _ => None,
    } {
        return PathBuf::from(p);
    }

    resolve_biomeos_dir(env).join(format!("{cap}.sock"))
}

/// Pure logic: resolve crypto capability socket fallback (non-discovery path).
#[deprecated(
    since = "0.92.0",
    note = "Use resolve_capability_socket_fallback(\"crypto\", env)"
)]
#[must_use]
pub fn resolve_beardog_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("crypto", env)
}

/// Pure logic: resolve coordination capability socket fallback (non-discovery path).
#[deprecated(
    since = "0.92.0",
    note = "Use resolve_capability_socket_fallback(\"coordination\", env)"
)]
#[must_use]
pub fn resolve_songbird_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("coordination", env)
}

/// Pure logic: resolve storage capability socket fallback (non-discovery path).
#[deprecated(
    since = "0.92.0",
    note = "Use resolve_capability_socket_fallback(\"storage\", env)"
)]
#[must_use]
pub fn resolve_nestgate_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("storage", env)
}

/// Pure logic: resolve routing capability socket (non-discovery path).
#[must_use]
pub fn resolve_routing_socket(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("routing", env)
}

/// Pure logic: resolve legacy “squirrel” routing socket — same as [`resolve_routing_socket`].
#[deprecated(
    since = "0.92.0",
    note = "Use resolve_routing_socket(env) or resolve_capability_socket_fallback(\"routing\", env)"
)]
#[must_use]
pub fn resolve_squirrel_socket(env: &SocketPathEnv) -> PathBuf {
    resolve_routing_socket(env)
}

/// Pure logic: resolve nucleus socket
#[must_use]
pub fn resolve_nucleus_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.nucleus_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join("nucleus.sock")
}

/// Pure logic: resolve toadstool socket
#[must_use]
pub fn resolve_toadstool_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.toadstool_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(SELF_SOCKET)
}

/// Pure logic: resolve socket path for any service label (capability id or legacy primal alias).
#[must_use]
pub fn resolve_socket_path_for_service(
    service_name: &str,
    env: &SocketPathEnv,
    service_socket_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = service_socket_override {
        return path;
    }
    let lower = service_name.to_lowercase();
    let s = lower.as_str();

    if let Some(cap) = service_label_to_capability_id(s) {
        return resolve_capability_socket_fallback(cap, env);
    }

    match s {
        "toadstool" | "toad-stool" => resolve_toadstool_socket(env),
        "nucleus" | "biomeos" => resolve_nucleus_socket(env),
        _ => resolve_biomeos_dir(env).join(format!("{s}.sock")),
    }
}

#[cfg(test)]
mod tests {
    use super::super::env::SocketPathEnv;
    use super::*;

    fn test_env() -> SocketPathEnv {
        SocketPathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            user: Some("testuser".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_resolve_runtime_dir_xdg_takes_precedence() {
        let env = test_env();
        assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
    }

    #[test]
    fn test_resolve_runtime_dir_no_xdg_uses_temp_and_user() {
        let env = SocketPathEnv {
            xdg_runtime_dir: None,
            user: Some("alice".to_string()),
            ..Default::default()
        };
        let dir = resolve_runtime_dir(&env);
        assert!(dir.contains("toadstool-runtime-alice"));
    }

    #[test]
    fn test_resolve_runtime_dir_no_user_uses_default() {
        let env = SocketPathEnv {
            xdg_runtime_dir: None,
            user: None,
            ..Default::default()
        };
        let dir = resolve_runtime_dir(&env);
        assert!(dir.contains("toadstool-runtime-default"));
    }

    #[test]
    fn test_resolve_nucleus_socket_prefers_nucleus_over_biomeos_path() {
        let env = SocketPathEnv {
            nucleus_socket: Some("/custom/nucleus.sock".to_string()),
            biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
            ..test_env()
        };
        let path = resolve_nucleus_socket(&env);
        assert_eq!(path, PathBuf::from("/custom/nucleus.sock"));
    }

    #[test]
    fn test_resolve_toadstool_socket_prefers_toadstool_over_biomeos_path() {
        let env = SocketPathEnv {
            toadstool_socket: Some("/custom/toad.sock".to_string()),
            biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
            ..test_env()
        };
        let path = resolve_toadstool_socket(&env);
        assert_eq!(path, PathBuf::from("/custom/toad.sock"));
    }

    #[test]
    fn test_resolve_socket_path_override_takes_precedence() {
        let env = test_env();
        let override_path = PathBuf::from("/override/custom.sock");
        let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
        assert_eq!(result, override_path);
    }

    #[test]
    #[allow(deprecated)]
    fn test_resolve_beardog_socket_with_env_override() {
        let env = SocketPathEnv {
            beardog_socket: Some("/custom/beardog.sock".to_string()),
            ..test_env()
        };
        let path = resolve_beardog_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/beardog.sock"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_resolve_songbird_socket_with_env_override() {
        let env = SocketPathEnv {
            songbird_socket: Some("/custom/songbird.sock".to_string()),
            ..test_env()
        };
        let path = resolve_songbird_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/songbird.sock"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_resolve_nestgate_socket_with_env_override() {
        let env = SocketPathEnv {
            nestgate_socket: Some("/custom/nestgate.sock".to_string()),
            ..test_env()
        };
        let path = resolve_nestgate_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/nestgate.sock"));
    }

    #[test]
    fn test_resolve_routing_socket_with_env_override() {
        let env = SocketPathEnv {
            squirrel_socket: Some("/custom/squirrel.sock".to_string()),
            ..test_env()
        };
        let path = resolve_routing_socket(&env);
        assert_eq!(path, PathBuf::from("/custom/squirrel.sock"));
    }

    #[test]
    fn test_resolve_nucleus_socket_uses_biomeos_fallback() {
        let env = SocketPathEnv {
            nucleus_socket: None,
            biomeos_socket_path: None,
            ..test_env()
        };
        let path = resolve_nucleus_socket(&env);
        assert!(path.to_string_lossy().contains("nucleus"));
        assert!(path.to_string_lossy().ends_with("nucleus.sock"));
    }

    #[test]
    fn test_resolve_toadstool_socket_uses_biomeos_fallback() {
        let env = SocketPathEnv {
            toadstool_socket: None,
            biomeos_socket_path: None,
            ..test_env()
        };
        let path = resolve_toadstool_socket(&env);
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().ends_with("toadstool.sock"));
    }

    #[test]
    fn test_resolve_socket_path_unknown_service() {
        let env = test_env();
        let path = resolve_socket_path_for_service("unknown_service", &env, None);
        assert!(path.to_string_lossy().contains("unknown_service"));
        assert!(path.to_string_lossy().ends_with("unknown_service.sock"));
    }

    #[test]
    fn test_resolve_socket_path_capability_and_legacy_aliases() {
        let env = test_env();
        let bear_dog_aliased = resolve_socket_path_for_service("bear-dog", &env, None);
        let crypto = resolve_socket_path_for_service("crypto", &env, None);
        assert_eq!(bear_dog_aliased, crypto);
        assert!(crypto.to_string_lossy().ends_with("crypto.sock"));

        let path = resolve_socket_path_for_service("toad-stool", &env, None);
        assert!(path.to_string_lossy().contains("toadstool"));
    }

    #[test]
    fn test_resolve_biomeos_dir() {
        let env = test_env();
        let path = resolve_biomeos_dir(&env);
        assert!(path.to_string_lossy().contains("biomeos"));
    }

    #[test]
    fn test_resolve_family_id_default() {
        let env = SocketPathEnv {
            biomeos_family_id: None,
            ..test_env()
        };
        assert_eq!(resolve_family_id(&env), "default");
    }

    #[test]
    fn test_resolve_family_id_from_env() {
        let env = SocketPathEnv {
            biomeos_family_id: Some("custom-family".to_string()),
            ..test_env()
        };
        assert_eq!(resolve_family_id(&env), "custom-family");
    }

    #[test]
    fn test_biomeos_crypto_socket_precedence_over_legacy() {
        let env = SocketPathEnv {
            biomeos_crypto_socket: Some("/via/biomeos/crypto.sock".to_string()),
            beardog_socket: Some("/legacy/beardog.sock".to_string()),
            ..test_env()
        };
        let path = resolve_capability_socket_fallback("crypto", &env);
        assert_eq!(path, PathBuf::from("/via/biomeos/crypto.sock"));
    }
}
