// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO layer for sovereign GPU dispatch — Phase C absorption from coral-driver.
//!
//! This module contains the hardware lifecycle portions of the VFIO stack:
//! kernel ABI types, ioctl wrappers, DMA, PCI discovery, device open/map,
//! BAR cartography, vendor metal identification, and memory topology.
//!
//! GSP-dependent modules (`bar0`, `probe`, `vfio_compute`) remain in
//! coralReef until Phase D or until the firmware boundary is resolved.

pub mod amd_metal;
pub mod channel;
pub mod bar_cartography;
pub mod cache_ops;
pub mod device;
pub mod dma;
pub mod ember_client;
pub mod ember_gate;
pub mod gpu_vendor;
pub mod ioctl;
pub mod irq;
pub mod isolation;
pub mod memory;
pub mod nv_metal;
pub(crate) mod pci_config;
pub mod pci_discovery;
pub mod sovereign_init;
pub mod sovereign_stages;
pub mod sovereign_types;
pub mod sysfs_bar0;
pub mod types;
pub mod warm_capture;

pub use device::{DmaBackend, ReceivedVfioFds, VfioBackendKind, VfioDevice};
pub use dma::DmaBuffer;
pub use gpu_vendor::GpuMetal;
pub use nv_metal::detect_gpu_metal;
pub use pci_discovery::{GpuVendor, PciDeviceInfo, force_pci_d0};
