// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure lifecycle step lists and sysfs execution (injectable [`crate::sysfs::SysfsPort`]).
//!
//! Absorbed from coralReef `coral-ember`.

use crate::error::SwapError;
use crate::sysfs::{self, SysfsPort};

/// One sysfs-side effect used during `prepare_for_unbind` (ordering matters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleStep {
    /// Pin device and bridge power rails (`power/control`, `d3cold_allowed`).
    PinPower,
    /// Pin upstream bridge power (walks PCI parents — single level).
    PinBridgePower,
    /// Pin **every** ancestor bridge from device to root complex.
    /// Required for multi-level switch topologies (PLX PEX 8747 on K80).
    PinBridgeHierarchy,
    /// Clear PCI `reset_method` (write empty / newline semantics).
    ClearResetMethod,
}

/// Run `steps` in order via [`SysfsPort`] (for tests and production).
pub fn execute_lifecycle_steps(
    bdf: &str,
    steps: &[LifecycleStep],
    port: &dyn SysfsPort,
) -> Result<(), SwapError> {
    for step in steps {
        match *step {
            LifecycleStep::PinPower => {
                sysfs::pin_power_with(port, bdf);
            }
            LifecycleStep::PinBridgePower => {
                sysfs::pin_bridge_power_with(port, bdf);
            }
            LifecycleStep::PinBridgeHierarchy => {
                let count = sysfs::pin_bridge_hierarchy(bdf);
                tracing::debug!(bdf, bridges_pinned = count, "hierarchy power pinned");
            }
            LifecycleStep::ClearResetMethod => {
                let path = sysfs::pci_device_path(bdf, "reset_method");
                let _ = sysfs::sysfs_write_direct_with(port, &path.display().to_string(), "");
            }
        }
    }
    Ok(())
}

/// Steps for NVIDIA Kepler before unbind.
///
/// Uses [`PinBridgeHierarchy`](LifecycleStep::PinBridgeHierarchy) instead of
/// single-parent `PinBridgePower` because the K80 sits behind a multi-level
/// PLX PEX 8747 switch that enters D3cold if any ancestor is un-pinned.
#[must_use]
pub fn nvidia_kepler_lifecycle_prepare_steps() -> Vec<LifecycleStep> {
    vec![
        LifecycleStep::PinPower,
        LifecycleStep::PinBridgeHierarchy,
        LifecycleStep::ClearResetMethod,
    ]
}

/// Steps for generic/unknown vendor before unbind.
#[must_use]
pub fn generic_lifecycle_prepare_steps(current_driver: &str) -> Vec<LifecycleStep> {
    let mut steps = vec![LifecycleStep::PinPower];
    if current_driver == "vfio-pci" {
        steps.push(LifecycleStep::ClearResetMethod);
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::{
        LifecycleStep, generic_lifecycle_prepare_steps, nvidia_kepler_lifecycle_prepare_steps,
    };

    #[test]
    fn generic_prepare_non_vfio_is_pin_power_only() {
        assert_eq!(
            generic_lifecycle_prepare_steps("nouveau"),
            vec![LifecycleStep::PinPower]
        );
    }

    #[test]
    fn generic_prepare_vfio_adds_clear_reset_method() {
        assert_eq!(
            generic_lifecycle_prepare_steps("vfio-pci"),
            vec![LifecycleStep::PinPower, LifecycleStep::ClearResetMethod]
        );
    }

    #[test]
    fn nvidia_kepler_prepare_three_steps() {
        let s = nvidia_kepler_lifecycle_prepare_steps();
        assert_eq!(s.len(), 3);
        assert_eq!(
            s,
            vec![
                LifecycleStep::PinPower,
                LifecycleStep::PinBridgePower,
                LifecycleStep::ClearResetMethod,
            ]
        );
    }
}
