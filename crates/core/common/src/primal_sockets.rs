//! # Primal Unix Socket Discovery
//!
//! Pure Rust unix socket path resolution for primal-to-primal communication.
//!
//! ## TRUE PRIMAL Architecture
//!
//! - **No HTTP**: All primal communication via unix sockets
//! - **Songbird Handles External**: Only Songbird uses HTTP/TLS for external
//! - **Local IPC**: Fast, secure, pure Rust
//! - **Discovery-Based**: Socket paths from environment/runtime

use std::path::PathBuf;

/// Get runtime directory for socket files
///
/// Priority:
/// 1. XDG_RUNTIME_DIR environment variable
/// 2. /tmp with username fallback
///
/// **TRUE PRIMAL**: Environment-based, no hardcoding
pub fn get_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        format!("/tmp/toadstool-runtime-{}", username)
    })
}

/// Get family ID from environment
///
/// Priority:
/// 1. BIOMEOS_FAMILY_ID (orchestrator-provided)
/// 2. TOADSTOOL_FAMILY (instance-specific)
/// 3. "default" fallback
pub fn get_family_id() -> String {
    std::env::var("BIOMEOS_FAMILY_ID")
        .or_else(|_| std::env::var("TOADSTOOL_FAMILY"))
        .unwrap_or_else(|_| "default".to_string())
}

/// Get BearDog unix socket path
///
/// **DEPRECATED**: Use capability-based discovery + `get_socket_path_for_service("beardog")` instead
///
/// This function violates TRUE PRIMAL self-knowledge principle by hardcoding "beardog".
/// Prefer: `StorageClient::discover()` or `get_socket_path_for_service(discovered_name)`
///
/// Priority:
/// 1. BEARDOG_SOCKET environment variable (absolute path)
/// 2. Runtime directory + family: `{runtime_dir}/beardog-{family}.sock`
#[deprecated(
    since = "4.9.0",
    note = "Use capability-based discovery + get_socket_path_for_service() instead. This violates TRUE PRIMAL self-knowledge."
)]
pub fn get_beardog_socket_path() -> PathBuf {
    // Priority 1: Direct socket path
    if let Ok(socket) = std::env::var("BEARDOG_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: Runtime directory with family
    let runtime_dir = get_runtime_dir();
    let family = get_family_id();
    PathBuf::from(&runtime_dir).join(format!("beardog-{}.sock", family))
}

/// Get Songbird unix socket path
///
/// Priority:
/// 1. SONGBIRD_SOCKET environment variable (absolute path)
/// 2. Runtime directory + family: `{runtime_dir}/songbird-{family}.sock`
pub fn get_songbird_socket_path() -> PathBuf {
    // Priority 1: Direct socket path
    if let Ok(socket) = std::env::var("SONGBIRD_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: Runtime directory with family
    let runtime_dir = get_runtime_dir();
    let family = get_family_id();
    PathBuf::from(&runtime_dir).join(format!("songbird-{}.sock", family))
}

/// Get NestGate unix socket path
///
/// **DEPRECATED**: Use capability-based discovery + `get_socket_path_for_service("nestgate")` instead
///
/// This function violates TRUE PRIMAL self-knowledge principle by hardcoding "nestgate".
///
/// Priority:
/// 1. NESTGATE_SOCKET environment variable (absolute path)
/// 2. Runtime directory + family: `{runtime_dir}/nestgate-{family}.sock`
#[deprecated(
    since = "4.9.0",
    note = "Use capability-based discovery + get_socket_path_for_service() instead"
)]
pub fn get_nestgate_socket_path() -> PathBuf {
    // Priority 1: Direct socket path
    if let Ok(socket) = std::env::var("NESTGATE_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: Runtime directory with family
    let runtime_dir = get_runtime_dir();
    let family = get_family_id();
    PathBuf::from(&runtime_dir).join(format!("nestgate-{}.sock", family))
}

/// Get Squirrel unix socket path
///
/// **DEPRECATED**: Use capability-based discovery + `get_socket_path_for_service("squirrel")` instead
///
/// This function violates TRUE PRIMAL self-knowledge principle by hardcoding "squirrel".
///
/// Priority:
/// 1. SQUIRREL_SOCKET environment variable (absolute path)
/// 2. Runtime directory + family: `{runtime_dir}/squirrel-{family}.sock`
#[deprecated(
    since = "4.9.0",
    note = "Use capability-based discovery + get_socket_path_for_service() instead"
)]
pub fn get_squirrel_socket_path() -> PathBuf {
    // Priority 1: Direct socket path
    if let Ok(socket) = std::env::var("SQUIRREL_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: Runtime directory with family
    let runtime_dir = get_runtime_dir();
    let family = get_family_id();
    PathBuf::from(&runtime_dir).join(format!("squirrel-{}.sock", family))
}

/// Get BiomeOS NUCLEUS socket path
///
/// Priority:
/// 1. NUCLEUS_SOCKET environment variable (absolute path)
/// 2. BIOMEOS_SOCKET_PATH environment variable (orchestrator-provided)
/// 3. Runtime directory: `{runtime_dir}/nucleus.sock`
pub fn get_nucleus_socket_path() -> PathBuf {
    // Priority 1: NUCLEUS-specific socket
    if let Ok(socket) = std::env::var("NUCLEUS_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: BiomeOS-provided socket path
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        return PathBuf::from(socket);
    }

    // Priority 3: Runtime directory
    let runtime_dir = get_runtime_dir();
    PathBuf::from(&runtime_dir).join("nucleus.sock")
}

/// Get ToadStool unix socket path (our own server)
///
/// Priority:
/// 1. TOADSTOOL_SOCKET environment variable (absolute path)
/// 2. BIOMEOS_SOCKET_PATH environment variable (orchestrator-provided)
/// 3. Runtime directory + family: `{runtime_dir}/toadstool-{family}.sock`
pub fn get_toadstool_socket_path() -> PathBuf {
    // Priority 1: ToadStool-specific socket
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: BiomeOS-provided socket path
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        return PathBuf::from(socket);
    }

    // Priority 3: Runtime directory with family
    let runtime_dir = get_runtime_dir();
    let family = get_family_id();
    PathBuf::from(&runtime_dir).join(format!("toadstool-{}.sock", family))
}

/// Get socket path for any service by name
///
/// **TRUE PRIMAL**: Generic socket resolution for ANY service
///
/// Maps service names to socket paths using established patterns.
/// Falls back to generic pattern for unknown services.
///
/// This is the **preferred** method for socket path resolution as it:
/// - Works with ANY service name (discovered or known)
/// - Respects environment variables
/// - Has consistent fallback behavior
#[allow(deprecated)]  // Calls deprecated functions internally for backward compat
pub fn get_socket_path_for_service(service_name: &str) -> PathBuf {
    // Map known service names to specific socket paths (for environment variable support)
    match service_name.to_lowercase().as_str() {
        "beardog" | "bear-dog" => get_beardog_socket_path(),
        "songbird" | "song-bird" => get_songbird_socket_path(),
        "nestgate" | "nest-gate" => get_nestgate_socket_path(),
        "squirrel" => get_squirrel_socket_path(),
        "toadstool" | "toad-stool" => get_toadstool_socket_path(),
        "nucleus" | "biomeos" => get_nucleus_socket_path(),
        
        // Generic pattern for unknown services (TRUE PRIMAL - works with ANY service!)
        _ => {
            // Try service-specific environment variable first
            let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
            if let Ok(socket) = std::env::var(&env_var) {
                return PathBuf::from(socket);
            }
            
            // Fall back to generic pattern
            let runtime_dir = get_runtime_dir();
            let family = get_family_id();
            PathBuf::from(&runtime_dir).join(format!("{}-{}.sock", service_name.to_lowercase(), family))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_dir_from_xdg() {
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        assert_eq!(get_runtime_dir(), "/run/user/1000");
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn test_runtime_dir_fallback() {
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::set_var("USER", "testuser");
        assert_eq!(get_runtime_dir(), "/tmp/toadstool-runtime-testuser");
        std::env::remove_var("USER");
    }

    #[test]
    fn test_beardog_socket_from_env() {
        std::env::set_var("BEARDOG_SOCKET", "/custom/beardog.sock");
        assert_eq!(
            get_beardog_socket_path(),
            PathBuf::from("/custom/beardog.sock")
        );
        std::env::remove_var("BEARDOG_SOCKET");
    }

    #[test]
    fn test_beardog_socket_default() {
        std::env::remove_var("BEARDOG_SOCKET");
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        std::env::set_var("BIOMEOS_FAMILY_ID", "nat0");
        
        let path = get_beardog_socket_path();
        assert_eq!(path, PathBuf::from("/run/user/1000/beardog-nat0.sock"));
        
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::remove_var("BIOMEOS_FAMILY_ID");
    }

    #[test]
    fn test_all_primals_have_unique_paths() {
        let beardog = get_beardog_socket_path();
        let songbird = get_songbird_socket_path();
        let nestgate = get_nestgate_socket_path();
        let squirrel = get_squirrel_socket_path();
        let toadstool = get_toadstool_socket_path();

        // All should be different
        assert_ne!(beardog, songbird);
        assert_ne!(beardog, nestgate);
        assert_ne!(beardog, squirrel);
        assert_ne!(songbird, nestgate);
        assert_ne!(songbird, squirrel);
        assert_ne!(nestgate, squirrel);
        
        // ToadStool can equal BIOMEOS_SOCKET_PATH if set
        // (orchestrator binds multiple primals)
    }
}
