// SPDX-License-Identifier: AGPL-3.0-or-later
#![warn(missing_docs)]

//! # toadstool-hw-safe — Safe Wrappers for Hardware Primitives
//!
//! This crate is toadStool's **single unsafe containment zone**. Every other
//! crate in the workspace uses `#![forbid(unsafe_code)]` and depends on
//! this crate for hardware-level operations.
//!
//! ## What lives here
//!
//! - [`SafeMmapRegion`] — RAII memory-mapped file region (mmap/munmap)
//! - [`VolatileMmio`] — bounds-checked volatile MMIO register access
//! - [`AlignedAlloc`] — heap allocation with arbitrary alignment
//! - [`LockedMemory`] — mlock/munlock for DMA-safe and secure memory
//!
//! ## Design principle
//!
//! Each type encapsulates the minimum unsafe needed for its operation.
//! The public API is entirely safe. All `unsafe` blocks have `// SAFETY:`
//! comments documenting invariants.
//!
//! The goal is to reduce this crate's unsafe surface to the irreducible
//! minimum (~26 operations), then iterate each one toward pure Rust
//! alternatives (e.g. `memmap2` for mmap, `aligned-vec` for allocation).

#[cfg(target_os = "linux")]
pub mod aligned_alloc;
#[cfg(target_os = "linux")]
mod contiguous;
#[cfg(target_os = "linux")]
pub mod device_mmap;
#[cfg(target_os = "linux")]
pub mod drm_ioctl;
#[cfg(target_os = "linux")]
mod exclusive_ptr;
#[cfg(target_os = "linux")]
pub mod huge_page;
#[cfg(target_os = "linux")]
pub mod locked_memory;
#[cfg(target_os = "linux")]
pub mod safe_mmap;
#[cfg(target_os = "linux")]
pub mod systemd_fds;
#[cfg(target_os = "linux")]
pub mod vfio_dma;
#[cfg(target_os = "linux")]
pub mod vfio_setup;
#[cfg(target_os = "linux")]
pub mod volatile_mmio;

#[cfg(target_os = "linux")]
pub mod platform_backends;

#[cfg(target_os = "linux")]
pub use aligned_alloc::AlignedAlloc;
#[cfg(target_os = "linux")]
pub use contiguous::ContiguousBytes;
#[cfg(target_os = "linux")]
pub use device_mmap::DeviceMmap;
#[cfg(target_os = "linux")]
pub use huge_page::HugePageMemory;
#[cfg(target_os = "linux")]
pub use locked_memory::LockedMemory;
#[cfg(target_os = "linux")]
pub use platform_backends::{
    ForkResult, FsStats, LinuxDeviceFile, LinuxDeviceIo, LinuxEvent, LinuxEventNotifier,
    LinuxFilesystemIsolation, LinuxMemoryMapper, LinuxPinnedMemory, LinuxPrivilegeProbeBackend,
    LinuxSystemParameters, Pid, UnixAddr, WaitResult, clock_monotonic_ns, delete_module,
    exit_group, finit_module, fork, fs_stats, getpid, ioctl_infra, kill_process, lock_memory,
    mknod_char, mmap_device, munmap_device, open_path, pipe_cloexec, recv_with_fds, seek_end,
    send_signal, sendmsg_with_fds, unix_dgram_socket, unlock_memory, vfio_bar_map, vfio_bar_unmap,
    waitpid_nohang,
};
#[cfg(target_os = "linux")]
pub use safe_mmap::SafeMmapRegion;
#[cfg(target_os = "linux")]
pub use volatile_mmio::MmioError;
#[cfg(target_os = "linux")]
pub use volatile_mmio::VolatileMmio;

#[cfg(target_os = "linux")]
pub(crate) use exclusive_ptr::ExclusivePtr;
