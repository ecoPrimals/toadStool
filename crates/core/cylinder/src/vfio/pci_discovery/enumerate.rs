// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native accelerator enumeration: identity from sysfs, liveness from the bus.
//!
//! # Why this exists
//!
//! Consumers across the tree reached for `nvidia-smi`, `rocm-smi`, and `lspci`
//! to answer "what accelerators are present?". Every one is a vendor or distro
//! tool that may be absent, may be a different version than was parsed for,
//! and in `nvidia-smi`'s case ships with the proprietary driver stack this
//! project exists to not require. Parsing their output also made the answer
//! vendor-shaped: the NVIDIA path returned fields the AMD path did not.
//!
//! Everything they report about identity is in sysfs, which the kernel
//! maintains regardless of which driver is bound — including `vfio-pci`, and
//! including none.
//!
//! # Two sources, because they answer different questions
//!
//! [`toadstool_common::pci_discovery`] is the ecosystem's shared sysfs
//! scanner, and this builds on it rather than walking sysfs again. It reads
//! the kernel's *cached* attributes (`vendor`, `device`, `class`), which were
//! captured at bus enumeration.
//!
//! Cached identity survives a device going silent, and that turns out to
//! matter enormously. On biomeGate, both wedged Tesla K80 dies still report
//! `vendor=0x10de device=0x102d class=0x030200` from sysfs attributes while
//! their live config space reads all-ones. Identity from the cache, liveness
//! from the bus, gives "Tesla K80 at 0000:4b:00.0, not responding" — where
//! either source alone gives half an answer, and a class filter over live
//! config space gives *no* answer, because an unresponsive device's class
//! reads `0xffffff` and filters away as though it were never installed.
//!
//! # Capability, not vendor
//!
//! Devices are selected by PCI **class code**, so anything presenting as a
//! display or compute accelerator is found whether or not we recognise the
//! vendor. Vendor is a reported attribute, never a filter.

use std::fs;

use toadstool_common::pci_discovery::{PciDevice, PciFilter, discover_pci_devices};

use super::device_info::PciDeviceInfo;
use super::parse::{parse_pci_resource_file, pci_class_base};
use crate::error::PciDiscoveryError;
use crate::linux_paths::sysfs_pci_device_file;

/// PCI base class 0x03 — display controllers.
const PCI_CLASS_DISPLAY: u8 = 0x03;

/// PCI base class 0x12 — processing accelerators.
///
/// Where datacentre compute parts that present no display engine live, and
/// where an `lspci | grep VGA` style filter misses them entirely.
const PCI_CLASS_ACCELERATOR: u8 = 0x12;

/// PCI config space reads all-ones when a device does not answer. No real
/// vendor ID is `0xFFFF`, so this is unambiguous.
const PCI_VENDOR_NO_RESPONSE: u16 = 0xFFFF;

/// Does this class code denote something we can compute on?
#[must_use]
pub fn is_accelerator_class(class_code_24: u32) -> bool {
    matches!(
        pci_class_base(class_code_24),
        PCI_CLASS_DISPLAY | PCI_CLASS_ACCELERATOR
    )
}

/// Whether a device answers reads on the bus right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// Config space returns real data.
    Responding,
    /// Config space reads all-ones: present and enumerated, but silent.
    ///
    /// A wedged GPU, or one in a power state that does not answer.
    NotResponding,
    /// Config space could not be read at all — typically insufficient
    /// privilege. Says nothing about the device.
    Unknown,
}

impl Liveness {
    /// Whether it is safe to attempt MMIO against this device.
    ///
    /// [`Unknown`](Self::Unknown) is not a yes. Treating "I could not tell"
    /// as "go ahead" is the sentinel-as-data mistake that has cost this
    /// project several GPUs.
    #[must_use]
    pub fn is_usable(self) -> bool {
        self == Self::Responding
    }
}

/// An accelerator: who it is, and whether it is answering.
#[derive(Debug, Clone)]
pub struct Accelerator {
    /// Identity from the kernel's cached sysfs attributes. Survives the
    /// device going silent.
    pub device: PciDevice,
    /// Whether the device answers config space reads right now.
    pub liveness: Liveness,
}

impl Accelerator {
    /// PCI address, e.g. `0000:4b:00.0`.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.device.bdf
    }

    /// Deep-parse config space: BARs, capabilities, power, PCIe link.
    ///
    /// Only meaningful when [`liveness`](Self::liveness) is
    /// [`Responding`](Liveness::Responding); a silent device parses to
    /// all-ones garbage.
    ///
    /// # Errors
    ///
    /// Returns an error if config space cannot be read or is too short.
    pub fn probe_config(&self) -> Result<PciDeviceInfo, PciDiscoveryError> {
        read_device(&self.device.bdf)
    }
}

/// Every accelerator the kernel knows about, responding or not.
///
/// Selection is by class code, so unrecognised vendors still enumerate.
/// Ordering is by BDF, which [`discover_pci_devices`] already guarantees.
#[must_use]
pub fn scan_accelerators() -> Vec<Accelerator> {
    let filter = PciFilter::default().with_class(is_accelerator_class);

    discover_pci_devices(&filter)
        .into_iter()
        .map(|device| {
            let liveness = probe_liveness(&device.bdf);
            if liveness == Liveness::NotResponding {
                tracing::warn!(
                    bdf = %device.bdf,
                    vendor_id = format!("{:#06x}", device.vendor_id),
                    device_id = format!("{:#06x}", device.device_id),
                    "accelerator is enumerated but not answering config space reads"
                );
            }
            Accelerator { device, liveness }
        })
        .collect()
}

/// Accelerators that are answering.
///
/// Prefer [`scan_accelerators`] where a silent device matters — this form
/// cannot distinguish absent from wedged.
#[must_use]
pub fn responding_accelerators() -> Vec<Accelerator> {
    scan_accelerators()
        .into_iter()
        .filter(|a| a.liveness.is_usable())
        .collect()
}

/// Read a device's live vendor ID to decide whether it is answering.
#[must_use]
pub fn probe_liveness(bdf: &str) -> Liveness {
    let path = sysfs_pci_device_file(bdf, "config");
    // Only the first four bytes are needed, but sysfs config reads are cheap
    // and the kernel serves them from a single access.
    let Ok(config) = fs::read(&path) else {
        return Liveness::Unknown;
    };
    if config.len() < 2 {
        return Liveness::Unknown;
    }

    let vendor = u16::from_le_bytes([config[0], config[1]]);
    if vendor == PCI_VENDOR_NO_RESPONSE {
        Liveness::NotResponding
    } else {
        Liveness::Responding
    }
}

/// Read one PCI device's full description from sysfs config space.
///
/// Works for any bound driver, and for none.
///
/// # Errors
///
/// Returns an error if the config file cannot be read, or is too short.
pub fn read_device(bdf: &str) -> Result<PciDeviceInfo, PciDiscoveryError> {
    let config_path = sysfs_pci_device_file(bdf, "config");
    let config = fs::read(&config_path)
        .map_err(|e| PciDiscoveryError::sysfs_io("read PCI config", config_path, e))?;

    // Absent or unreadable BARs are not fatal: identity is still useful, and
    // a device behind a driver that hides its resources still has a class.
    let bars = fs::read_to_string(sysfs_pci_device_file(bdf, "resource"))
        .map(|s| parse_pci_resource_file(&s))
        .unwrap_or_default();

    PciDeviceInfo::from_config_bytes(bdf, &config, bars, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_accelerator_classes_are_both_found() {
        // A consumer GPU presents as a display controller...
        assert!(is_accelerator_class(0x03_00_00));
        // ...a Tesla presents as a 3D controller...
        assert!(is_accelerator_class(0x03_02_00));
        // ...and datacentre compute parts present as 0x12, which a
        // `grep VGA` over lspci output misses entirely.
        assert!(is_accelerator_class(0x12_00_00));
    }

    #[test]
    fn non_accelerators_are_excluded() {
        assert!(!is_accelerator_class(0x01_06_00), "SATA controller");
        assert!(!is_accelerator_class(0x02_00_00), "ethernet");
        assert!(!is_accelerator_class(0x06_04_00), "PCI bridge");
        assert!(!is_accelerator_class(0x0c_03_30), "USB controller");
    }

    /// A silent device's class reads all-ones, which class-filters away as
    /// though it were never installed. Enumerating from cached sysfs
    /// attributes instead of live config space is what avoids this.
    #[test]
    fn all_ones_is_not_a_class_code() {
        assert!(
            !is_accelerator_class(0x00ff_ffff),
            "an unresponsive device must not be judged by its class — it has none"
        );
    }

    /// "Could not tell" must never be treated as "yes".
    #[test]
    fn unknown_liveness_is_not_usable() {
        assert!(Liveness::Responding.is_usable());
        assert!(!Liveness::NotResponding.is_usable());
        assert!(!Liveness::Unknown.is_usable());
    }

    /// Must not panic when sysfs is absent (containers, CI, non-Linux hosts).
    #[test]
    fn enumeration_is_infallible() {
        let _ = scan_accelerators();
        let _ = responding_accelerators();
        assert_eq!(probe_liveness("0000:ff:ff.7"), Liveness::Unknown);
    }
}

#[cfg(test)]
mod live_hardware_probe {
    use super::*;

    /// Prints what the native scanner sees on this machine. Not an assertion —
    /// CI has no GPUs — but the check that it agrees with the vendor tools it
    /// replaces, including on devices those tools omit.
    #[test]
    fn show_accelerators() {
        for a in scan_accelerators() {
            eprintln!(
                "  {} vendor={:#06x} device={:#06x} class={:#08x} driver={:<10} {:?}",
                a.bdf(),
                a.device.vendor_id,
                a.device.device_id,
                a.device.class_code,
                a.device.driver.as_deref().unwrap_or("-"),
                a.liveness
            );
        }
    }
}
