// SPDX-License-Identifier: AGPL-3.0-only
//! System-level queries (hostname, kernel, etc.) via `/proc` and `/etc`.

use std::path::Path;

/// Read the system hostname.
///
/// Tries `/proc/sys/kernel/hostname` first (always current), then falls back
/// to `/etc/hostname` (static). Returns `None` only if both are unreadable.
#[must_use]
pub fn hostname() -> Option<String> {
    for path in &[
        Path::new("/proc/sys/kernel/hostname"),
        Path::new("/etc/hostname"),
    ] {
        if let Ok(contents) = std::fs::read_to_string(path) {
            let trimmed = contents.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_returns_some_on_linux() {
        // On any Linux system in CI, at least one of the paths should exist.
        if Path::new("/proc/sys/kernel/hostname").exists() || Path::new("/etc/hostname").exists() {
            let h = hostname();
            assert!(h.is_some());
            assert!(!h.unwrap().is_empty());
        }
    }
}
