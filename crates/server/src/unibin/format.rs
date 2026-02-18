//! Socket path format and biomeOS directory layout
//!
//! Defines the standard socket path resolution and directory structure
//! for ecoBin/UniBin deployment.

use std::path::{Path, PathBuf};

use tracing::info;

/// Ensure biomeos directory exists with proper permissions
pub fn ensure_biomeos_directory(
    runtime_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let biomeos_dir = runtime_dir.join("biomeos");

    std::fs::create_dir_all(&biomeos_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&biomeos_dir, perms)?;
    }

    info!("✅ biomeos directory ensured: {}", biomeos_dir.display());
    Ok(biomeos_dir)
}

/// Get socket filename based on family ID
///
/// - If family ID is "default" or not set: `toadstool.sock`
/// - If family ID is set and not "default": `toadstool-{family_id}.sock`
pub fn socket_filename_for_family(family_id: &str) -> String {
    if family_id.is_empty() || family_id == "default" {
        "toadstool.sock".to_string()
    } else {
        format!("toadstool-{}.sock", family_id)
    }
}

/// Get socket path following biomeOS-standardized fallback
///
/// Priority order:
/// 1. TOADSTOOL_SOCKET env var
/// 2. PRIMAL_SOCKET env var (with family suffix)
/// 3. BIOMEOS_SOCKET_PATH env var
/// 4. XDG runtime directory
/// 5. /tmp fallback
pub fn get_socket_path(
    family_id: &str,
    _node_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        info!("✅ Using socket path from TOADSTOOL_SOCKET: {}", socket);
        return Ok(PathBuf::from(socket));
    }

    if let Ok(socket) = std::env::var("PRIMAL_SOCKET") {
        let socket_with_family = format!("{}-{}", socket, family_id);
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
            PathBuf::from(format!("/run/user/{}", uid))
        } else {
            std::env::var("USER")
                .ok()
                .and_then(|user| {
                    std::fs::read_to_string("/etc/passwd")
                        .ok()
                        .and_then(|passwd| {
                            passwd
                                .lines()
                                .find(|line| line.starts_with(&format!("{}:", user)))
                                .and_then(|line| {
                                    line.split(':')
                                        .nth(2)
                                        .and_then(|uid| uid.parse::<u32>().ok())
                                })
                                .map(|uid| PathBuf::from(format!("/run/user/{}", uid)))
                        })
                })
                .unwrap_or_else(|| PathBuf::from("/tmp"))
        }
    } else {
        PathBuf::from("/tmp")
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

    let tmp_biomeos = PathBuf::from("/tmp/biomeos");
    std::fs::create_dir_all(&tmp_biomeos)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&tmp_biomeos, perms)?;
    }

    let socket_filename = socket_filename_for_family(family_id);
    let tmp_path = tmp_biomeos.join(socket_filename);
    info!("⚠️  Using /tmp fallback for dev/testing deployment");
    Ok(tmp_path)
}
