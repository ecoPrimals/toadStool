// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO layer for sovereign GPU dispatch — Phase C absorption from coral-driver.
//!
//! This module contains the hardware lifecycle portions of the VFIO stack:
//! kernel ABI types, ioctl wrappers, DMA, PCI discovery, device open/map,
//! BAR cartography, vendor metal identification, and memory topology.
//!
//! GSP-dependent modules (`bar0`, `probe`, `vfio_compute`) remain in
//! coralReef until Phase D or until the firmware boundary is resolved.

#[cfg(feature = "amd")]
pub mod amd_metal;
pub mod boot_state;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod ce_validate;
pub mod channel;
pub mod bar_cartography;
pub mod cache_ops;
pub mod clutch;
pub mod device;
pub mod dma;
pub mod ember_client;
pub mod ember_gate;
pub mod gpu_vendor;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod guarded_sysfs;
pub mod ioctl;
pub mod irq;
pub mod isolation;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod kernel_health;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod kmod;
pub mod memory;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod module_patch;
pub mod nv_metal;
pub(crate) mod pci_config;
pub mod pci_discovery;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod pmu_investigate;
pub mod init_kepler;
pub mod init_pipeline;
pub mod init_volta;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod sovereign_handoff;
pub mod sovereign_init;
pub mod sovereign_profile;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod sovereign_stages;
pub mod sovereign_strategy;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod sovereign_tiers;
pub mod sovereign_types;
pub mod sysfs_bar0;
pub mod types;
#[expect(missing_docs, reason = "VFIO hardware module — docs tracked as D-DOC")]
pub mod reagent;
pub mod warm_capture;

pub use boot_state::{BootCapability, ColdBootReason, SovereignBootState, probe_boot_state};
pub use device::{DmaBackend, DupAnchorFds, ReceivedVfioFds, VfioBackendKind, VfioDevice};
pub use dma::DmaBuffer;
pub use gpu_vendor::GpuMetal;
pub use nv_metal::detect_gpu_metal;
pub use pci_discovery::{GpuVendor, PciDeviceInfo, force_pci_d0};
