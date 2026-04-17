// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Stable C ABI for dynamic plugins (`dlopen` / `LoadLibrary`).

/// Current plugin ABI version — must match [`PluginVTable::abi_version`] and
/// the value returned by the exported `plugin_version` symbol.
pub const PLUGIN_ABI_VERSION: u32 = 1;

/// Initializes the plugin and returns a static vtable pointer valid until `plugin_fini` / unload.
pub type PluginInitFn = unsafe extern "C" fn() -> *mut PluginVTable;

/// Returns the ABI version implemented by this shared object.
pub type PluginVersionFn = unsafe extern "C" fn() -> u32;

/// Optional: returns a NUL-terminated plugin name (fallback if vtable `name` is null).
pub type PluginNameFn = unsafe extern "C" fn() -> *const std::ffi::c_char;

/// Stable vtable published by the plugin.
#[repr(C)]
pub struct PluginVTable {
    /// Must equal [`PLUGIN_ABI_VERSION`].
    pub abi_version: u32,
    /// NUL-terminated UTF-8 name, or null if provided only via `plugin_name`.
    pub name: *const std::ffi::c_char,
    /// Optional hook invoked after successful init (return non-zero on failure).
    pub on_load: Option<unsafe extern "C" fn() -> i32>,
    /// Optional hook invoked before the host drops the library handle.
    pub on_unload: Option<unsafe extern "C" fn()>,
}
