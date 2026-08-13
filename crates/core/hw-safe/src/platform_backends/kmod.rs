// SPDX-License-Identifier: AGPL-3.0-or-later

/// Load a kernel module from an open `.ko` file via `finit_module(2)`.
#[cfg(target_os = "linux")]
pub fn finit_module(
    ko_file: &impl std::os::fd::AsFd,
    params: &std::ffi::CStr,
    flags: i32,
) -> std::io::Result<()> {
    rustix::system::finit_module(ko_file, params, flags).map_err(std::io::Error::from)
}

/// Unload a kernel module by name via `delete_module(2)`.
#[cfg(target_os = "linux")]
pub fn delete_module(name: &std::ffi::CStr, flags: i32) -> std::io::Result<()> {
    rustix::system::delete_module(name, flags).map_err(std::io::Error::from)
}
