// SPDX-License-Identifier: AGPL-3.0-only
#![deny(unsafe_code)] // overridden per-module for bar0
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

//! Sovereign GPU power management for NVIDIA GPUs.
//!
//! Pure Rust replacement for `nvidia-smi` on sovereign systems. Reads GPU
//! telemetry via hwmon/sysfs without requiring proprietary drivers or
//! firmware at runtime.
//!
//! # Architecture
//!
//! ```text
//! nvPmu
//! ├── hwmon      — temperature, power, clock via /sys/class/hwmon/
//! ├── firmware   — probe /lib/firmware/nvidia/{chip}/ inventory
//! ├── pci        — PCI device discovery via /sys/bus/pci/devices/
//! └── monitor    — continuous polling + safety thresholds
//! ```
//!
//! # Phase 0: Read-only monitoring (discover GPUs, read sensors, probe firmware)
//! # Phase 3: BAR0 MMIO register access for software PMU init

// bar0 and init modules require unsafe for mmap/MMIO — isolated from the rest
#[allow(unsafe_code, reason = "BAR0 MMIO requires mmap + volatile read/write")]
pub mod bar0;
#[allow(
    unsafe_code,
    reason = "DMA buffer allocation requires mmap, mlock, and VFIO DMA ioctls"
)]
pub mod dma;
pub mod error;
pub mod firmware;
pub mod hwmon;
#[allow(
    unsafe_code,
    reason = "init sequence writes to BAR0 registers via RegisterAccess"
)]
pub mod init;
pub mod monitor;
pub mod nvidia_smi;
pub mod pci;
pub mod permissions;
pub mod power;
#[allow(
    unsafe_code,
    reason = "VFIO BAR0 access requires mmap + volatile read/write + VFIO ioctls"
)]
pub mod vfio;
pub mod vfio_bind;
pub mod watchdog;

pub use bar0::Bar0Access;
pub use dma::{supports_huge_pages, DmaAllocator, DmaBuffer, HugePageSize};
pub use error::{NvPmuError, Result};
pub use firmware::FirmwareInventory;
pub use hwmon::HwmonSensors;
pub use init::RegisterSnapshot;
pub use monitor::{MonitorConfig, SafetyStatus};
pub use pci::NvidiaGpu;
pub use power::{GpuPowerController, PciPowerState, ResetMethod};
pub use vfio::{VfioBar0Access, VfioMsixInterrupt};
pub use vfio_bind::{BindResult, BindingState};
