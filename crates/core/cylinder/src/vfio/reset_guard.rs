// SPDX-License-Identifier: AGPL-3.0-or-later

//! Guarded PCI function reset.
//!
//! # Why resetting a wedged device can hang the whole machine
//!
//! A sysfs `reset` writes config space, waits, and writes it again. Config
//! access to a device that is not answering enters the kernel's CRS
//! ("Configuration Request Retry Status") retry loop, and that loop runs
//! holding the global `pci_lock`. Every other PCI operation on the box then
//! blocks behind it — including the display GPU's, so the machine appears to
//! freeze rather than to report an error.
//!
//! The odds get worse behind a switch. Both Tesla K80 dies sit under a PLX
//! PEX 8747, and a reset of one function is arbitrated by a bridge that is
//! itself reachable only through config space.
//!
//! Observed 2026-08-16: both K80 dies were wedged (BAR0 all-ones) after a
//! sovereign init, and `echo 1 > /sys/bus/pci/devices/0000:4b:00.0/reset` was
//! issued to recover them. The gate locked hard and needed a power cut. The
//! irony is exact: the reset was an attempt to recover a device whose
//! unresponsiveness was the very thing that made the reset unsafe.
//!
//! # The rule
//!
//! **Never reset a device that is not answering.** A responsive device can be
//! reset; an unresponsive one must be recovered by a reboot, which reinitialises
//! the bridge hierarchy along with the endpoint. That is slower and it is also
//! the only option that does not risk taking the host down with it.
//!
//! This module exists so the rule lives in code rather than in someone's
//! memory of a bad afternoon.

use crate::linux_paths;
use crate::nv::register_read::RegisterRead;

/// Why a reset was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetRefusal {
    /// BAR0 is not answering; resetting risks a `pci_lock` deadlock.
    DeviceNotAnswering {
        /// What BAR0 offset 0 returned.
        boot0: RegisterRead,
    },
    /// BAR0 could not be mapped at all, so responsiveness is unknown.
    Unprobeable {
        /// Why the mapping failed.
        reason: String,
    },
    /// The device has no `reset` attribute.
    NoResetAttribute,
}

impl std::fmt::Display for ResetRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceNotAnswering { boot0 } => write!(
                f,
                "device is not answering (boot0 = {}); a reset would enter CRS retry \
                 holding pci_lock and can hang the host. Recover by reboot instead",
                boot0.describe()
            ),
            Self::Unprobeable { reason } => write!(
                f,
                "cannot establish whether the device answers ({reason}); \
                 refusing to reset on an unknown state"
            ),
            Self::NoResetAttribute => f.write_str("device exposes no reset attribute"),
        }
    }
}

/// Whether the device answers a BAR0 read of `PMC_BOOT_0`.
///
/// This is the liveness question that decides whether a reset is safe. It is
/// deliberately a BAR0 read rather than a config read: probing config space is
/// the very operation that hangs on a wedged device.
fn probe_boot0(bdf: &str) -> Result<RegisterRead, String> {
    let bar = crate::vfio::device::MappedBar::from_sysfs_rw(bdf, 0x1000)
        .map_err(|e| format!("BAR0 map failed: {e}"))?;
    Ok(RegisterRead::from_result(bar.read_u32(0)))
}

/// Check whether it is safe to reset this device, without resetting it.
///
/// # Errors
///
/// Returns the reason a reset must not be attempted.
pub fn check_reset_safe(bdf: &str) -> Result<(), ResetRefusal> {
    if !std::path::Path::new(&linux_paths::sysfs_pci_device_file(bdf, "reset")).exists() {
        return Err(ResetRefusal::NoResetAttribute);
    }

    match probe_boot0(bdf) {
        Ok(boot0) if boot0.is_valid() => Ok(()),
        Ok(boot0) => Err(ResetRefusal::DeviceNotAnswering { boot0 }),
        Err(reason) => Err(ResetRefusal::Unprobeable { reason }),
    }
}

/// Reset a PCI function, but only if it is still answering.
///
/// Prefer this over writing `reset` directly. The direct write is what took
/// the gate down on 2026-08-16; see the module docs.
///
/// # Errors
///
/// Refuses when the device is unresponsive, and propagates write failures.
pub fn guarded_function_reset(bdf: &str) -> Result<(), String> {
    check_reset_safe(bdf).map_err(|r| {
        tracing::error!(bdf, refusal = %r, "refusing PCI function reset");
        r.to_string()
    })?;

    tracing::warn!(
        bdf,
        "issuing PCI function reset (device confirmed responsive)"
    );
    std::fs::write(linux_paths::sysfs_pci_device_file(bdf, "reset"), "1")
        .map_err(|e| format!("reset write failed: {e}"))?;

    // A device that stops answering across its own reset has not recovered,
    // and must not be reset again — the second attempt is the dangerous one.
    std::thread::sleep(std::time::Duration::from_millis(500));
    match probe_boot0(bdf) {
        Ok(b) if b.is_valid() => {
            tracing::info!(bdf, boot0 = b.describe(), "device responsive after reset");
            Ok(())
        }
        Ok(b) => Err(format!(
            "device stopped answering after reset (boot0 = {}). Do NOT retry: \
             recover by reboot",
            b.describe()
        )),
        Err(e) => Err(format!("could not re-probe after reset: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_device_has_no_reset_attribute() {
        assert_eq!(
            check_reset_safe("ffff:ff:ff.9"),
            Err(ResetRefusal::NoResetAttribute)
        );
    }

    /// The exact state both K80 dies were in when the reset was issued.
    #[test]
    fn refusal_message_names_the_hazard_and_the_remedy() {
        let r = ResetRefusal::DeviceNotAnswering {
            boot0: RegisterRead::classify(0xFFFF_FFFF),
        };
        let msg = r.to_string();
        assert!(msg.contains("pci_lock"), "must name the deadlock mechanism");
        assert!(msg.contains("reboot"), "must name the safe remedy");
    }

    /// A live device is resettable; the guard must not block ordinary use.
    #[test]
    fn a_responsive_read_is_not_a_refusal() {
        assert!(RegisterRead::classify(0x0f22_d0a1).is_valid());
        assert!(!RegisterRead::classify(0xFFFF_FFFF).is_valid());
    }
}
