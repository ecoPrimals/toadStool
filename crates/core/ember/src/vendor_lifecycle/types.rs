// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core types for vendor lifecycle: reset methods, rebind strategy, and the [`VendorLifecycle`] trait.

use std::fmt;

use crate::error::SwapError;

/// Available PCI reset methods for a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMethod {
    /// VFIO_DEVICE_RESET ioctl — requires an open VFIO fd and FLR-capable hardware.
    VfioFlr,
    /// Sysfs `reset` file on the device itself.
    SysfsSbr,
    /// Reset via the parent PCI bridge's `reset` file (Secondary Bus Reset).
    BridgeSbr,
    /// Full PCI remove + bus rescan cycle. Most aggressive.
    RemoveRescan,
}

/// How to transition a device from unbound to a new driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebindStrategy {
    /// Standard sysfs driver_override + drivers/{target}/bind.
    SimpleBind,
    /// Try simple bind first; fall back to PCI remove + bus rescan on failure.
    SimpleWithRescanFallback,
    /// Go straight to PCI remove + bus rescan.
    PciRescan,
    /// PM power cycle (D3hot→D0) then simple bind. Required for AMD Vega 20.
    PmResetAndBind,
}

/// Vendor-specific lifecycle hooks invoked by the swap orchestrator.
///
/// Implementors encode hardware-specific knowledge about safe driver
/// transitions. The trait is intentionally coarse-grained — each method
/// maps to a phase of the swap sequence rather than individual sysfs writes.
pub trait VendorLifecycle: Send + Sync + fmt::Debug {
    /// Human-readable chip family description.
    fn description(&self) -> &'static str;

    /// Called before any driver unbind.
    fn prepare_for_unbind(&self, bdf: &str, current_driver: &str) -> Result<(), SwapError>;

    /// How to rebind a native driver after the device is in unbound state.
    fn rebind_strategy(&self, target_driver: &str) -> RebindStrategy;

    /// Seconds to wait for driver initialization after bind succeeds.
    fn settle_secs(&self, target_driver: &str) -> u64;

    /// Called immediately after a driver binds and settles.
    fn stabilize_after_bind(&self, bdf: &str, target_driver: &str);

    /// Post-bind health check.
    fn verify_health(&self, bdf: &str, target_driver: &str) -> Result<(), SwapError>;

    /// Whether to skip the direct `driver/unbind` sysfs write during swap.
    fn skip_sysfs_unbind(&self) -> bool {
        false
    }

    /// Which reset methods are safe/available, in priority order.
    fn available_reset_methods(&self) -> Vec<ResetMethod> {
        vec![ResetMethod::VfioFlr, ResetMethod::SysfsSbr]
    }
}
