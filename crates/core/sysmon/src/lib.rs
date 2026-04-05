// SPDX-License-Identifier: AGPL-3.0-or-later
//! # toadstool-sysmon
//!
//! Pure Rust system monitoring via `/proc` filesystem parsing.
//! Replaces `sysinfo` crate, eliminating all transitive C/`libc` dependencies.
//!
//! ## Design
//!
//! - **Linux-first**: All data sourced from `/proc` and `statvfs` syscalls.
//! - **No mega-object**: Individual query functions instead of a `System` object
//!   that refreshes everything.
//! - **Zero C**: Uses `rustix` for `statvfs` (direct syscalls, no libc FFI).
//!   Everything else is `std::fs::read_to_string` + parsing.
//!
//! ## Usage
//!
//! ```no_run
//! use toadstool_sysmon::{memory_info, cpu_count, load_average, disk_usage};
//!
//! let mem = memory_info().unwrap();
//! println!("RAM: {} / {} bytes", mem.used, mem.total);
//!
//! println!("CPUs: {}", cpu_count());
//!
//! let la = load_average().unwrap();
//! println!("Load: {:.2} {:.2} {:.2}", la.one, la.five, la.fifteen);
//!
//! for disk in disk_usage().unwrap() {
//!     println!("{}: {} free of {}", disk.mount_point, disk.available_space, disk.total_space);
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod cpu;
pub mod disk;
pub mod error;
pub mod gpu;
pub mod loadavg;
pub mod memory;
pub mod network;
pub mod pcie_topology;
pub mod process;
pub mod system;

pub use cpu::{cpu_brand, cpu_count, cpu_usage, per_cpu_usage};
pub use disk::{DiskInfo, disk_usage};
pub use error::SysmonError;
pub use gpu::{
    FirmwareInventory, FwStatus, GpuDevice, GpuTelemetry, GpuVendor, PcieTopology, discover_gpus,
};
pub use loadavg::{LoadAverage, load_average};
pub use memory::{MemoryInfo, memory_info};
pub use network::{NetworkInterface, network_stats};
pub use pcie_topology::{
    GpuPairTopology, PciBridge, PcieTopologyGraph, discover_topology, raw_pcie_bandwidth_bps,
};
pub use process::{ProcessInfo, all_processes, process_count, process_info};
