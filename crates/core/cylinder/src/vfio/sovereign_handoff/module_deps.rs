// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::vfio::guarded_sysfs;
use crate::vfio::kmod;

use super::errors::HandoffError;

/// Load all dependencies for a kernel module.
///
/// Resolves dependencies via `modules.dep` (pure Rust) with fallback to
/// `modprobe --show-depends`, then loads each in order via `insmod`.
/// This is necessary because `insmod` (used for patched modules) doesn't
/// resolve dependencies like `modprobe` does.
pub(crate) fn load_module_dependencies(module_name: &str) -> Result<(), HandoffError> {
    let deps = kmod::resolve_module_dependencies(module_name).map_err(|source| {
        HandoffError::ModuleDependencyResolutionFailed {
            module: module_name.to_string(),
            source,
        }
    })?;

    let mut loaded = 0;

    for ko_path in &deps {
        let dep_name = ko_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .replace('-', "_");

        if kmod::is_module_loaded(&dep_name) {
            continue;
        }

        tracing::debug!(dep = %ko_path.display(), "loading module dependency");
        if let Err(e) = guarded_sysfs::insmod_guarded(ko_path, guarded_sysfs::INSMOD_TIMEOUT) {
            tracing::warn!(dep = %ko_path.display(), error = %e, "dependency load failed (continuing)");
        } else {
            loaded += 1;
        }
    }

    tracing::info!(module = module_name, deps_loaded = loaded, "module dependencies loaded");
    Ok(())
}
