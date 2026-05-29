// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCI ID tables and lifecycle detection from sysfs PCI IDs.

use crate::sysfs;

use super::types::VendorLifecycle;
use super::{
    AmdRdnaLifecycle, AmdVega20Lifecycle, BrainChipLifecycle, GenericLifecycle, IntelXeLifecycle,
    NvidiaKeplerLifecycle, NvidiaLifecycle, NvidiaOpenLifecycle, NvidiaOracleLifecycle,
};

// Canonical vendor IDs: toadstool_common::pci::vendors::{NVIDIA,AMD,INTEL,BRAINCHIP}_VENDOR_ID
const NVIDIA_VENDOR: u16 = 0x10de;
const AMD_VENDOR: u16 = 0x1002;
const INTEL_VENDOR: u16 = 0x8086;
const BRAINCHIP_VENDOR: u16 = 0x1E7C;

const AMD_VEGA20_IDS: &[u16] = &[0x66a0, 0x66a1, 0x66af];

const NVIDIA_KEPLER_IDS: &[u16] = &[
    0x1003, 0x1004, 0x1005, 0x100a, 0x100c, // GK110
    0x1021, 0x1022, 0x1024, 0x1026, 0x1027, 0x1028, 0x1029, 0x102a, 0x102e, 0x102f, // GK110B
    0x102d, // GK210 (K80)
];

pub(crate) fn is_nvidia_kepler(device_id: u16) -> bool {
    NVIDIA_KEPLER_IDS.contains(&device_id)
}

pub(crate) fn is_amd_vega20(device_id: u16) -> bool {
    AMD_VEGA20_IDS.contains(&device_id)
}

/// Build a [`VendorLifecycle`] from PCI config-space IDs.
pub(crate) fn lifecycle_from_pci_ids(vendor_id: u16, device_id: u16) -> Box<dyn VendorLifecycle> {
    match vendor_id {
        NVIDIA_VENDOR => {
            if is_nvidia_kepler(device_id) {
                Box::new(NvidiaKeplerLifecycle { device_id })
            } else {
                Box::new(NvidiaLifecycle { device_id })
            }
        }
        AMD_VENDOR => {
            if is_amd_vega20(device_id) {
                Box::new(AmdVega20Lifecycle { device_id })
            } else {
                Box::new(AmdRdnaLifecycle { device_id })
            }
        }
        INTEL_VENDOR => Box::new(IntelXeLifecycle::new(device_id)),
        BRAINCHIP_VENDOR => Box::new(BrainChipLifecycle { device_id }),
        _ => Box::new(GenericLifecycle {
            vendor_id,
            device_id,
        }),
    }
}

/// Build a lifecycle for a specific target driver override.
pub fn detect_lifecycle_for_target(bdf: &str, target: &str) -> Box<dyn VendorLifecycle> {
    if target.starts_with("nvidia_oracle") {
        let device_id = sysfs::read_pci_id(bdf, "device");
        return Box::new(NvidiaOracleLifecycle {
            device_id,
            module_name: target.to_string(),
        });
    }
    if target == "nvidia-open" {
        let device_id = sysfs::read_pci_id(bdf, "device");
        return Box::new(NvidiaOpenLifecycle { device_id });
    }
    detect_lifecycle(bdf)
}

/// Auto-detect the appropriate [`VendorLifecycle`] for a PCI device.
pub fn detect_lifecycle(bdf: &str) -> Box<dyn VendorLifecycle> {
    let vendor_id = sysfs::read_pci_id(bdf, "vendor");
    let device_id = sysfs::read_pci_id(bdf, "device");

    tracing::info!(
        bdf,
        vendor = format!("0x{vendor_id:04x}"),
        device = format!("0x{device_id:04x}"),
        "detecting vendor lifecycle"
    );

    lifecycle_from_pci_ids(vendor_id, device_id)
}
