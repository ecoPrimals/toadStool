// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! `libloading` integration and dynamic symbol resolution for [`super::abi::PluginVTable`].

#![allow(
    unsafe_code,
    reason = "libloading FFI symbol resolution requires unsafe"
)]

use std::ffi::CStr;
use std::path::Path;

use libloading::Library;

use super::abi::{PLUGIN_ABI_VERSION, PluginInitFn, PluginNameFn, PluginVersionFn};
use super::types::PluginError;

/// Owning handle for a loaded plugin shared object and its vtable.
pub struct LoadedPlugin {
    /// Dynamic library handle; dropping it unloads the plugin (`dlclose`).
    #[expect(
        dead_code,
        reason = "held for drop side-effect: dropping unloads the plugin via dlclose"
    )]
    pub library: Library,
    /// Vtable pointer returned by `plugin_init` — valid while `library` is held.
    pub vtable: *mut PluginVTable,
}

impl LoadedPlugin {
    /// Open `path`, resolve `plugin_init`, `plugin_version`, and optional `plugin_name`,
    /// validate ABI compatibility, and run `on_load` when present.
    ///
    /// # Errors
    ///
    /// Returns [`PluginError::LoadFailed`] when the library cannot be opened or a required
    /// symbol is missing; [`PluginError::PluginAbiMismatch`] when ABI versions disagree;
    /// [`PluginError::InvalidManifest`] when the plugin name cannot be read.
    pub fn load(path: &Path, expected_name: &str) -> Result<Self, PluginError> {
        // SAFETY: `Library::new` calls `dlopen` on `path`. The caller is responsible for
        // providing a path to a valid shared object built against the PluginVTable ABI.
        // Constructors in the .so may run arbitrary code; we rely on the plugin contract.
        let library = unsafe { Library::new(path) }.map_err(|e| {
            PluginError::LoadFailed(format!("Library::new({}): {e}", path.display()))
        })?;

        // SAFETY: `library.get` performs `dlsym` for `plugin_init`. The symbol must be an
        // `extern "C"` function matching `PluginInitFn`; ABI compatibility is verified
        // immediately after via `plugin_version`.
        let init: libloading::Symbol<PluginInitFn> = unsafe { library.get(b"plugin_init") }
            .map_err(|_| {
                PluginError::SymbolNotFound(
                    "required symbol `plugin_init` not found in plugin".to_string(),
                )
            })?;

        // SAFETY: `dlsym` for `plugin_version` — same contract as `plugin_init` above.
        let version_fn: libloading::Symbol<PluginVersionFn> =
            unsafe { library.get(b"plugin_version") }.map_err(|_| {
                PluginError::SymbolNotFound(
                    "required symbol `plugin_version` not found in plugin".to_string(),
                )
            })?;

        // SAFETY: Calling `plugin_version` — an `extern "C" fn() -> u32` resolved via dlsym.
        // The library is loaded and the symbol is valid for the lifetime of `library`.
        let plugin_ver = unsafe { version_fn() };
        if plugin_ver != PLUGIN_ABI_VERSION {
            return Err(PluginError::PluginAbiMismatch {
                host: PLUGIN_ABI_VERSION,
                plugin: plugin_ver,
            });
        }

        // SAFETY: Calling `plugin_init` — an `extern "C" fn() -> *mut PluginVTable` resolved
        // via dlsym. ABI version was just confirmed to match. Null return is checked below.
        let vtable = unsafe { init() };
        if vtable.is_null() {
            return Err(PluginError::LoadFailed(
                "plugin_init returned null vtable pointer".to_string(),
            ));
        }

        // SAFETY: `vtable` was confirmed non-null above. The pointer is valid because it was
        // returned by `plugin_init` from a library whose ABI version matches ours, so the
        // layout of `PluginVTable` is guaranteed compatible.
        let vt = unsafe { &*vtable };
        if vt.abi_version != PLUGIN_ABI_VERSION {
            return Err(PluginError::PluginAbiMismatch {
                host: PLUGIN_ABI_VERSION,
                plugin: vt.abi_version,
            });
        }

        let name_from_vt = if vt.name.is_null() {
            None
        } else {
            Some(
                // SAFETY: `vt.name` was confirmed non-null. The plugin contract requires it
                // to point to a valid NUL-terminated C string that lives as long as the library.
                unsafe { CStr::from_ptr(vt.name) }
                    .to_str()
                    .map_err(|_| {
                        PluginError::InvalidManifest(
                            "plugin vtable name is not valid UTF-8".to_string(),
                        )
                    })?
                    .to_string(),
            )
        };

        let name_from_fn: Option<String> =
            // SAFETY: `dlsym` for optional `plugin_name` — same contract as other symbol lookups.
            if let Ok(sym) = unsafe { library.get::<PluginNameFn>(b"plugin_name") } {
                // SAFETY: Calling `plugin_name` — `extern "C" fn() -> *const c_char`. The
                // symbol was resolved from the same ABI-verified library.
                let p = unsafe { sym() };
                if p.is_null() {
                    None
                } else {
                    Some(
                        // SAFETY: `p` was confirmed non-null. The plugin contract requires it
                        // to point to a valid NUL-terminated C string for the library lifetime.
                        unsafe { CStr::from_ptr(p) }
                            .to_str()
                            .map_err(|_| {
                                PluginError::InvalidManifest(
                                    "plugin_name() returned invalid UTF-8".to_string(),
                                )
                            })?
                            .to_string(),
                    )
                }
            } else {
                None
            };

        let resolved = name_from_vt.or(name_from_fn).ok_or_else(|| {
            PluginError::InvalidManifest(
                "plugin must set vtable.name or export plugin_name()".to_string(),
            )
        })?;

        if resolved != expected_name {
            return Err(PluginError::InvalidManifest(format!(
                "plugin library name `{resolved}` does not match registered name `{expected_name}`"
            )));
        }

        if let Some(f) = vt.on_load {
            // SAFETY: `on_load` is an `extern "C" fn() -> i32` stored in the vtable.
            // The vtable pointer was validated non-null and ABI-compatible above.
            let rc = unsafe { f() };
            if rc != 0 {
                return Err(PluginError::LoadFailed(format!(
                    "plugin on_load returned error code {rc}"
                )));
            }
        }

        Ok(Self { library, vtable })
    }

    /// Invoke `on_unload` if present. Safe to call multiple times (second is no-op).
    pub fn unload(&mut self) {
        if self.vtable.is_null() {
            return;
        }
        // SAFETY: `vtable` was confirmed non-null above, and was originally returned by
        // `plugin_init` from an ABI-compatible library that is still loaded (`self.library`).
        let vt = unsafe { &*self.vtable };
        if let Some(f) = vt.on_unload {
            // SAFETY: `on_unload` is an `extern "C" fn()` from the validated vtable.
            // The library is still loaded (dropped after this struct).
            unsafe { f() };
        }
        self.vtable = std::ptr::null_mut();
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        self.unload();
    }
}
