// SPDX-License-Identifier: AGPL-3.0-only
//! GPU power state and PCI reset management.
//!
//! Provides PCI-level control for GPU lifecycle management:
//! - Power state transitions (D0/D3hot)
//! - PCI function-level reset (FLR) and secondary bus reset
//! - Power budget queries

use crate::error::{NvPmuError, Result};
use std::path::{Path, PathBuf};

/// PCI power state as reported by sysfs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciPowerState {
    D0,
    D1,
    D2,
    D3Hot,
    D3Cold,
    Unknown,
}

impl std::fmt::Display for PciPowerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::D0 => write!(f, "D0"),
            Self::D1 => write!(f, "D1"),
            Self::D2 => write!(f, "D2"),
            Self::D3Hot => write!(f, "D3hot"),
            Self::D3Cold => write!(f, "D3cold"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl PciPowerState {
    fn from_sysfs(s: &str) -> Self {
        match s.trim() {
            "D0" => Self::D0,
            "D1" => Self::D1,
            "D2" => Self::D2,
            "D3hot" => Self::D3Hot,
            "D3cold" => Self::D3Cold,
            _ => Self::Unknown,
        }
    }
}

/// PCI reset method available for a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMethod {
    Flr,
    SecondaryBusReset,
    PmReset,
}

impl std::fmt::Display for ResetMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flr => write!(f, "flr"),
            Self::SecondaryBusReset => write!(f, "bus"),
            Self::PmReset => write!(f, "pm"),
        }
    }
}

/// GPU power and reset controller via PCI sysfs.
pub struct GpuPowerController {
    bdf: String,
    sysfs_path: PathBuf,
}

impl GpuPowerController {
    #[must_use]
    pub fn new(bdf: &str) -> Self {
        Self {
            bdf: bdf.to_string(),
            sysfs_path: PathBuf::from(format!("/sys/bus/pci/devices/{bdf}")),
        }
    }

    /// Query current PCI power state.
    ///
    /// # Errors
    ///
    /// Returns error if the sysfs `power_state` file cannot be read.
    pub fn power_state(&self) -> Result<PciPowerState> {
        let path = self.sysfs_path.join("power_state");
        read_sysfs_trimmed(&path)
            .map(|s| PciPowerState::from_sysfs(&s))
            .map_err(|e| NvPmuError::Hardware(format!("{} power_state: {e}", self.bdf)))
    }

    /// Query available reset methods.
    #[must_use]
    pub fn available_reset_methods(&self) -> Vec<ResetMethod> {
        let path = self.sysfs_path.join("reset_method");
        let Ok(content) = read_sysfs_trimmed(&path) else {
            return Vec::new();
        };

        content
            .split_whitespace()
            .filter_map(|m| match m {
                "flr" => Some(ResetMethod::Flr),
                "bus" => Some(ResetMethod::SecondaryBusReset),
                "pm" => Some(ResetMethod::PmReset),
                _ => None,
            })
            .collect()
    }

    /// Perform a PCI function-level reset (FLR).
    ///
    /// This resets the GPU to a clean state. The device must be unbound
    /// from its driver or bound to vfio-pci. Writing "1" to the `reset`
    /// sysfs file triggers the reset.
    ///
    /// # Errors
    ///
    /// Returns error if the reset sysfs write fails (likely not root or device busy).
    pub fn reset(&self) -> Result<()> {
        let path = self.sysfs_path.join("reset");
        std::fs::write(&path, "1")
            .map_err(|e| NvPmuError::Hardware(format!("{} PCI reset: {e}", self.bdf)))?;
        tracing::info!(bdf = %self.bdf, "PCI function-level reset completed");
        Ok(())
    }

    /// Transition GPU to D0 (full power) state.
    ///
    /// # Errors
    ///
    /// Returns error if the sysfs power control write fails.
    pub fn power_on(&self) -> Result<()> {
        self.set_power_state("D0")
    }

    /// Transition GPU to D3hot (low power) state.
    ///
    /// # Errors
    ///
    /// Returns error if the sysfs power control write fails.
    pub fn power_suspend(&self) -> Result<()> {
        self.set_power_state("D3hot")
    }

    fn set_power_state(&self, state: &str) -> Result<()> {
        let control_path = self.sysfs_path.join("power").join("control");
        if state == "D0" {
            std::fs::write(&control_path, "on")
                .map_err(|e| NvPmuError::Hardware(format!("{} power control: {e}", self.bdf)))?;
        } else {
            std::fs::write(&control_path, "auto")
                .map_err(|e| NvPmuError::Hardware(format!("{} power control: {e}", self.bdf)))?;
        }
        tracing::info!(bdf = %self.bdf, target_state = state, "power state transition requested");
        Ok(())
    }

    /// Query the maximum power draw limit (in microwatts) if available.
    #[must_use]
    pub fn power_limit_uw(&self) -> Option<u64> {
        let path = self.sysfs_path.join("hwmon");
        let hwmon_dir = std::fs::read_dir(&path).ok()?;
        for entry in hwmon_dir.flatten() {
            let limit_path = entry.path().join("power1_cap");
            if let Ok(val) = read_sysfs_trimmed(&limit_path) {
                return val.parse().ok();
            }
        }
        None
    }

    /// Whether this device supports PCI reset.
    #[must_use]
    pub fn supports_reset(&self) -> bool {
        self.sysfs_path.join("reset").exists()
    }

    /// The PCI BDF address.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }
}

fn read_sysfs_trimmed(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path).map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_state_parsing() {
        assert_eq!(PciPowerState::from_sysfs("D0"), PciPowerState::D0);
        assert_eq!(PciPowerState::from_sysfs("D3hot\n"), PciPowerState::D3Hot);
        assert_eq!(PciPowerState::from_sysfs("D3cold"), PciPowerState::D3Cold);
        assert_eq!(PciPowerState::from_sysfs("unknown"), PciPowerState::Unknown);
    }

    #[test]
    fn power_state_display() {
        assert_eq!(PciPowerState::D0.to_string(), "D0");
        assert_eq!(PciPowerState::D3Hot.to_string(), "D3hot");
    }

    #[test]
    fn reset_method_display() {
        assert_eq!(ResetMethod::Flr.to_string(), "flr");
        assert_eq!(ResetMethod::SecondaryBusReset.to_string(), "bus");
    }

    #[test]
    fn controller_creation() {
        let ctrl = GpuPowerController::new("0000:65:00.0");
        assert_eq!(ctrl.bdf(), "0000:65:00.0");
    }

    #[test]
    fn nonexistent_device_methods_return_empty() {
        let ctrl = GpuPowerController::new("ffff:ff:ff.f");
        assert!(ctrl.available_reset_methods().is_empty());
        assert!(!ctrl.supports_reset());
        assert!(ctrl.power_limit_uw().is_none());
    }
}
