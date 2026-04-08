// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// BTSP Protocol Standard §Compliance: validate insecure guard.
///
/// When `FAMILY_ID` is set (non-"default") **and** `BIOMEOS_INSECURE=1`,
/// the primal **must refuse to start**. This combination is contradictory:
/// `FAMILY_ID` activates production-mode BTSP handshake, while
/// `BIOMEOS_INSECURE` disables all security. Allowing both would create
/// a false sense of security.
///
/// Returns `Ok(())` if the environment is consistent, or `Err(message)`
/// describing the conflict.
pub fn validate_insecure_guard(env: &SocketPathEnv) -> Result<(), String> {
    let family_id = resolve_family_id(env);
    let is_production_family = family_id != "default";

    let is_insecure = env
        .biomeos_insecure
        .as_deref()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    if is_production_family && is_insecure {
        return Err(format!(
            "BTSP security conflict: FAMILY_ID={family_id:?} (production) \
             with BIOMEOS_INSECURE=1 (development). These are mutually exclusive. \
             Either unset BIOMEOS_INSECURE for production, or unset FAMILY_ID for development. \
             See BTSP_PROTOCOL_STANDARD.md §Compliance Checklist."
        ));
    }

    Ok(())
}

/// Returns `true` when `FAMILY_ID` is set to a non-default value,
/// indicating BTSP handshake is expected on incoming connections.
#[must_use]
pub fn is_btsp_required(env: &SocketPathEnv) -> bool {
    let family_id = resolve_family_id(env);
    family_id != "default"
}

/// Pure logic: resolve socket path for a capability id (`crypto`, `coordination`, …).
///
/// Precedence: `BIOMEOS_{CAP}_SOCKET` → legacy env fallbacks (`BEARDOG_SOCKET`, `SONGBIRD_SOCKET`, …) →
/// `{capability}.sock` under the biomeOS runtime directory (never product-code filenames).
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

    // Legacy env field names map crypto→security, coordination, storage, intelligence sockets.
    if let Some(p) = match cap {
        "crypto" | "security" => env.legacy_security_socket.as_ref(), // legacy: BEARDOG_SOCKET
        "coordination" => env.legacy_coordination_socket.as_ref(),    // legacy: SONGBIRD_SOCKET
        "storage" => env.legacy_storage_socket.as_ref(),              // legacy: NESTGATE_SOCKET
        "routing" | "intelligence" | "ai" => env.legacy_intelligence_socket.as_ref(), // legacy: SQUIRREL_SOCKET
        _ => None,
    } {
        return PathBuf::from(p);
    }

    resolve_biomeos_dir(env).join(format!("{cap}.sock"))
}

/// Pure logic: resolve routing capability socket (non-discovery path).
#[must_use]
pub fn resolve_routing_socket(env: &SocketPathEnv) -> PathBuf {
    resolve_capability_socket_fallback("routing", env)
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
        // legacy orchestrator label still resolves via CapabilityDomain
        let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
        assert_eq!(result, override_path);
    }

    #[test]
    fn test_resolve_crypto_socket_with_legacy_env_override() {
        let env = SocketPathEnv {
            legacy_security_socket: Some("/custom/crypto-via-legacy.sock".to_string()),
            ..test_env()
        };
        let path = resolve_capability_socket_fallback("crypto", &env);
        assert_eq!(path, PathBuf::from("/custom/crypto-via-legacy.sock"));
    }

    #[test]
    fn test_resolve_coordination_socket_with_legacy_env_override() {
        let env = SocketPathEnv {
            legacy_coordination_socket: Some("/custom/coordination-via-legacy.sock".to_string()),
            ..test_env()
        };
        let path = resolve_capability_socket_fallback("coordination", &env);
        assert_eq!(path, PathBuf::from("/custom/coordination-via-legacy.sock"));
    }

    #[test]
    fn test_resolve_storage_socket_with_legacy_env_override() {
        let env = SocketPathEnv {
            legacy_storage_socket: Some("/custom/storage-via-legacy.sock".to_string()),
            ..test_env()
        };
        let path = resolve_capability_socket_fallback("storage", &env);
        assert_eq!(path, PathBuf::from("/custom/storage-via-legacy.sock"));
    }

    #[test]
    fn test_resolve_routing_socket_with_legacy_env_override() {
        let env = SocketPathEnv {
            legacy_intelligence_socket: Some("/custom/intelligence-via-legacy.sock".to_string()),
            ..test_env()
        };
        let path = resolve_routing_socket(&env);
        assert_eq!(path, PathBuf::from("/custom/intelligence-via-legacy.sock"));
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
            legacy_security_socket: Some("/legacy/crypto-fallback.sock".to_string()),
            ..test_env()
        };
        let path = resolve_capability_socket_fallback("crypto", &env);
        assert_eq!(path, PathBuf::from("/via/biomeos/crypto.sock"));
    }
}
