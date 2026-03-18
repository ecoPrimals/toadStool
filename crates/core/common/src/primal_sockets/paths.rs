// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure socket path resolution (no env access - inject `SocketPathEnv`)

use std::path::PathBuf;

use super::env::SocketPathEnv;

pub(super) const FALLBACK_CRYPTO_SOCKET: &str = "beardog.sock";
pub(super) const FALLBACK_COORDINATION_SOCKET: &str = "songbird.sock";
pub(super) const FALLBACK_STORAGE_SOCKET: &str = "nestgate.sock";
pub(super) const FALLBACK_MCP_SOCKET: &str = "squirrel.sock";
pub(super) const SELF_SOCKET: &str = "toadstool.sock";

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

/// Pure logic: resolve beardog socket fallback (non-discovery path)
#[must_use]
pub fn resolve_beardog_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.beardog_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_CRYPTO_SOCKET)
}

/// Pure logic: resolve songbird socket fallback (non-discovery path)
#[must_use]
pub fn resolve_songbird_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.songbird_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_COORDINATION_SOCKET)
}

/// Pure logic: resolve nestgate socket fallback (non-discovery path)
#[must_use]
pub fn resolve_nestgate_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.nestgate_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_STORAGE_SOCKET)
}

/// Pure logic: resolve squirrel socket
#[must_use]
pub fn resolve_squirrel_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.squirrel_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_MCP_SOCKET)
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

/// Pure logic: resolve socket path for any service by name
#[must_use]
pub fn resolve_socket_path_for_service(
    service_name: &str,
    env: &SocketPathEnv,
    service_socket_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = service_socket_override {
        return path;
    }
    match service_name.to_lowercase().as_str() {
        "beardog" | "bear-dog" => resolve_beardog_socket_fallback(env),
        "songbird" | "song-bird" => resolve_songbird_socket_fallback(env),
        "nestgate" | "nest-gate" => resolve_nestgate_socket_fallback(env),
        "squirrel" => resolve_squirrel_socket(env),
        "toadstool" | "toad-stool" => resolve_toadstool_socket(env),
        "nucleus" | "biomeos" => resolve_nucleus_socket(env),
        _ => resolve_biomeos_dir(env).join(format!("{}.sock", service_name.to_lowercase())),
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
    fn test_resolve_beardog_socket_with_env_override() {
        let env = SocketPathEnv {
            beardog_socket: Some("/custom/beardog.sock".to_string()),
            ..test_env()
        };
        let path = resolve_beardog_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/beardog.sock"));
    }

    #[test]
    fn test_resolve_songbird_socket_with_env_override() {
        let env = SocketPathEnv {
            songbird_socket: Some("/custom/songbird.sock".to_string()),
            ..test_env()
        };
        let path = resolve_songbird_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/songbird.sock"));
    }

    #[test]
    fn test_resolve_nestgate_socket_with_env_override() {
        let env = SocketPathEnv {
            nestgate_socket: Some("/custom/nestgate.sock".to_string()),
            ..test_env()
        };
        let path = resolve_nestgate_socket_fallback(&env);
        assert_eq!(path, PathBuf::from("/custom/nestgate.sock"));
    }

    #[test]
    fn test_resolve_squirrel_socket_with_env_override() {
        let env = SocketPathEnv {
            squirrel_socket: Some("/custom/squirrel.sock".to_string()),
            ..test_env()
        };
        let path = resolve_squirrel_socket(&env);
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
    fn test_resolve_socket_path_service_aliases() {
        let env = test_env();
        let bear_dog_aliased = resolve_socket_path_for_service("bear-dog", &env, None);
        let beardog = resolve_socket_path_for_service("beardog", &env, None);
        assert_eq!(bear_dog_aliased, beardog);

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
}
