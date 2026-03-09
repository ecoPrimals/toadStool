// SPDX-License-Identifier: AGPL-3.0-only
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

#![deny(unsafe_code)]

pub mod cpu;
pub mod disk;
pub mod error;
pub mod loadavg;
pub mod memory;
pub mod network;
pub mod process;

pub use cpu::{cpu_brand, cpu_count, cpu_usage, per_cpu_usage};
pub use disk::{disk_usage, DiskInfo};
pub use error::SysmonError;
pub use loadavg::{load_average, LoadAverage};
pub use memory::{memory_info, MemoryInfo};
pub use network::{network_stats, NetworkInterface};
pub use process::{all_processes, process_count, process_info, ProcessInfo};
