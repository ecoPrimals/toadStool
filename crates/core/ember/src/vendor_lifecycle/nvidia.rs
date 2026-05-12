// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA GPU lifecycle implementations (Kepler, Volta+, Open, Oracle).
//!
//! Absorbed from coralReef `coral-ember` — adapted to use `crate::sysfs`
//! instead of `coral-driver::linux_paths`.

use crate::error::SwapError;
use crate::sysfs;

use super::types::{RebindStrategy, ResetMethod, VendorLifecycle};

/// NVIDIA Kepler GPUs — GDDR5, no FLR, no bus SBR, cold-hardware-sensitive.
///
/// Kepler differs from Volta+ (HBM2): bus reset does not destroy VRAM
/// training, but there is no FLR and sysfs unbind D-states through PLX bridges.
#[derive(Debug)]
pub struct NvidiaKeplerLifecycle {
    /// PCI device ID — reserved for GK110 vs GK210 differentiation.
    pub device_id: u16,
}

impl VendorLifecycle for NvidiaKeplerLifecycle {
    fn description(&self) -> &'static str {
        "NVIDIA Kepler (GDDR5, no FLR — sysfs unbind D-states through PLX bridge)"
    }

    fn available_reset_methods(&self) -> Vec<ResetMethod> {
        vec![ResetMethod::RemoveRescan]
    }

    fn skip_sysfs_unbind(&self) -> bool {
        true
    }

    fn prepare_for_unbind(&self, bdf: &str, _current_driver: &str) -> Result<(), SwapError> {
        sysfs::pin_power(bdf);
        sysfs::pin_bridge_power(bdf);
        let reset_path = sysfs::pci_device_path(bdf, "reset_method");
        let _ = sysfs::sysfs_write_direct(&reset_path.display().to_string(), "");
        Ok(())
    }

    fn rebind_strategy(&self, target_driver: &str) -> RebindStrategy {
        match target_driver {
            "vfio" | "vfio-pci" => RebindStrategy::SimpleBind,
            _ => RebindStrategy::PciRescan,
        }
    }

    fn settle_secs(&self, target_driver: &str) -> u64 {
        match target_driver {
            "nouveau" => 20,
            _ => 5,
        }
    }

    fn stabilize_after_bind(&self, bdf: &str, _target_driver: &str) {
        sysfs::pin_power(bdf);
        sysfs::pin_bridge_power(bdf);
        let _ = sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        );
    }

    fn verify_health(&self, bdf: &str, _target_driver: &str) -> Result<(), SwapError> {
        let power = sysfs::read_power_state(bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(SwapError::VerifyHealth {
                bdf: bdf.to_string(),
                detail: "Kepler device in D3cold after bind".to_string(),
            });
        }
        Ok(())
    }
}

/// NVIDIA GPUs (Volta+) — bus reset kills HBM2 training; `reset_method` must be disabled.
#[derive(Debug)]
pub struct NvidiaLifecycle {
    /// PCI device ID — reserved for Volta vs Turing vs Ada refinement.
    pub device_id: u16,
}

impl VendorLifecycle for NvidiaLifecycle {
    fn description(&self) -> &'static str {
        "NVIDIA Volta+ (bus reset kills HBM2 — reset_method disabled, PCI rescan for DRM unbind)"
    }

    fn available_reset_methods(&self) -> Vec<ResetMethod> {
        vec![
            ResetMethod::BridgeSbr,
            ResetMethod::SysfsSbr,
            ResetMethod::RemoveRescan,
        ]
    }

    fn prepare_for_unbind(&self, bdf: &str, _current_driver: &str) -> Result<(), SwapError> {
        sysfs::pin_power(bdf);
        sysfs::pin_bridge_power(bdf);
        let _ = sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        );
        Ok(())
    }

    fn skip_sysfs_unbind(&self) -> bool {
        true
    }

    fn rebind_strategy(&self, target_driver: &str) -> RebindStrategy {
        match target_driver {
            "vfio" | "vfio-pci" => RebindStrategy::SimpleBind,
            _ => RebindStrategy::SimpleWithRescanFallback,
        }
    }

    fn settle_secs(&self, target_driver: &str) -> u64 {
        match target_driver {
            "nouveau" => 15,
            _ => 5,
        }
    }

    fn stabilize_after_bind(&self, bdf: &str, _target_driver: &str) {
        sysfs::pin_power(bdf);
        let _ = sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        );
    }

    fn verify_health(&self, bdf: &str, target_driver: &str) -> Result<(), SwapError> {
        let power = sysfs::read_power_state(bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(SwapError::VerifyHealth {
                bdf: bdf.to_string(),
                detail: "device in D3cold after bind".to_string(),
            });
        }

        if target_driver == "vfio-pci" || target_driver == "vfio" {
            let config_path = sysfs::pci_device_path(bdf, "config");
            if let Ok(data) = std::fs::read(&config_path)
                && data.len() >= 4
            {
                let vendor = u16::from_le_bytes([data[0], data[1]]);
                if vendor == 0xFFFF {
                    return Err(SwapError::VerifyHealth {
                        bdf: bdf.to_string(),
                        detail: "PCIe config space all-FF after swap — link dead".to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// NVIDIA open kernel module — uses GSP firmware for falcon management.
#[derive(Debug)]
pub struct NvidiaOpenLifecycle {
    /// PCI device ID — reserved for per-chip GSP support detection.
    pub device_id: u16,
}

impl VendorLifecycle for NvidiaOpenLifecycle {
    fn description(&self) -> &'static str {
        "NVIDIA Open (GSP-based — bus reset kills HBM2)"
    }

    fn available_reset_methods(&self) -> Vec<ResetMethod> {
        vec![
            ResetMethod::BridgeSbr,
            ResetMethod::SysfsSbr,
            ResetMethod::RemoveRescan,
        ]
    }

    fn prepare_for_unbind(&self, bdf: &str, _current_driver: &str) -> Result<(), SwapError> {
        sysfs::pin_power(bdf);
        tracing::info!(
            bdf,
            "NVIDIA Open: disabling reset_method (bus reset destroys HBM2 training)"
        );
        sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        )?;
        Ok(())
    }

    fn rebind_strategy(&self, _target_driver: &str) -> RebindStrategy {
        RebindStrategy::SimpleBind
    }

    fn settle_secs(&self, target_driver: &str) -> u64 {
        match target_driver {
            "nouveau" => 10,
            _ => 8,
        }
    }

    fn stabilize_after_bind(&self, bdf: &str, _target_driver: &str) {
        sysfs::pin_power(bdf);
        let _ = sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        );
    }

    fn verify_health(&self, bdf: &str, _target_driver: &str) -> Result<(), SwapError> {
        let power = sysfs::read_power_state(bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(SwapError::VerifyHealth {
                bdf: bdf.to_string(),
                detail: "NVIDIA Open device in D3cold after bind".to_string(),
            });
        }
        Ok(())
    }
}

/// NVIDIA Oracle — renamed `nvidia_oracle.ko` module for driver coexistence.
#[derive(Debug)]
pub struct NvidiaOracleLifecycle {
    /// PCI device ID from config space.
    pub device_id: u16,
    /// The oracle module name (e.g. `"nvidia_oracle"`, `"nvidia_oracle_535"`).
    pub module_name: String,
}

impl VendorLifecycle for NvidiaOracleLifecycle {
    fn description(&self) -> &'static str {
        "NVIDIA Oracle (renamed module for driver coexistence)"
    }

    fn available_reset_methods(&self) -> Vec<ResetMethod> {
        vec![
            ResetMethod::BridgeSbr,
            ResetMethod::SysfsSbr,
            ResetMethod::RemoveRescan,
        ]
    }

    fn prepare_for_unbind(&self, bdf: &str, _current_driver: &str) -> Result<(), SwapError> {
        sysfs::pin_power(bdf);
        sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        )?;
        Ok(())
    }

    fn rebind_strategy(&self, _target_driver: &str) -> RebindStrategy {
        RebindStrategy::SimpleBind
    }

    fn settle_secs(&self, target_driver: &str) -> u64 {
        match target_driver {
            "nouveau" => 10,
            _ => 8,
        }
    }

    fn stabilize_after_bind(&self, bdf: &str, _target_driver: &str) {
        sysfs::pin_power(bdf);
        let _ = sysfs::sysfs_write_direct(
            &sysfs::pci_device_path(bdf, "reset_method")
                .display()
                .to_string(),
            "",
        );
    }

    fn verify_health(&self, bdf: &str, _target_driver: &str) -> Result<(), SwapError> {
        let power = sysfs::read_power_state(bdf);
        if power.as_deref() == Some("D3cold") {
            return Err(SwapError::VerifyHealth {
                bdf: bdf.to_string(),
                detail: "device in D3cold after bind".to_string(),
            });
        }
        Ok(())
    }
}
