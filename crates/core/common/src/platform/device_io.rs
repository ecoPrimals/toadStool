// SPDX-License-Identifier: AGPL-3.0-or-later

//! # Device I/O Backend Traits (G68 L3)
//!
//! Abstract interfaces for hardware device operations that use platform-specific
//! syscalls (ioctl, mmap, eventfd, etc.). These traits define the contract;
//! concrete implementations live in the hardware crates (`hw-safe`) behind
//! `#[cfg(target_os = "linux")]`.
//!
//! ## Design Principle
//!
//! Hardware device I/O is inherently platform-specific. Rather than pretending
//! Windows can do VFIO ioctls, L3 traits enable:
//! 1. **Mockability** — inject test doubles without `#[cfg(test)]` pollution
//! 2. **Documentation** — the trait surface documents what operations exist
//! 3. **Future portability** — if a platform gains support, add an impl
//!
//! ## Implementation Status
//!
//! | Trait | Concrete impl | Location |
//! |-------|--------------|----------|
//! | [`MappedMemory`] | `SafeMmapRegion` | `hw-safe/safe_mmap.rs` |
//! | [`MemoryMapper`] | `LinuxMemoryMapper` | `hw-safe/platform_backends.rs` |
//! | [`PinnedMemory`] | `LinuxPinnedMemory` | `hw-safe/platform_backends.rs` |
//! | [`DeviceFile`] | `LinuxDeviceFile` | `hw-safe/platform_backends.rs` |
//! | [`EventNotifier`] | `LinuxEventNotifier` | `hw-safe/platform_backends.rs` |
//! | [`ProcessIsolation`] | `fork_isolated_raw` (pipe-based) | `cylinder/vfio/isolation.rs` |
//!
//! ## Categories
//!
//! - [`MappedMemory`] — memory-mapped I/O regions (BAR access, DMA buffers)
//! - [`DeviceFile`] — device node open/read/write with platform flags
//! - [`EventNotifier`] — interrupt-style notification (eventfd on Linux)
//! - [`ProcessIsolation`] — fork-and-exec sandboxing for untrusted device ops
//! - [`PinnedMemory`] — mlock/munlock for DMA-safe physical page pinning

use std::path::Path;
use std::time::Duration;

/// A memory-mapped I/O region.
///
/// Wraps the lifecycle of a `mmap`/`munmap` pair. Implementations must ensure
/// the mapping is valid for the lifetime of the handle and unmapped on drop.
///
/// # Safety Contract
///
/// Implementors guarantee:
/// - The returned pointer is valid and aligned for the mapped length
/// - The mapping is unmapped on drop (no dangling pointers)
/// - Concurrent access is the caller's responsibility (volatile reads/writes)
pub trait MappedMemory: Send + Sync {
    /// Base pointer of the mapped region.
    fn as_ptr(&self) -> *const u8;

    /// Mutable base pointer of the mapped region.
    fn as_mut_ptr(&mut self) -> *mut u8;

    /// Length in bytes of the mapped region.
    fn len(&self) -> usize;

    /// Whether the mapped region is zero-length.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Memory mapper — creates [`MappedMemory`] handles from file descriptors or
/// anonymous regions.
pub trait MemoryMapper: Send + Sync {
    /// The concrete mapped-memory handle type.
    type Mapping: MappedMemory;

    /// Error type for mapping operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Map a file region into memory.
    ///
    /// - `offset`: byte offset into the file
    /// - `length`: number of bytes to map
    /// - `writable`: whether writes to the region should propagate
    fn map_file(
        &self,
        path: &Path,
        offset: u64,
        length: usize,
        writable: bool,
    ) -> Result<Self::Mapping, Self::Error>;

    /// Create an anonymous (non-file-backed) memory mapping.
    ///
    /// Useful for DMA buffers and huge-page allocations.
    fn map_anonymous(&self, length: usize) -> Result<Self::Mapping, Self::Error>;
}

/// Pinned (locked) memory that will not be paged out.
///
/// Required for DMA operations where the physical address must remain stable.
/// The implementor is responsible for validating pointer/length arguments
/// internally — callers provide safe Rust references.
pub trait PinnedMemory: Send + Sync {
    /// Error type for pin operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Pin a memory region (prevent page-out).
    ///
    /// The implementation must validate that the slice describes a mapped region
    /// before issuing the underlying syscall.
    fn pin(&self, region: &[u8]) -> Result<(), Self::Error>;

    /// Unpin a previously pinned memory region.
    ///
    /// The implementation should tolerate double-unpin gracefully.
    fn unpin(&self, region: &[u8]) -> Result<(), Self::Error>;
}

/// Device file operations — open, read, write with platform-specific flags.
///
/// Abstracts over `rustix::fs::OFlags` and raw fd operations.
pub trait DeviceFile: Send + Sync {
    /// Error type for device file operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Opaque handle to an opened device.
    type Handle: Send + Sync;

    /// Open a device node at `path`.
    ///
    /// - `writable`: open for read-write vs read-only
    /// - `non_blocking`: O_NONBLOCK equivalent
    fn open(
        &self,
        path: &Path,
        writable: bool,
        non_blocking: bool,
    ) -> Result<Self::Handle, Self::Error>;

    /// Read bytes from a device handle.
    fn read(&self, handle: &Self::Handle, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Write bytes to a device handle.
    fn write(&self, handle: &Self::Handle, buf: &[u8]) -> Result<usize, Self::Error>;
}

/// Interrupt-style event notification.
///
/// On Linux this wraps `eventfd` + `poll`. Other platforms may use different
/// mechanisms (Windows events, kqueue, etc.).
pub trait EventNotifier: Send + Sync {
    /// Error type for event operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Opaque event handle.
    type Event: Send + Sync;

    /// Create a new event notification channel.
    fn create(&self) -> Result<Self::Event, Self::Error>;

    /// Signal (increment) the event.
    fn signal(&self, event: &Self::Event) -> Result<(), Self::Error>;

    /// Wait for the event to be signaled, with timeout.
    ///
    /// Returns `Ok(Some(count))` if signaled, `Ok(None)` on timeout.
    fn wait(&self, event: &Self::Event, timeout: Duration) -> Result<Option<u64>, Self::Error>;
}

/// Process isolation — fork a child process for untrusted device operations.
///
/// Used by cylinder's guarded sysfs writes and kmod loading where a crash
/// in the child should not take down the parent.
///
/// # Implementation Note
///
/// This trait defines the contract for testability and documentation purposes.
/// The actual Linux implementation (`fork_isolated_raw` in `cylinder::vfio::isolation`)
/// uses a lower-level pipe-based interface because the child process must be
/// async-signal-safe (no heap allocation after fork). The `Box<dyn FnOnce>`
/// interface here is suitable for high-level orchestration and test doubles.
pub trait ProcessIsolation: Send + Sync {
    /// Error type for process operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Spawn an isolated child process that executes `work`.
    ///
    /// Returns the child's output bytes on success, or an error if the child
    /// crashed, timed out, or was killed.
    fn spawn_isolated(
        &self,
        timeout: Duration,
        work: Box<dyn FnOnce() -> Vec<u8> + Send>,
    ) -> Result<Vec<u8>, Self::Error>;
}
