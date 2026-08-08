// SPDX-License-Identifier: AGPL-3.0-or-later

//! # Platform Substrate Abstraction (G68)
//!
//! Cross-platform primitives for filesystem operations that historically used
//! raw `std::os::unix` APIs behind `#[cfg(unix)]` blocks.
//!
//! ## Layers
//!
//! - **L1 Links** — [`platform_link`]: symlink on unix, symlink/junction on Windows
//! - **L2 Access** — [`PlatformAccess`] + [`set_access`] / [`check_access`]:
//!   semantic permission intent (mode bits on unix, best-effort on Windows)
//! - **L3 Device I/O** — [`device_io`]: backend traits for memory-mapped I/O,
//!   device files, event notification, process isolation, and pinned memory.
//!   Implementations live in hardware crates behind `#[cfg(unix)]`.
//!
//! ## Design
//!
//! The G68 test: "Does this primal do *less* on Windows, or the *same thing differently*?"
//! These abstractions ensure the primal does the **same thing differently** on each platform.

pub mod access;
pub mod device_io;
pub mod links;

pub use access::{PlatformAccess, check_access, set_access};
pub use device_io::{
    DeviceFile, DeviceIoctl, EventNotifier, FdPassing, FilesystemIsolation, MappedMemory,
    MemoryMapper, PinnedMemory, PrivilegeProbe, ProcessIsolation, SystemParameters,
};
pub use links::platform_link;
