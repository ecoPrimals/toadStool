//! Pure socket path resolution (no env access - inject SocketPathEnv)

use std::path::PathBuf;

#[allow(deprecated)]
use crate::interned_strings::primals;

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
        .join(format!("toadstool-runtime-{}", username))
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
#[allow(deprecated)]
pub fn resolve_socket_path_for_service(
    service_name: &str,
    env: &SocketPathEnv,
    service_socket_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = service_socket_override {
        return path;
    }
    match service_name.to_lowercase().as_str() {
        s if s == primals::BEARDOG || s == "bear-dog" => resolve_beardog_socket_fallback(env),
        s if s == primals::SONGBIRD || s == "song-bird" => resolve_songbird_socket_fallback(env),
        s if s == primals::NESTGATE || s == "nest-gate" => resolve_nestgate_socket_fallback(env),
        s if s == primals::SQUIRREL => resolve_squirrel_socket(env),
        s if s == primals::TOADSTOOL || s == "toad-stool" => resolve_toadstool_socket(env),
        "nucleus" | "biomeos" => resolve_nucleus_socket(env),
        _ => resolve_biomeos_dir(env).join(format!("{}.sock", service_name.to_lowercase())),
    }
}
