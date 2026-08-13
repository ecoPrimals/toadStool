// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use toadstool_common::platform;

/// Linux filesystem isolation — mount namespace operations.
///
/// Implements [`platform::FilesystemIsolation`] using `rustix::mount`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxFilesystemIsolation;

#[cfg(target_os = "linux")]
impl platform::FilesystemIsolation for LinuxFilesystemIsolation {
    type Error = std::io::Error;

    fn bind_mount(&self, source: &Path, target: &Path, read_only: bool) -> Result<(), Self::Error> {
        rustix::mount::mount_bind(source, target).map_err(std::io::Error::from)?;
        if read_only {
            rustix::mount::mount_remount(
                target,
                rustix::mount::MountFlags::RDONLY | rustix::mount::MountFlags::BIND,
                "",
            )
            .map_err(std::io::Error::from)?;
        }
        Ok(())
    }

    fn mount_tmpfs(&self, target: &Path) -> Result<(), Self::Error> {
        rustix::mount::mount(
            "tmpfs",
            target,
            "tmpfs",
            rustix::mount::MountFlags::empty(),
            Option::<&std::ffi::CStr>::None,
        )
        .map_err(std::io::Error::from)
    }

    fn mount_virtual(&self, target: &Path, fstype: &str) -> Result<(), Self::Error> {
        rustix::mount::mount(
            fstype,
            target,
            fstype,
            rustix::mount::MountFlags::empty(),
            Option::<&std::ffi::CStr>::None,
        )
        .map_err(std::io::Error::from)
    }

    fn unmount(&self, target: &Path) -> Result<(), Self::Error> {
        rustix::mount::unmount(target, rustix::mount::UnmountFlags::empty())
            .map_err(std::io::Error::from)
    }
}
