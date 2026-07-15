// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)] // overridden per-module for bar0
#![warn(missing_docs)]

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
//! ├── registers     — GV100 BAR0 register map (PMC, PFIFO, PBUS, FB)
//! ├── power_manager — five-state sovereign power model (glow plug)
//! ├── power_policy  — autonomous power policies (OnDemand, Eco, etc.)
//! ├── fb            — HBM2 framebuffer controller probe (skeleton)
//! ├── hwmon         — temperature, power, clock via /sys/class/hwmon/
//! ├── firmware      — probe /lib/firmware/nvidia/{chip}/ inventory
//! ├── pci           — PCI device discovery via /sys/bus/pci/devices/
//! └── monitor       — continuous polling + safety thresholds
//! ```
//!
//! # Phase 0: Read-only monitoring (discover GPUs, read sensors, probe firmware)
//! # Phase 3: BAR0 MMIO register access for software PMU init
//! # Phase 4: Sovereign power management (glow plug, five-state model)

// All nvpmu modules require Linux kernel APIs (sysfs, VFIO, BAR0 MMIO).
#[cfg(target_os = "linux")]
pub mod bar0;
#[cfg(target_os = "linux")]
pub mod dma;
pub mod error;
#[cfg(target_os = "linux")]
pub mod fb;
#[cfg(target_os = "linux")]
pub mod firmware;
#[cfg(target_os = "linux")]
pub mod hwmon;
#[cfg(target_os = "linux")]
pub mod init;
#[cfg(target_os = "linux")]
pub mod monitor;
#[cfg(target_os = "linux")]
pub mod nvidia_smi;
#[cfg(target_os = "linux")]
pub mod pci;
#[cfg(target_os = "linux")]
pub mod permissions;
#[cfg(target_os = "linux")]
pub mod power;
#[cfg(target_os = "linux")]
pub mod power_manager;
#[cfg(target_os = "linux")]
pub mod power_policy;
#[cfg(target_os = "linux")]
pub mod registers;
#[cfg(target_os = "linux")]
#[expect(unsafe_code, reason = "DMA / VFIO BAR0 access requires unsafe")]
pub mod vfio;
#[cfg(target_os = "linux")]
pub mod vfio_bind;
#[cfg(target_os = "linux")]
pub mod watchdog;

#[cfg(target_os = "linux")]
pub use bar0::Bar0Access;
#[cfg(target_os = "linux")]
pub use dma::{DmaAllocator, DmaBuffer, HugePageSize, supports_huge_pages};
pub use error::{NvPmuError, Result};
#[cfg(target_os = "linux")]
pub use fb::{FbPartitionReport, FbStatus};
#[cfg(target_os = "linux")]
pub use firmware::FirmwareInventory;
#[cfg(target_os = "linux")]
pub use hwmon::HwmonSensors;
#[cfg(target_os = "linux")]
pub use init::RegisterSnapshot;
#[cfg(target_os = "linux")]
pub use monitor::{MonitorConfig, SafetyStatus};
#[cfg(target_os = "linux")]
pub use pci::NvidiaGpu;
#[cfg(target_os = "linux")]
pub use power::{GpuPowerController, PciPowerState, ResetMethod};
#[cfg(target_os = "linux")]
pub use power_manager::{ClockGateConfig, GpuPowerState, PowerManager};
#[cfg(target_os = "linux")]
pub use power_policy::{PolicyEngine, PowerPolicy};
#[cfg(target_os = "linux")]
pub use vfio::{VfioBar0Access, VfioMsixInterrupt};
#[cfg(target_os = "linux")]
pub use vfio_bind::{BindResult, BindingState};
