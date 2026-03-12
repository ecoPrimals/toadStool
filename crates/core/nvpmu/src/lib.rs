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
#[allow(unsafe_code)]
pub mod bar0;
pub mod error;
pub mod firmware;
pub mod hwmon;
#[allow(unsafe_code)]
pub mod init;
pub mod monitor;
pub mod nvidia_smi;
pub mod pci;
pub mod watchdog;

pub use error::{NvPmuError, Result};
pub use firmware::FirmwareInventory;
pub use hwmon::HwmonSensors;
pub use bar0::Bar0Access;
pub use monitor::{MonitorConfig, SafetyStatus};
pub use pci::NvidiaGpu;
