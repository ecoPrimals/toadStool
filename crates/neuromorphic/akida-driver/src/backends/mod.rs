// SPDX-License-Identifier: AGPL-3.0-or-later
//! NPU backend implementations
//!
//! Three backends available:
//! - **Kernel**: Uses `/dev/akida*` (requires C kernel module, best performance)
//! - **VFIO**: Pure Rust with DMA via IOMMU (no C module, good performance)
//! - **Userspace**: Memory-mapped PCIe BARs (pure Rust, no DMA, development)
//!
//! Deep Debt Compliance:
//! - Runtime capability discovery (no hardcoding)
//! - Comprehensive error handling
//! - Graceful fallbacks

#[cfg(unix)]
pub mod kernel;
#[cfg(unix)]
pub mod mmap;
#[cfg(unix)]
pub mod userspace;
#[cfg(target_os = "linux")]
pub mod vfio;

#[cfg(unix)]
pub use kernel::KernelBackend;
#[cfg(unix)]
pub use userspace::UserspaceBackend;
#[cfg(target_os = "linux")]
pub use vfio::{DmaBuffer, VfioBackend};

/// Read NPU power consumption from hwmon sysfs (pure Rust, no `glob` crate).
///
/// Enumerates `/sys/bus/pci/devices/{addr}/hwmon/hwmon*/power1_average`
/// using `std::fs::read_dir` instead of the `glob` external dependency.
#[cfg(unix)]
pub(crate) fn read_hwmon_power(pcie_address: &str) -> Option<f32> {
    let hwmon_dir = format!("/sys/bus/pci/devices/{pcie_address}/hwmon");
    for entry in std::fs::read_dir(&hwmon_dir).ok()?.flatten() {
        let power_path = entry.path().join("power1_average");
        if let Ok(content) = std::fs::read_to_string(&power_path)
            && let Ok(microwatts) = content.trim().parse::<u64>()
        {
            #[expect(
                clippy::cast_precision_loss,
                reason = "precision loss acceptable for this conversion"
            )]
            let watts = microwatts as f32 / 1_000_000.0;
            return Some(watts);
        }
    }
    None
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn test_read_hwmon_power_nonexistent_device() {
        let result = read_hwmon_power("0000:00:00.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_read_hwmon_power_invalid_address() {
        let result = read_hwmon_power("/invalid/path");
        assert!(result.is_none());
    }
}
