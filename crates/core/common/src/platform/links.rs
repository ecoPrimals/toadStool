// SPDX-License-Identifier: AGPL-3.0-or-later

//! Platform-appropriate filesystem links (G68 L1).
//!
//! Replaces scattered `std::os::unix::fs::symlink` calls with a single
//! cross-platform entry point.

use std::path::Path;

/// Create a platform-appropriate filesystem link.
///
/// - **Unix**: creates a symbolic link (`symlink(target, link)`)
/// - **Windows**: creates a symlink (file or directory depending on target)
///
/// This is designed for best-effort compatibility links (legacy socket names,
/// migration symlinks). Callers should treat failure as non-fatal.
///
/// # Errors
///
/// Returns an I/O error if the link cannot be created (e.g., target doesn't
/// exist, insufficient privileges on Windows, or filesystem doesn't support links).
pub fn platform_link(target: &Path, link: &Path) -> std::io::Result<()> {
    platform_link_impl(target, link)
}

#[cfg(unix)]
fn platform_link_impl(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn platform_link_impl(target: &Path, link: &Path) -> std::io::Result<()> {
    if target.is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

#[cfg(not(any(unix, windows)))]
fn platform_link_impl(_target: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "filesystem links not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn link_to_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "hello").unwrap();

        let link = dir.path().join("link.txt");
        platform_link(&target, &link).unwrap();

        assert!(link.exists());
        assert_eq!(fs::read_to_string(&link).unwrap(), "hello");
    }

    #[test]
    fn link_to_nonexistent_target() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("does_not_exist");
        let link = dir.path().join("link");

        // On unix, symlink to nonexistent target succeeds (dangling symlink)
        #[cfg(unix)]
        {
            platform_link(&target, &link).unwrap();
            assert!(link.symlink_metadata().is_ok());
        }

        // On other platforms, behavior may vary
        #[cfg(not(unix))]
        {
            let _ = platform_link(&target, &link);
        }
    }

    #[test]
    fn link_already_exists_fails() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        fs::write(&target, "hello").unwrap();

        let link = dir.path().join("link.txt");
        platform_link(&target, &link).unwrap();

        let result = platform_link(&target, &link);
        assert!(result.is_err());
    }
}
