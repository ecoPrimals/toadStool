// SPDX-License-Identifier: AGPL-3.0-or-later

//! Platform-appropriate access control (G68 L2).
//!
//! Replaces scattered `PermissionsExt::set_mode()` and `from_mode()` calls
//! with semantic access levels that map to the correct mechanism per platform.

use std::path::Path;

/// Semantic access levels for files and directories.
///
/// Each variant describes an **intent** rather than a mechanism. The platform
/// implementation maps intent to mode bits (unix) or ACLs (Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformAccess {
    /// Owner-only read/write, no group, no world. (unix: `0o600`)
    ///
    /// Use for: secrets, credentials, private keys.
    OwnerOnly,

    /// Owner read/write/execute, no group, no world. (unix: `0o700`)
    ///
    /// Use for: private directories that contain secrets (e.g., biomeOS socket dir).
    OwnerExclusive,

    /// Owner full control, group read+traverse. (unix: `0o750`)
    ///
    /// Use for: service directories where membrane composition peers need access.
    OwnerFullGroupTraverse,

    /// Owner + group read/write, no world. (unix: `0o660`)
    ///
    /// Use for: IPC sockets shared between processes in the same group.
    GroupShared,

    /// Owner full control, group+world read+execute. (unix: `0o755`)
    ///
    /// Use for: installed executables, launcher scripts.
    Executable,

    /// Arbitrary unix mode bits. (Windows: best-effort mapping)
    ///
    /// Use sparingly — prefer semantic variants. This exists for env-var overrides
    /// like `TOADSTOOL_SOCKET_MODE`.
    Custom(u32),
}

impl PlatformAccess {
    /// Convert to unix mode bits.
    #[cfg(unix)]
    const fn to_mode(self) -> u32 {
        match self {
            Self::OwnerOnly => 0o600,
            Self::OwnerExclusive => 0o700,
            Self::OwnerFullGroupTraverse => 0o750,
            Self::GroupShared => 0o660,
            Self::Executable => 0o755,
            Self::Custom(mode) => mode,
        }
    }
}

/// Set access permissions on a path using semantic intent.
///
/// - **Unix**: applies `chmod` with the appropriate mode bits
/// - **Windows**: best-effort (no-op currently; proper ACL support is a follow-up)
///
/// # Errors
///
/// Returns an I/O error if permissions cannot be set (e.g., not owner, path doesn't exist).
pub fn set_access(path: &Path, access: PlatformAccess) -> std::io::Result<()> {
    set_access_impl(path, access)
}

/// Check whether a path meets a minimum access requirement.
///
/// - **Unix**: checks mode bits match the expected pattern
/// - **Windows**: always returns `true` (ACL checks are a follow-up)
///
/// Returns `true` if the path meets or exceeds the requirement.
///
/// # Errors
///
/// Returns an I/O error if the path metadata cannot be read.
pub fn check_access(path: &Path, required: PlatformAccess) -> std::io::Result<bool> {
    check_access_impl(path, required)
}

// ── Unix implementation ──────────────────────────────────────────────────────

#[cfg(unix)]
fn set_access_impl(path: &Path, access: PlatformAccess) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mode = access.to_mode();
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, perms)
}

#[cfg(unix)]
fn check_access_impl(path: &Path, required: PlatformAccess) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let actual_mode = metadata.permissions().mode() & 0o7777;
    let required_mode = required.to_mode();

    #[allow(clippy::verbose_bit_mask)]
    Ok(match required {
        // For OwnerOnly: ensure no group/world bits are set
        PlatformAccess::OwnerOnly => actual_mode & 0o077 == 0,
        // For OwnerExclusive: ensure no group/world bits
        PlatformAccess::OwnerExclusive => actual_mode & 0o077 == 0,
        // For Executable: ensure at least one execute bit is set
        PlatformAccess::Executable => actual_mode & 0o111 != 0,
        // For Custom: check that all required bits are present
        PlatformAccess::Custom(mode) => actual_mode & mode == mode,
        // For others: exact match on relevant bits
        _ => actual_mode == required_mode,
    })
}

// ── Windows implementation ───────────────────────────────────────────────────

#[cfg(windows)]
fn set_access_impl(path: &Path, access: PlatformAccess) -> std::io::Result<()> {
    // Windows doesn't have unix mode bits. For now, we set read-only for
    // OwnerOnly (removing write for "others" is the closest std equivalent).
    // Proper ACL support via windows-sys is a follow-up.
    let metadata = std::fs::metadata(path)?;
    let mut perms = metadata.permissions();

    match access {
        PlatformAccess::OwnerOnly | PlatformAccess::OwnerExclusive => {
            perms.set_readonly(true);
        }
        _ => {
            perms.set_readonly(false);
        }
    }

    std::fs::set_permissions(path, perms)
}

#[cfg(windows)]
fn check_access_impl(path: &Path, required: PlatformAccess) -> std::io::Result<bool> {
    let metadata = std::fs::metadata(path)?;
    let perms = metadata.permissions();

    Ok(match required {
        PlatformAccess::OwnerOnly | PlatformAccess::OwnerExclusive => perms.readonly(),
        PlatformAccess::Executable => {
            // On Windows, executability is determined by extension, not permissions
            path.extension().map_or(false, |ext| {
                ["exe", "cmd", "bat", "ps1"].iter().any(|e| ext == *e)
            })
        }
        _ => true,
    })
}

// ── Fallback for other platforms ─────────────────────────────────────────────

#[cfg(not(any(unix, windows)))]
fn set_access_impl(_path: &Path, _access: PlatformAccess) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn check_access_impl(_path: &Path, _required: PlatformAccess) -> std::io::Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn set_and_check_owner_only() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("secret.txt");
        fs::write(&path, "secret").unwrap();

        set_access(&path, PlatformAccess::OwnerOnly).unwrap();
        assert!(check_access(&path, PlatformAccess::OwnerOnly).unwrap());
    }

    #[test]
    fn set_group_shared() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("socket");
        fs::write(&path, "").unwrap();

        set_access(&path, PlatformAccess::GroupShared).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
            assert_eq!(mode, 0o660);
        }
    }

    #[test]
    fn set_executable() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        fs::write(&path, "#!/bin/sh").unwrap();

        set_access(&path, PlatformAccess::Executable).unwrap();
        assert!(check_access(&path, PlatformAccess::Executable).unwrap());
    }

    #[test]
    fn set_owner_exclusive_dir() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("private");
        fs::create_dir(&sub).unwrap();

        set_access(&sub, PlatformAccess::OwnerExclusive).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&sub).unwrap().permissions().mode() & 0o7777;
            assert_eq!(mode, 0o700);
        }
    }

    #[test]
    fn custom_mode() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("custom");
        fs::write(&path, "").unwrap();

        set_access(&path, PlatformAccess::Custom(0o640)).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;
            assert_eq!(mode, 0o640);
        }
    }

    #[test]
    fn check_nonexistent_path_errors() {
        let result = check_access(
            Path::new("/nonexistent/path/xyz"),
            PlatformAccess::OwnerOnly,
        );
        assert!(result.is_err());
    }
}
