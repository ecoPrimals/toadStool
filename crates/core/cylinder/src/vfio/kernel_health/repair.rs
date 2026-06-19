// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;

use super::KernelHealthError;
use super::RepairStrategy;
use super::paths::{autoconf_path, kernel_release};

/// Attempt to repair the kernel headers by restoring the original `autoconf.h`.
///
/// Returns the path to the restored file on success.
pub fn repair_autoconf(strategy: RepairStrategy) -> Result<PathBuf, KernelHealthError> {
    let krel = kernel_release()?;
    let target = autoconf_path(krel);

    match strategy {
        RepairStrategy::PackageRestore => repair_from_deb_cache(krel, &target),
        RepairStrategy::PackageReinstall => repair_via_reinstall(krel, &target),
    }
}

fn repair_from_deb_cache(krel: &str, target: &Path) -> Result<PathBuf, KernelHealthError> {
    let cache_dir = PathBuf::from("/var/cache/apt/archives");
    let pattern = format!("linux-headers-{krel}_");

    let entries: Vec<_> = std::fs::read_dir(&cache_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name().to_string_lossy().starts_with(&pattern)
                && e.file_name().to_string_lossy().ends_with(".deb")
        })
        .collect();

    if entries.is_empty() {
        return Err(KernelHealthError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "no cached .deb matching {pattern}*.deb in {}",
                cache_dir.display()
            ),
        )));
    }

    let deb_path = entries[0].path();
    let extract_dir = std::env::temp_dir().join("toadstool_autoconf_repair");
    let _ = std::fs::remove_dir_all(&extract_dir);
    std::fs::create_dir_all(&extract_dir)?;

    let status = Command::new("dpkg-deb")
        .args(["-x"])
        .arg(&deb_path)
        .arg(&extract_dir)
        .status()
        .map_err(KernelHealthError::Io)?;

    if !status.success() {
        return Err(KernelHealthError::Io(std::io::Error::other(
            "dpkg-deb extraction failed",
        )));
    }

    let relative = format!("usr/src/linux-headers-{krel}/include/generated/autoconf.h");
    let extracted = extract_dir.join(&relative);

    if !extracted.exists() {
        return Err(KernelHealthError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "autoconf.h not found in extracted .deb at {}",
                extracted.display()
            ),
        )));
    }

    let backup = target.with_extension("h.bak");
    if target.exists() {
        std::fs::copy(target, &backup)?;
        tracing::info!(backup = %backup.display(), "backed up current autoconf.h");
    }

    std::fs::copy(&extracted, target)?;
    tracing::info!(
        source = %deb_path.display(),
        target = %target.display(),
        "restored autoconf.h from .deb cache"
    );

    let _ = std::fs::remove_dir_all(&extract_dir);

    Ok(target.to_path_buf())
}

fn repair_via_reinstall(krel: &str, target: &Path) -> Result<PathBuf, KernelHealthError> {
    let pkg = format!("linux-headers-{krel}");

    let status = Command::new("apt-get")
        .args(["install", "--reinstall", "-y"])
        .arg(&pkg)
        .status()
        .map_err(KernelHealthError::Io)?;

    if !status.success() {
        return Err(KernelHealthError::Io(std::io::Error::other(format!(
            "apt-get install --reinstall {pkg} failed"
        ))));
    }

    Ok(target.to_path_buf())
}
