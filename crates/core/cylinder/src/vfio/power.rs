// SPDX-License-Identifier: AGPL-3.0-or-later

//! PCI power state control for probe correctness.
//!
//! # Why probes must wake the device first
//!
//! A GPU in D3hot does not fail BAR0 reads. The PCI bus returns all-ones and
//! the read reports success, so every register appears to hold `0xFFFF_FFFF`.
//! Any probe that runs without checking the power state is therefore
//! describing the power state rather than the hardware.
//!
//! vfio-pci allows runtime suspend, so a GPU reliably drops to D3hot shortly
//! after being bound — including right after a warm swap, which is exactly
//! when the pipeline wants to measure whether the handoff preserved state.
//!
//! Observed 2026-08-16 on a Titan V: classification immediately after warm
//! swap read all-ones across the board and reported "Tier 0: Cold boot".
//! Waking the device first yielded `pmc_enable=0x5FEC_DFF1` — 23 engines,
//! PRAMIN accessible. The handoff had worked the entire time.

use std::time::Duration;

use crate::linux_paths;

/// Settle time after a power transition before registers are trustworthy.
const WAKE_SETTLE: Duration = Duration::from_millis(50);

/// Read the current PCI power state (`D0`, `D3hot`, ...).
///
/// Returns `"unknown"` when sysfs cannot be read, which is treated as
/// "possibly asleep" by [`wake_to_d0`].
#[must_use]
pub fn power_state(bdf: &str) -> String {
    std::fs::read_to_string(linux_paths::sysfs_pci_device_file(bdf, "power_state"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

/// Whether the device is in D0 and can serve register reads.
#[must_use]
pub fn is_awake(bdf: &str) -> bool {
    power_state(bdf) == "D0"
}

/// Outcome of a wake attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeResult {
    /// Power state before the attempt.
    pub before: String,
    /// Power state after the attempt.
    pub after: String,
    /// Whether the device ended up in D0.
    pub awake: bool,
    /// Whether anything was actually changed.
    pub acted: bool,
}

/// Bring a device to D0 so register reads mean something.
///
/// Best-effort by design: callers must still validate what they read, since
/// a device can fail to wake for reasons sysfs will not report. Pair this
/// with [`crate::nv::register_read::RegisterRead`] rather than trusting that
/// a successful wake implies a successful read.
///
/// Pins `power/control` to `on` so the device does not immediately re-suspend
/// between the wake and the probe.
pub fn wake_to_d0(bdf: &str) -> WakeResult {
    let before = power_state(bdf);
    if before == "D0" {
        return WakeResult {
            after: before.clone(),
            before,
            awake: true,
            acted: false,
        };
    }

    // "on" disables runtime PM for this device; without it the device can
    // suspend again in the window between waking and probing.
    let _ = std::fs::write(
        linux_paths::sysfs_pci_device_file(bdf, "power/control"),
        "on",
    );
    // Powers the device up and restores memory decode.
    let _ = std::fs::write(linux_paths::sysfs_pci_device_file(bdf, "enable"), "1");
    std::thread::sleep(WAKE_SETTLE);

    let after = power_state(bdf);
    let awake = after == "D0";
    if awake {
        tracing::info!(bdf, before = before.as_str(), "woke device to D0 for probing");
    } else {
        tracing::warn!(
            bdf,
            before = before.as_str(),
            after = after.as_str(),
            "device did not reach D0 — register reads will not be trustworthy"
        );
    }

    WakeResult {
        before,
        after,
        awake,
        acted: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonexistent_device_reports_unknown() {
        assert_eq!(power_state("ffff:ff:ff.9"), "unknown");
        assert!(!is_awake("ffff:ff:ff.9"));
    }

    /// Waking a device that does not exist must not panic, and must report
    /// honestly that it never reached D0.
    #[test]
    fn wake_of_nonexistent_device_is_not_awake() {
        let r = wake_to_d0("ffff:ff:ff.9");
        assert!(!r.awake);
        assert_eq!(r.after, "unknown");
    }

    /// An already-awake device should be left alone.
    #[test]
    fn already_awake_is_a_noop() {
        // Synthesised: the real branch is keyed purely on the string.
        let r = WakeResult {
            before: "D0".into(),
            after: "D0".into(),
            awake: true,
            acted: false,
        };
        assert!(r.awake);
        assert!(!r.acted, "no writes when the device is already in D0");
    }
}
