// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use super::KernelHealthError;

pub(crate) fn kernel_release() -> Result<&'static str, KernelHealthError> {
    crate::linux_paths::kernel_release()
        .ok_or_else(|| KernelHealthError::KernelRelease(
            "could not read /proc/sys/kernel/osrelease".into(),
        ))
}

pub(crate) fn headers_dir(krel: &str) -> PathBuf {
    PathBuf::from(format!("/usr/src/linux-headers-{krel}"))
}

pub(crate) fn autoconf_path(krel: &str) -> PathBuf {
    headers_dir(krel).join("include/generated/autoconf.h")
}

pub(crate) fn kernel_image_path(krel: &str) -> PathBuf {
    PathBuf::from(format!("/boot/vmlinuz-{krel}"))
}
