// SPDX-License-Identifier: AGPL-3.0-or-later
#![warn(missing_docs)]

//! # toadstool-hw-safe — Safe Wrappers for Hardware Primitives
//!
//! This crate is toadStool's **single unsafe containment zone**. Every other
//! crate in the workspace uses `#![forbid(unsafe_code)]` and depends on
//! this crate for hardware-level operations.
//!
//! ## Cross-Platform Architecture
//!
//! The crate is structured in layers:
//!
//! **Layer 0 — Pure Rust (all platforms):**
//! - [`AlignedAlloc`] — heap allocation with arbitrary alignment
//! - [`VolatileMmio`] — bounds-checked volatile MMIO register access
//! - [`ExclusivePtr`](exclusive_ptr) — ownership-tracked raw pointer
//! - [`ContiguousBytes`] — trait for contiguous memory regions
//!
//! **Layer 1 — Memory management (Linux, with cross-platform stubs):**
//! - [`SafeMmapRegion`] — RAII memory-mapped file region
//! - [`DeviceMmap`] — RAII device fd mmap with offset
//! - [`LockedMemory`] — mlock/munlock for DMA-safe and secure memory
//! - [`HugePageMemory`] — locked huge-page allocation
//!
//! **Layer 2 — Device backends (Linux-only kernel ABI):**
//! - [`vfio_setup`] — VFIO container/group/device ioctls
//! - [`vfio_dma`] — VFIO IOMMU DMA map/unmap
//! - [`drm_ioctl`] — DRM ioctl execution
//! - [`platform_backends`] — process, socket, filesystem operations
//!
//! On non-Linux targets, Layer 1 types exist but constructors return
//! `Err(Unsupported)`. Layer 2 modules exist on all platforms (types are
//! unconditional), but functions are internally `#[cfg(target_os = "linux")]`.

// ── Layer 0: Pure Rust (unconditional) ───────────────────────────────────

pub mod aligned_alloc;
mod contiguous;
mod exclusive_ptr;
pub mod volatile_mmio;

pub use aligned_alloc::AlignedAlloc;
pub use contiguous::ContiguousBytes;
pub use volatile_mmio::MmioError;
pub use volatile_mmio::VolatileMmio;

pub(crate) use exclusive_ptr::ExclusivePtr;

// ── Layer 1: Memory management (types unconditional, impl gated) ─────────

pub mod device_mmap;
pub mod huge_page;
pub mod locked_memory;
pub mod safe_mmap;

pub use device_mmap::DeviceMmap;
pub use huge_page::HugePageMemory;
pub use locked_memory::LockedMemory;
pub use safe_mmap::SafeMmapRegion;

// ── Layer 2: Device backends (types unconditional, impl Linux-gated) ──────

pub mod drm_ioctl;
pub mod platform_backends;
pub mod vfio_dma;
pub mod vfio_setup;

#[cfg(target_os = "linux")]
pub mod systemd_fds;

// Cross-platform re-exports from Layer 2 (data-only types).
pub use platform_backends::{FsStats, UnixAddr, WaitResult};

#[cfg(target_os = "linux")]
pub use platform_backends::{
    ForkResult, LinuxDeviceFile, LinuxDeviceIo, LinuxEvent, LinuxEventNotifier,
    LinuxFilesystemIsolation, LinuxMemoryMapper, LinuxPinnedMemory, LinuxPrivilegeProbeBackend,
    LinuxSystemParameters, Pid, clock_monotonic_ns, delete_module, exit_group, finit_module, fork,
    fs_stats, getpid, ioctl_infra, kill_process, lock_memory, mknod_char, mmap_device,
    munmap_device, open_path, pipe_cloexec, recv_with_fds, seek_end, send_signal, sendmsg_with_fds,
    unix_dgram_socket, unlock_memory, vfio_bar_map, vfio_bar_unmap, waitpid_nohang,
};
