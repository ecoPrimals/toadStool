// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel module lifecycle management for sovereign driver rotation.
//!
//! Provides `insmod`/`rmmod`/`modinfo` wrappers so the diesel engine can
//! load patched kernel modules on demand without the operator ever touching
//! the kernel directly. Each GPU gets its own module lifecycle — load a
//! patched nouveau for warm handoff, then unload after vfio-pci rebind.
//!
//! Uses `std::process::Command` consistent with the Akida NPU precedent
//! in `akida-setup/src/pcie.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Errors from kernel module operations.
#[derive(Debug, thiserror::Error)]
pub enum KmodError {
    #[error("module '{name}' is already loaded")]
    AlreadyLoaded { name: String },

    #[error("module '{name}' is not loaded")]
    NotLoaded { name: String },

    #[error("insmod {path}: {detail}")]
    LoadFailed { path: String, detail: String },

    #[error("rmmod {name}: {detail}")]
    UnloadFailed { name: String, detail: String },

    #[error("modinfo -n {name}: {detail}")]
    ModinfoFailed { name: String, detail: String },

    #[error("stock module not found for '{name}'")]
    StockModuleNotFound { name: String },

    #[error("kernel build environment corrupted: {diagnosis}")]
    BuildEnvironmentCorrupted { diagnosis: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Check whether a kernel module is currently loaded.
///
/// Probes `/sys/module/{name}` — if the directory exists, the module is loaded.
pub fn is_module_loaded(name: &str) -> bool {
    let path = crate::linux_paths::sysfs_module_path(name);
    Path::new(&path).exists()
}

/// Locate the stock `.ko` file for a kernel module via `modinfo -n`.
///
/// Returns the absolute path to the module file under
/// `/lib/modules/$(uname -r)/`.
pub fn find_stock_module(name: &str) -> Result<PathBuf, KmodError> {
    let output = Command::new("modinfo")
        .args(["-n", name])
        .output()
        .map_err(|e| KmodError::ModinfoFailed {
            name: name.into(),
            detail: format!("failed to execute modinfo: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KmodError::ModinfoFailed {
            name: name.into(),
            detail: stderr.trim().to_string(),
        });
    }

    let path_str = String::from_utf8_lossy(&output.stdout);
    let path_str = path_str.trim();
    if path_str.is_empty() || path_str == "(builtin)" {
        return Err(KmodError::StockModuleNotFound { name: name.into() });
    }

    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(KmodError::StockModuleNotFound { name: name.into() });
    }

    Ok(path)
}

/// Locate a specific DKMS-built `.ko` for a given module name and version.
///
/// DKMS builds land under `/var/lib/dkms/{name}/{version}/{kernel}/x86_64/module/`.
/// This is used when we need a specific driver version (e.g., nvidia-470) that
/// differs from the system-installed version.
pub fn find_dkms_module(name: &str, version: &str) -> Result<PathBuf, KmodError> {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .map_err(|e| KmodError::ModinfoFailed {
            name: name.into(),
            detail: format!("uname -r failed: {e}"),
        })?;
    let kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let path = PathBuf::from(format!(
        "/var/lib/dkms/{name}/{version}/{kernel}/x86_64/module/{name}.ko",
    ));
    if path.exists() {
        return Ok(path);
    }
    let path_zst = path.with_extension("ko.zst");
    if path_zst.exists() {
        return Ok(path_zst);
    }
    Err(KmodError::StockModuleNotFound {
        name: format!("{name}/{version} (DKMS)"),
    })
}

/// Load a kernel module from a `.ko` file via guarded `insmod`.
///
/// Uses child-process isolation with timeout to prevent D-state hangs.
/// The caller is responsible for ensuring the module is not already loaded
/// (or that the module name differs from any loaded module). Use
/// [`is_module_loaded`] to check first.
pub fn load_module(path: &Path) -> Result<(), KmodError> {
    use crate::vfio::guarded_sysfs;

    guarded_sysfs::insmod_guarded(path, guarded_sysfs::INSMOD_TIMEOUT).map_err(|e| {
        KmodError::LoadFailed {
            path: path.display().to_string(),
            detail: e.to_string(),
        }
    })
}

/// Unload a kernel module by name via guarded `rmmod`.
///
/// Uses child-process isolation with timeout to prevent D-state hangs.
pub fn unload_module(name: &str) -> Result<(), KmodError> {
    use crate::vfio::guarded_sysfs;

    guarded_sysfs::rmmod_guarded(name, guarded_sysfs::RMMOD_TIMEOUT).map_err(|e| {
        KmodError::UnloadFailed {
            name: name.into(),
            detail: e.to_string(),
        }
    })
}

/// Ensure a module is loaded, loading it from its stock path if needed.
///
/// Returns `true` if the module was freshly loaded, `false` if it was
/// already present.
pub fn ensure_module_loaded(name: &str) -> Result<bool, KmodError> {
    if is_module_loaded(name) {
        return Ok(false);
    }
    let stock_path = find_stock_module(name)?;
    load_module(&stock_path)?;
    Ok(true)
}

/// Validate that the kernel build environment is healthy before any
/// module compilation or DKMS build. Catches `autoconf.h` corruption
/// and `struct module` layout mismatches that cause misleading load failures.
///
/// See Exp 216 for the full root cause analysis.
pub fn ensure_build_environment_healthy() -> Result<(), KmodError> {
    use crate::vfio::kernel_health;

    match kernel_health::full_kernel_health_check() {
        Ok(report) => {
            if !report.layout_matches {
                return Err(KmodError::BuildEnvironmentCorrupted {
                    diagnosis: report.diagnosis.to_string(),
                });
            }
            Ok(())
        }
        Err(e) => {
            tracing::warn!(err = %e, "kernel health check unavailable — skipping");
            Ok(())
        }
    }
}

/// Resolve the absolute path of all function symbols in a `.ko` file.
///
/// Shells out to `nm` and returns `(symbol_name, file_offset)` pairs for
/// all symbols of type `T` (text/code) or `t` (local text). The offsets
/// are relative to the `.text` section start — suitable for binary patching.
pub fn nm_text_symbols(ko_path: &Path) -> Result<Vec<(String, u64)>, KmodError> {
    let output = Command::new("nm")
        .args(["--defined-only", "-n"])
        .arg(ko_path)
        .output()
        .map_err(KmodError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KmodError::ModinfoFailed {
            name: ko_path.display().to_string(),
            detail: format!("nm failed: {}", stderr.trim()),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut symbols = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && (parts[1] == "T" || parts[1] == "t")
            && let Ok(addr) = u64::from_str_radix(parts[0], 16)
        {
            symbols.push((parts[2].to_string(), addr));
        }
    }

    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_module_loaded_detects_kernel_core() {
        // "kernel" is always present in /sys/module/ on Linux
        #[cfg(target_os = "linux")]
        assert!(is_module_loaded("kernel"));
    }

    #[test]
    fn is_module_loaded_returns_false_for_nonsense() {
        assert!(!is_module_loaded("toadstool_nonexistent_module_12345"));
    }

    #[test]
    fn find_stock_module_fails_for_nonsense() {
        let result = find_stock_module("toadstool_nonexistent_module_12345");
        assert!(result.is_err());
    }
}
