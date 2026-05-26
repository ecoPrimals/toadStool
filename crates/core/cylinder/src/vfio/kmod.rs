// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kernel module lifecycle management for sovereign driver rotation.
//!
//! Provides module load/unload/resolve wrappers so the diesel engine can
//! load patched kernel modules on demand without the operator ever touching
//! the kernel directly. Each GPU gets its own module lifecycle — load a
//! patched nouveau for warm handoff, then unload after vfio-pci rebind.
//!
//! Phase 3: `insmod`/`rmmod` → `finit_module(2)`/`delete_module(2)` syscalls
//! via rustix; `modinfo -n` → pure Rust `modules.dep` + `modules.builtin`
//! parser. Only `modprobe --show-depends` remains as a fallback.

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

    #[error("module resolve {name}: {detail}")]
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

/// Resolve a kernel module's `.ko` path by parsing `modules.dep` and
/// `modules.builtin`.
///
/// Returns `Some(path)` if the module exists on disk as a loadable `.ko`.
/// Returns `None` for builtin modules, missing modules, or if the kernel
/// module metadata is unparseable.
///
/// Phase 3: pure Rust — no `modinfo` binary dependency.
pub fn modinfo_path(name: &str) -> Option<PathBuf> {
    let krel = crate::linux_paths::kernel_release()?;
    let modules_dir = format!("/lib/modules/{krel}");

    // Check modules.builtin first — builtin modules have no .ko
    let builtin_path = format!("{modules_dir}/modules.builtin");
    if let Ok(builtin) = std::fs::read_to_string(&builtin_path) {
        let needle = format!("/{name}.ko");
        if builtin.lines().any(|line| line.ends_with(&needle)) {
            return None; // builtin, no .ko file
        }
    }

    // Parse modules.dep — each line is "relative/path.ko: dep1.ko dep2.ko ..."
    let dep_path = format!("{modules_dir}/modules.dep");
    if let Ok(depfile) = std::fs::read_to_string(&dep_path) {
        let needle = format!("/{name}.ko");
        for line in depfile.lines() {
            let Some((module_rel, _)) = line.split_once(':') else { continue };
            if module_rel.ends_with(&needle)
                || module_rel.ends_with(&format!("{needle}.zst"))
                || module_rel.ends_with(&format!("{needle}.xz"))
            {
                let abs = format!("{modules_dir}/{module_rel}");
                let abs_path = PathBuf::from(&abs);
                if abs_path.exists() {
                    return Some(abs_path);
                }
            }
        }
    }

    None
}

/// Locate the stock `.ko` file for a kernel module via `modinfo -n`.
///
/// Returns the absolute path to the module file under
/// `/lib/modules/{krel}/`. Returns an error if the module is not found
/// or is builtin.
pub fn find_stock_module(name: &str) -> Result<PathBuf, KmodError> {
    modinfo_path(name).ok_or_else(|| KmodError::StockModuleNotFound { name: name.into() })
}

/// Locate a specific DKMS-built `.ko` for a given module name and version.
///
/// DKMS builds land under `/var/lib/dkms/{name}/{version}/{kernel}/x86_64/module/`.
/// This is used when we need a specific driver version (e.g., nvidia-470) that
/// differs from the system-installed version.
pub fn find_dkms_module(name: &str, version: &str) -> Result<PathBuf, KmodError> {
    let kernel = crate::linux_paths::kernel_release().ok_or_else(|| {
        KmodError::ModinfoFailed {
            name: name.into(),
            detail: "could not read kernel release from /proc".into(),
        }
    })?;

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

/// Load a kernel module from a `.ko` or `.ko.zst` file via guarded `insmod`.
///
/// If the path ends in `.ko.zst`, the module is decompressed in-process via
/// `ruzstd` to a temporary `.ko` before calling `finit_module(2)`.
///
/// Uses child-process isolation with timeout to prevent D-state hangs.
/// The caller is responsible for ensuring the module is not already loaded
/// (or that the module name differs from any loaded module). Use
/// [`is_module_loaded`] to check first.
pub fn load_module(path: &Path) -> Result<(), KmodError> {
    use crate::vfio::guarded_sysfs;

    let effective_path = if needs_decompression(path) {
        decompress_ko_zst(path)?
    } else {
        path.to_path_buf()
    };

    let result = guarded_sysfs::insmod_guarded(&effective_path, guarded_sysfs::INSMOD_TIMEOUT)
        .map_err(|e| KmodError::LoadFailed {
            path: effective_path.display().to_string(),
            detail: e.to_string(),
        });

    if needs_decompression(path) {
        let _ = std::fs::remove_file(&effective_path);
    }

    result
}

/// Check if a module path needs decompression (`.ko.zst` or `.ko.xz`).
fn needs_decompression(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.ends_with(".ko.zst") || s.ends_with(".ko.xz")
}

/// Decompress a `.ko.zst` module to a temporary `.ko` file using ruzstd.
fn decompress_ko_zst(path: &Path) -> Result<PathBuf, KmodError> {
    let compressed = std::fs::read(path).map_err(|e| KmodError::LoadFailed {
        path: path.display().to_string(),
        detail: format!("failed to read compressed module: {e}"),
    })?;

    let mut decoder = ruzstd::decoding::StreamingDecoder::new(compressed.as_slice())
        .map_err(|e| KmodError::LoadFailed {
            path: path.display().to_string(),
            detail: format!("zstd decoder init failed: {e}"),
        })?;

    let mut decompressed = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut decompressed)
        .map_err(|e| KmodError::LoadFailed {
            path: path.display().to_string(),
            detail: format!("zstd decompression failed: {e}"),
        })?;

    let stem = path.file_stem()
        .and_then(|s| std::path::Path::new(s).file_stem())
        .unwrap_or_default()
        .to_string_lossy();
    let dest = std::env::temp_dir().join(format!("toadstool-decompressed-{stem}.ko"));
    std::fs::write(&dest, &decompressed).map_err(|e| KmodError::LoadFailed {
        path: path.display().to_string(),
        detail: format!("failed to write decompressed module: {e}"),
    })?;

    tracing::info!(
        src = %path.display(), dest = %dest.display(),
        compressed_bytes = compressed.len(), decompressed_bytes = decompressed.len(),
        "decompressed .ko.zst for insmod"
    );

    Ok(dest)
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

/// Resolve transitive module dependencies from `/lib/modules/{krel}/modules.dep`.
///
/// Returns dependency `.ko` paths in load order (deepest dependency first),
/// excluding the target module itself. Falls back to `modprobe --show-depends`
/// if `modules.dep` is missing or unparseable.
pub fn resolve_module_dependencies(module_name: &str) -> Result<Vec<PathBuf>, KmodError> {
    if let Some(deps) = resolve_from_modules_dep(module_name) {
        return Ok(deps);
    }
    resolve_from_modprobe(module_name)
}

/// Parse `modules.dep` and resolve transitive dependencies.
fn resolve_from_modules_dep(module_name: &str) -> Option<Vec<PathBuf>> {
    let krel = crate::linux_paths::kernel_release()?;
    let dep_path = format!("/lib/modules/{krel}/modules.dep");
    let content = std::fs::read_to_string(&dep_path).ok()?;

    // Build adjacency map: module_stem -> (full_path, [dep_stems])
    // Format: "kernel/drivers/gpu/drm/nvidia.ko: kernel/drivers/.../dep1.ko kernel/.../dep2.ko"
    let mut adjacency: std::collections::HashMap<String, (String, Vec<String>)> =
        std::collections::HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (module_rel, deps_str) = line.split_once(':')?;
        let module_rel = module_rel.trim();
        let stem = Path::new(module_rel)
            .file_stem()
            .and_then(|s| s.to_str())?
            .replace('-', "_");

        let dep_stems: Vec<String> = deps_str
            .split_whitespace()
            .filter_map(|d| {
                Path::new(d.trim())
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.replace('-', "_"))
            })
            .collect();

        let full_path = format!("/lib/modules/{krel}/{module_rel}");
        adjacency.insert(stem, (full_path, dep_stems));
    }

    let normalized = module_name.replace('-', "_");
    let (_, direct_deps) = adjacency.get(&normalized)?;

    // Resolve transitive deps via DFS (post-order = load order)
    let mut visited = std::collections::HashSet::new();
    let mut order = Vec::new();
    for dep in direct_deps {
        resolve_dep_dfs(dep, &adjacency, &mut visited, &mut order);
    }

    Some(order.into_iter().map(PathBuf::from).collect())
}

fn resolve_dep_dfs(
    name: &str,
    adjacency: &std::collections::HashMap<String, (String, Vec<String>)>,
    visited: &mut std::collections::HashSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(name.to_string()) {
        return;
    }
    if let Some((path, deps)) = adjacency.get(name) {
        for dep in deps {
            resolve_dep_dfs(dep, adjacency, visited, order);
        }
        order.push(path.clone());
    }
}

/// Fallback: use `modprobe --show-depends` when `modules.dep` is unavailable.
fn resolve_from_modprobe(module_name: &str) -> Result<Vec<PathBuf>, KmodError> {
    let output = Command::new("modprobe")
        .args(["--show-depends", module_name])
        .output()
        .map_err(|e| KmodError::ModinfoFailed {
            name: module_name.into(),
            detail: format!("modprobe --show-depends failed: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(KmodError::ModinfoFailed {
            name: module_name.into(),
            detail: format!("modprobe --show-depends: {}", stderr.trim()),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let deps: Vec<PathBuf> = stdout
        .lines()
        .filter_map(|line| {
            let ko_path = line.strip_prefix("insmod ")?.trim();
            if ko_path.contains(&format!("/{module_name}.ko")) {
                None
            } else {
                Some(PathBuf::from(ko_path))
            }
        })
        .collect();

    Ok(deps)
}

/// Resolve all defined text function symbols in a `.ko` file.
///
/// Pure Rust ELF parser — no external `nm` process. Returns
/// `(symbol_name, address)` pairs for `STT_FUNC` symbols with
/// `STB_GLOBAL` or `STB_LOCAL` binding (equivalent to `nm -n --defined-only`
/// filtering `T`/`t`). Results are sorted by address.
pub fn nm_text_symbols(ko_path: &Path) -> Result<Vec<(String, u64)>, KmodError> {
    use object::read::elf::ElfFile64;
    use object::{Endianness, Object, ObjectSymbol, SymbolKind, SymbolSection};

    let data = std::fs::read(ko_path)?;
    let elf = ElfFile64::<Endianness>::parse(&*data).map_err(|e| {
        KmodError::ModinfoFailed {
            name: ko_path.display().to_string(),
            detail: format!("ELF parse failed: {e}"),
        }
    })?;

    let mut symbols: Vec<(String, u64)> = elf
        .symbols()
        .filter(|sym| {
            sym.kind() == SymbolKind::Text
                && !matches!(sym.section(), SymbolSection::Undefined)
                && sym.address() > 0
        })
        .filter_map(|sym| {
            sym.name().ok().map(|name| (name.to_string(), sym.address()))
        })
        .collect();

    symbols.sort_by_key(|(_, addr)| *addr);
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
