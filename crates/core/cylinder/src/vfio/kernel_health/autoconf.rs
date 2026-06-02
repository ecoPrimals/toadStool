// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::SystemTime;

use super::paths::{autoconf_path, kernel_image_path, kernel_release};
use super::KernelHealthError;

/// Compare mtime of `autoconf.h` against the kernel image.
///
/// Returns `(is_fresh, delta_seconds)` where `is_fresh` is true if
/// autoconf.h is older or same age as the kernel image (the expected state).
pub fn check_autoconf_freshness() -> Result<(bool, i64), KernelHealthError> {
    let krel = kernel_release()?;
    check_autoconf_freshness_for(krel)
}

pub(crate) fn check_autoconf_freshness_for(krel: &str) -> Result<(bool, i64), KernelHealthError> {
    let ac = autoconf_path(krel);
    let ki = kernel_image_path(krel);

    let ac_mtime = std::fs::metadata(&ac)
        .and_then(|m| m.modified())
        .map_err(|e| {
            KernelHealthError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", ac.display()),
            ))
        })?;

    let ki_mtime = std::fs::metadata(&ki)
        .and_then(|m| m.modified())
        .map_err(|e| {
            KernelHealthError::Io(std::io::Error::new(
                e.kind(),
                format!("{}: {e}", ki.display()),
            ))
        })?;

    let delta = mtime_delta_secs(ac_mtime, ki_mtime);
    let fresh = delta <= 0;
    Ok((fresh, delta))
}

pub(crate) fn mtime_delta_secs(a: SystemTime, b: SystemTime) -> i64 {
    match a.duration_since(b) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}
