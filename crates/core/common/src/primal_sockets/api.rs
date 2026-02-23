//! Public API - thin wrappers with single env snapshot at call site

use std::path::PathBuf;

use super::discovery;
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

#[deprecated(
    since = "0.2.0",
    note = "Use discover_crypto_socket() for capability-based discovery. See docs for migration."
)]
#[must_use]
pub fn get_beardog_socket_path() -> PathBuf {
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discovery::discover_crypto_socket()).ok()
    })
    .join()
    .ok()
    .flatten();
    if let Some(path) = discovery_result {
        return path;
    }
    paths::resolve_beardog_socket_fallback(&SocketPathEnv::from_env())
}

#[deprecated(
    since = "0.2.0",
    note = "Use discover_coordination_socket() for capability-based discovery. See docs for migration."
)]
#[must_use]
pub fn get_songbird_socket_path() -> PathBuf {
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discovery::discover_coordination_socket()).ok()
    })
    .join()
    .ok()
    .flatten();
    if let Some(path) = discovery_result {
        return path;
    }
    paths::resolve_songbird_socket_fallback(&SocketPathEnv::from_env())
}

#[deprecated(
    since = "0.2.0",
    note = "Use discover_storage_socket() for capability-based discovery. See docs for migration."
)]
#[must_use]
pub fn get_nestgate_socket_path() -> PathBuf {
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discovery::discover_storage_socket()).ok()
    })
    .join()
    .ok()
    .flatten();
    if let Some(path) = discovery_result {
        return path;
    }
    paths::resolve_nestgate_socket_fallback(&SocketPathEnv::from_env())
}

#[allow(deprecated)]
#[must_use]
pub fn get_squirrel_socket_path() -> PathBuf {
    paths::resolve_squirrel_socket(&SocketPathEnv::from_env())
}

#[must_use]
pub fn get_nucleus_socket_path() -> PathBuf {
    paths::resolve_nucleus_socket(&SocketPathEnv::from_env())
}

#[must_use]
pub fn get_toadstool_socket_path() -> PathBuf {
    paths::resolve_toadstool_socket(&SocketPathEnv::from_env())
}

#[allow(deprecated)]
pub fn get_socket_path_for_service(service_name: &str) -> PathBuf {
    match service_name.to_lowercase().as_str() {
        s if s == "beardog" || s == "bear-dog" => get_beardog_socket_path(),
        s if s == "songbird" || s == "song-bird" => get_songbird_socket_path(),
        s if s == "nestgate" || s == "nest-gate" => get_nestgate_socket_path(),
        s if s == "squirrel" => get_squirrel_socket_path(),
        s if s == "toadstool" || s == "toad-stool" => get_toadstool_socket_path(),
        "nucleus" | "biomeos" => get_nucleus_socket_path(),
        _ => {
            let env = SocketPathEnv::from_env();
            let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
            let override_path = std::env::var(&env_var).ok().map(PathBuf::from);
            paths::resolve_socket_path_for_service(service_name, &env, override_path)
        }
    }
}
