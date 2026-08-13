// SPDX-License-Identifier: AGPL-3.0-or-later

//! [`toadstool_common::platform`] trait implementations for Linux.
//!
//! Provides concrete `MemoryMapper` and `PinnedMemory` implementations using
//! `rustix` syscalls. These are the "L3 backend" for the G68 platform traits.
//!
//! Data-only types (`WaitResult`, `FsStats`, `UnixAddr`) are available on all
//! platforms. All functions and trait impls are Linux-only (gated internally).

mod device_io;
mod event;
mod ipc;
mod isolation;
mod kmod;
mod memory;
mod system;

pub use device_io::{
    LinuxDeviceFile, LinuxDeviceIo, mknod_char, open_path, seek_end,
};
pub use event::{LinuxEvent, LinuxEventNotifier};
pub use ipc::{
    UnixAddr, ioctl_infra, recv_with_fds, sendmsg_with_fds, unix_dgram_socket,
};
pub use isolation::LinuxFilesystemIsolation;
pub use kmod::{delete_module, finit_module};
pub use memory::{
    LinuxMemoryMapper, LinuxPinnedMemory, lock_memory, mmap_device, munmap_device,
    unlock_memory, vfio_bar_map, vfio_bar_unmap,
};
pub use system::{
    FsStats, LinuxPrivilegeProbeBackend, LinuxSystemParameters, clock_monotonic_ns, fs_stats,
};
