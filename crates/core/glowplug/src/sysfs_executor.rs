// SPDX-License-Identifier: AGPL-3.0-or-later

//! Production [`SwapExecutor`] using sysfs driver bind/unbind.
//!
//! Replaces `coral-glowplug`'s `EmberClient` cross-process IPC pattern:
//! since ember is now toadStool-internal (S237), personality swaps are
//! performed directly via `/sys/bus/pci/drivers/*/unbind` and `/bind`.
//!
//! The executor reads the current driver, unbinds it, writes
//! `driver_override` to the target, and triggers the new driver to bind.
//! This is the same operation that `coral-ember` performed over Unix
//! sockets — now inlined.

use std::time::Instant;

use crate::device_id::DeviceId;
use crate::swap::{SwapExecutor, SwapObservation};

/// Sysfs-based swap executor for PCI devices.
///
/// Performs driver unbind/rebind via sysfs writes. This is the first
/// production `SwapExecutor` — it replaces `EmberClient::swap_device`.
#[derive(Debug)]
pub struct SysfsSwapExecutor;

/// Errors from sysfs swap operations.
#[derive(Debug, thiserror::Error)]
pub enum SysfsSwapError {
    /// The device identifier is not a PCI BDF.
    #[error("sysfs swap requires PCI BDF, got: {0}")]
    NotPciBdf(String),
    /// sysfs write failed.
    #[error("sysfs write to {path}: {reason}")]
    SysfsWrite {
        /// sysfs path that failed.
        path: String,
        /// Reason for failure.
        reason: String,
    },
    /// The target driver did not bind after the swap.
    #[error("driver {driver} did not bind to {bdf} after swap")]
    BindFailed {
        /// PCI BDF.
        bdf: String,
        /// Expected driver.
        driver: String,
    },
}

impl SysfsSwapExecutor {
    fn bdf_from_id(device: &DeviceId) -> Result<&str, SysfsSwapError> {
        match device {
            DeviceId::PciBdf(bdf) => Ok(bdf.as_str()),
            other => Err(SysfsSwapError::NotPciBdf(other.short_label())),
        }
    }

    fn read_current_driver(bdf: &str) -> Option<String> {
        let link = format!("/sys/bus/pci/devices/{bdf}/driver");
        std::fs::read_link(&link)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    fn sysfs_write(path: &str, value: &str) -> Result<(), SysfsSwapError> {
        std::fs::write(path, value).map_err(|e| SysfsSwapError::SysfsWrite {
            path: path.into(),
            reason: e.to_string(),
        })
    }

    fn driver_name_for_target(target: &str) -> &str {
        match target {
            "vfio" | "vfio-pci" => "vfio-pci",
            "nouveau" => "nouveau",
            "nvidia" => "nvidia",
            "nvidia-open" | "nvidia_open" => "nvidia",
            "amdgpu" => "amdgpu",
            "xe" => "xe",
            "i915" => "i915",
            "akida" | "akida-pcie" => "akida-pcie",
            other => other,
        }
    }
}

impl SwapExecutor for SysfsSwapExecutor {
    type Error = SysfsSwapError;

    async fn execute_swap(
        &self,
        device: &DeviceId,
        target_personality: &str,
    ) -> Result<SwapObservation, Self::Error> {
        let bdf = Self::bdf_from_id(device)?;
        let start = Instant::now();
        let from = Self::read_current_driver(bdf)
            .unwrap_or_else(|| "unbound".to_string());

        let target_driver = Self::driver_name_for_target(target_personality);

        if let Some(ref current) = Self::read_current_driver(bdf) {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            tracing::info!(bdf, driver = current.as_str(), "unbinding current driver");
            Self::sysfs_write(&unbind_path, bdf)?;
        }

        if target_personality != "unbound" {
            let override_path = format!("/sys/bus/pci/devices/{bdf}/driver_override");
            Self::sysfs_write(&override_path, target_driver)?;

            let probe_path = "/sys/bus/pci/drivers_probe";
            Self::sysfs_write(probe_path, bdf)?;

            let bound = Self::read_current_driver(bdf);
            if bound.as_deref() != Some(target_driver) {
                return Err(SysfsSwapError::BindFailed {
                    bdf: bdf.into(),
                    driver: target_driver.into(),
                });
            }

            tracing::info!(bdf, driver = target_driver, "driver bound successfully");
        }

        let duration = start.elapsed();
        Ok(SwapObservation {
            device_id: device.short_label(),
            from,
            to: target_personality.into(),
            success: true,
            duration,
            error: None,
            detail: Some(serde_json::json!({
                "method": "sysfs",
                "bind_ms": duration.as_millis() as u64,
            })),
        })
    }

    async fn release(&self, device: &DeviceId) -> Result<(), Self::Error> {
        let bdf = Self::bdf_from_id(device)?;
        if let Some(ref current) = Self::read_current_driver(bdf) {
            let unbind_path = format!("/sys/bus/pci/drivers/{current}/unbind");
            Self::sysfs_write(&unbind_path, bdf)?;
            tracing::info!(bdf, driver = current.as_str(), "device released (unbound)");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdf_from_pci_device_id() {
        let id = DeviceId::PciBdf("0000:01:00.0".into());
        assert_eq!(SysfsSwapExecutor::bdf_from_id(&id).unwrap(), "0000:01:00.0");
    }

    #[test]
    fn bdf_from_non_pci_errors() {
        let id = DeviceId::UsbPath("1-2".into());
        assert!(SysfsSwapExecutor::bdf_from_id(&id).is_err());
    }

    #[test]
    fn driver_name_mapping() {
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("vfio-pci"), "vfio-pci");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nouveau"), "nouveau");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("nvidia-open"), "nvidia");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("amdgpu"), "amdgpu");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("xe"), "xe");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("akida"), "akida-pcie");
        assert_eq!(SysfsSwapExecutor::driver_name_for_target("custom"), "custom");
    }

    #[test]
    fn sysfs_swap_error_display() {
        let err = SysfsSwapError::NotPciBdf("usb:1-2".into());
        assert!(err.to_string().contains("PCI BDF"));

        let err = SysfsSwapError::SysfsWrite {
            path: "/sys/foo".into(),
            reason: "permission denied".into(),
        };
        assert!(err.to_string().contains("/sys/foo"));

        let err = SysfsSwapError::BindFailed {
            bdf: "0000:01:00.0".into(),
            driver: "vfio-pci".into(),
        };
        assert!(err.to_string().contains("vfio-pci"));
    }

    #[tokio::test]
    async fn release_nonexistent_device_is_noop() {
        let exec = SysfsSwapExecutor;
        let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
        let result = exec.release(&id).await;
        assert!(result.is_ok());
    }
}
