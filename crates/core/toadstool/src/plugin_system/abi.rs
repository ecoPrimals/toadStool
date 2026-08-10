// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Stable C ABI for dynamic plugins (deprecated — ecoBin v3.0 removed C FFI loading).
//!
//! Retained for documentation of the legacy plugin ABI. Native loading is no longer
//! supported; use WASM runtimes or capability IPC instead.
//!
//! - **Calling convention**: `extern "C"` with no unwinding across the boundary.
//! - **Lifetime**: pointers returned from `plugin_init` / `plugin_name` / `vtable.name`
//!   remain valid until `on_unload` completes and the library is unloaded.
//! - **Threading**: hooks may be called from the host loader thread only unless
//!   documented otherwise; re-entrancy from `on_load` is forbidden.
//!
//! Violating these invariants can cause use-after-free, ABI mismatch crashes, or
//! undefined behavior when the host dereferences plugin-owned pointers.

/// Current plugin ABI version — must match [`PluginVTable::abi_version`] and
/// the value returned by the exported `plugin_version` symbol.
pub const PLUGIN_ABI_VERSION: u32 = 1;

/// Initializes the plugin and returns a static vtable pointer valid until `plugin_fini` / unload.
///
/// # Safety (call-site contract)
///
/// - Resolved via `dlsym` from a plugin built against [`PLUGIN_ABI_VERSION`].
/// - Must return a non-null, correctly laid-out [`PluginVTable`] on success.
/// - The vtable and any pointers it contains must outlive the loaded library handle.
/// - Calling before ABI validation or after unload is undefined behavior.
pub type PluginInitFn = unsafe extern "C" fn() -> *mut PluginVTable;

/// Returns the ABI version implemented by this shared object.
///
/// # Safety (call-site contract)
///
/// - Resolved via `dlsym`; must be a plain `extern "C"` function with no parameters.
/// - Return value must match the plugin's actual vtable layout; mismatch is caught
///   by the host but calling through a wrong function pointer is UB.
pub type PluginVersionFn = unsafe extern "C" fn() -> u32;

/// Optional: returns a NUL-terminated plugin name (fallback if vtable `name` is null).
///
/// # Safety (call-site contract)
///
/// - When non-null, the returned pointer must reference a valid NUL-terminated UTF-8
///   C string that lives until the library is unloaded.
/// - A null pointer is allowed and signals "name unavailable via this export".
pub type PluginNameFn = unsafe extern "C" fn() -> *const std::ffi::c_char;

/// Stable vtable published by the plugin.
#[repr(C)]
pub struct PluginVTable {
    /// Must equal [`PLUGIN_ABI_VERSION`].
    pub abi_version: u32,
    /// NUL-terminated UTF-8 name, or null if provided only via `plugin_name`.
    pub name: *const std::ffi::c_char,
    /// Optional hook invoked after successful init (return non-zero on failure).
    ///
    /// # Safety (call-site contract)
    ///
    /// - Invoked only while the host holds the open library handle and after
    ///   ABI/name validation during native load (deprecated).
    /// - Must not unwind across the FFI boundary; return non-zero to signal failure.
    /// - If absent (`None`), the host skips initialization hooks.
    pub on_load: Option<unsafe extern "C" fn() -> i32>,
    /// Optional hook invoked before the host drops the library handle.
    ///
    /// # Safety (call-site contract)
    ///
    /// - Invoked at most once per load, before `dlclose`, from the host unload path.
    /// - Must not access host-owned pointers after returning; must not unwind.
    /// - After this returns, all vtable pointers and plugin-owned strings are stale.
    pub on_unload: Option<unsafe extern "C" fn()>,
}
