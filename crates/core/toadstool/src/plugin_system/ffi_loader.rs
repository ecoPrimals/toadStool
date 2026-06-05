// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! `libloading` integration and dynamic symbol resolution for [`super::abi::PluginVTable`].

#![expect(
    unsafe_code,
    reason = "libloading FFI symbol resolution requires unsafe"
)]

use std::ffi::{CStr, c_char};
use std::path::Path;

use libloading::{Library, Symbol};

use super::abi::{
    PLUGIN_ABI_VERSION, PluginInitFn, PluginNameFn, PluginVTable, PluginVersionFn,
};
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
        let library = open_library(path)?;

        let init = resolve_required_symbol::<PluginInitFn>(&library, b"plugin_init", "plugin_init")?;
        let version_fn =
            resolve_required_symbol::<PluginVersionFn>(&library, b"plugin_version", "plugin_version")?;

        let plugin_ver = call_plugin_version(&version_fn);
        if plugin_ver != PLUGIN_ABI_VERSION {
            return Err(PluginError::PluginAbiMismatch {
                host: PLUGIN_ABI_VERSION,
                plugin: plugin_ver,
            });
        }

        let vtable_ptr = call_plugin_init(&init);
        if vtable_ptr.is_null() {
            return Err(PluginError::LoadFailed(
                "plugin_init returned null vtable pointer".to_string(),
            ));
        }

        let vt = vtable_ref(vtable_ptr)?;
        if vt.abi_version != PLUGIN_ABI_VERSION {
            return Err(PluginError::PluginAbiMismatch {
                host: PLUGIN_ABI_VERSION,
                plugin: vt.abi_version,
            });
        }

        let name_from_vt = utf8_from_plugin_c_str_optional(vt.name)?;

        let name_from_fn = resolve_optional_symbol::<PluginNameFn>(&library, b"plugin_name")
            .map(|sym| {
                let p = call_plugin_name(&sym);
                utf8_from_plugin_c_str_optional(p)
            })
            .transpose()?
            .flatten();

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
            let rc = call_on_load(f);
            if rc != 0 {
                return Err(PluginError::LoadFailed(format!(
                    "plugin on_load returned error code {rc}"
                )));
            }
        }

        Ok(Self {
            library,
            vtable: vtable_ptr,
        })
    }

    /// Invoke `on_unload` if present. Safe to call multiple times (second is no-op).
    pub fn unload(&mut self) {
        if self.vtable.is_null() {
            return;
        }
        let vt = match vtable_ref(self.vtable) {
            Ok(vt) => vt,
            Err(_) => {
                self.vtable = std::ptr::null_mut();
                return;
            }
        };
        if let Some(f) = vt.on_unload {
            call_on_unload(f);
        }
        self.vtable = std::ptr::null_mut();
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        self.unload();
    }
}

// --- FFI boundary: `libloading` + plugin C ABI ---

fn open_library(path: &Path) -> Result<Library, PluginError> {
    // SAFETY: `Library::new` calls `dlopen` on `path`. The caller must supply a path to a
    // shared object built against the PluginVTable ABI. Constructors in the .so may run
    // arbitrary code; we rely on the plugin contract.
    unsafe { Library::new(path) }.map_err(|e| {
        PluginError::LoadFailed(format!("Library::new({}): {e}", path.display()))
    })
}

fn resolve_required_symbol<'lib, T>(
    library: &'lib Library,
    name: &[u8],
    label: &str,
) -> Result<Symbol<'lib, T>, PluginError> {
    // SAFETY: `library.get` performs `dlsym`. The symbol must match type `T` as an
    // `extern "C"` export from the loaded plugin; ABI compatibility is verified separately.
    unsafe { library.get(name) }.map_err(|_| {
        PluginError::SymbolNotFound(format!("required symbol `{label}` not found in plugin"))
    })
}

fn resolve_optional_symbol<'lib, T>(
    library: &'lib Library,
    name: &[u8],
) -> Option<Symbol<'lib, T>> {
    // SAFETY: Optional `dlsym`; same contract as `resolve_required_symbol` when `Ok`.
    unsafe { library.get(name).ok() }
}

fn call_plugin_version(f: &Symbol<PluginVersionFn>) -> u32 {
    // SAFETY: Resolved from an open library; `PluginVersionFn` is `extern "C" fn() -> u32`.
    unsafe { f() }
}

fn call_plugin_init(f: &Symbol<PluginInitFn>) -> *mut PluginVTable {
    // SAFETY: Resolved from an open library; `PluginInitFn` returns a vtable pointer.
    unsafe { f() }
}

fn call_plugin_name(f: &Symbol<PluginNameFn>) -> *const c_char {
    // SAFETY: Resolved from an open library; `PluginNameFn` returns a name pointer.
    unsafe { f() }
}

fn call_on_load(f: unsafe extern "C" fn() -> i32) -> i32 {
    // SAFETY: `f` comes from an ABI-validated vtable while the library is still loaded.
    unsafe { f() }
}

fn call_on_unload(f: unsafe extern "C" fn()) {
    // SAFETY: `f` comes from an ABI-validated vtable while the library is still loaded.
    unsafe { f() };
}

fn vtable_ref<'a>(vtable: *mut PluginVTable) -> Result<&'a PluginVTable, PluginError> {
    if vtable.is_null() {
        return Err(PluginError::LoadFailed(
            "null vtable pointer".to_string(),
        ));
    }
    // SAFETY: Non-null pointer returned by `plugin_init` from an ABI-compatible library;
    // valid for `'a` while the host holds the loaded library handle.
    Ok(unsafe { &*vtable })
}

/// Decode a plugin-owned C string when the pointer may be null.
fn utf8_from_plugin_c_str_optional(ptr: *const c_char) -> Result<Option<String>, PluginError> {
    if ptr.is_null() {
        return Ok(None);
    }
    Ok(Some(utf8_from_plugin_c_str(ptr)?))
}

/// Decode a non-null plugin-owned NUL-terminated UTF-8 C string.
fn utf8_from_plugin_c_str(ptr: *const c_char) -> Result<String, PluginError> {
    debug_assert!(!ptr.is_null());
    // SAFETY: Caller checked non-null. The plugin contract requires a valid NUL-terminated
    // C string that lives as long as the loaded library.
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr.to_str()
        .map_err(|_| {
            PluginError::InvalidManifest("plugin C string is not valid UTF-8".to_string())
        })
        .map(|s| s.to_owned())
}
