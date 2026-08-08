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
//! Hardware device I/O is inherently platform-specific, but the CAPABILITIES
//! are universal. Every platform needs to:
//! - Map device memory into the process address space
//! - Issue typed control commands to device drivers
//! - Pin memory for DMA
//! - Receive interrupt notifications
//! - Isolate untrusted device operations
//! - Probe process privileges
//! - Pass device handles between processes
//!
//! L3 traits define platform-agnostic interfaces for these capabilities.
//! Concrete implementations use platform-specific mechanisms:
//! - **Linux**: rustix (mmap, ioctl, eventfd, fork, capabilities, SCM_RIGHTS)
//! - **macOS** (future): IOKit, kqueue, posix_spawn, entitlements, Mach ports
//! - **Windows** (future): VirtualAlloc, DeviceIoControl, Events, Job Objects, DuplicateHandle
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
//! | [`DeviceIoctl`] | per-device compile-time typed | `hw-safe/vfio_setup.rs`, `cylinder/vfio/ioctl.rs` |
//! | [`PrivilegeProbe`] | `LinuxPrivilegeProbe` | `sandbox/linux/privilege.rs` |
//! | [`FilesystemIsolation`] | `LinuxSandboxManager` | `sandbox/linux/mod.rs` |
//! | [`FdPassing`] | (cylinder ember_client) | `cylinder/vfio/ember_client.rs` |
//! | [`ProcessIsolation`] | `fork_isolated_raw` (pipe-based) | `cylinder/vfio/isolation.rs` |
//! | [`SystemParameters`] | `LinuxSystemParameters` | `hw-safe/platform_backends.rs` |
//!
//! ## Categories
//!
//! - [`MappedMemory`] — memory-mapped I/O regions (BAR access, DMA buffers)
//! - [`MemoryMapper`] — create memory mappings from files or anonymous regions
//! - [`PinnedMemory`] — mlock/munlock for DMA-safe physical page pinning
//! - [`DeviceFile`] — device node open/read/write with platform flags
//! - [`DeviceIoctl`] — typed control commands to device drivers
//! - [`EventNotifier`] — interrupt-style notification (eventfd on Linux)
//! - [`ProcessIsolation`] — fork/spawn sandboxing for untrusted device ops
//! - [`PrivilegeProbe`] — query process privilege/capability level
//! - [`FilesystemIsolation`] — mount namespace / sandbox filesystem views
//! - [`FdPassing`] — send device handles between processes
//! - [`SystemParameters`] — clock ticks, page size, platform constants

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

// ─── New L3 Traits (S364) ────────────────────────────────────────────────────

/// Typed device control commands.
///
/// Abstracts over platform-specific device control mechanisms:
/// - **Linux**: `ioctl(fd, request, arg)` via rustix
/// - **macOS** (future): `IOConnectCallMethod` / `IOConnectCallStructMethod`
/// - **Windows** (future): `DeviceIoControl(hDevice, dwIoControlCode, ...)`
///
/// The trait is generic over the command type `C` (which encodes the request
/// number and direction) and the argument type `A` (the data structure passed
/// to the kernel).
///
/// # Why not a single `ioctl(fd, u32, *mut c_void)`?
///
/// Type safety. Each ioctl has a specific data layout; encoding that in the
/// type system catches mismatched request/data pairs at compile time.
pub trait DeviceIoctl: Send + Sync {
    /// Error type for ioctl operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Opaque device handle (platform fd / HANDLE / io_connect_t).
    type Handle: Send + Sync;

    /// Issue a read-only control command (device → host).
    ///
    /// `request` is the platform-specific opcode (ioctl number on Linux).
    /// Returns the populated output struct.
    fn control_read<T: Default + Send>(
        &self,
        handle: &Self::Handle,
        request: u32,
    ) -> Result<T, Self::Error>;

    /// Issue a write-only control command (host → device).
    ///
    /// `request` is the platform-specific opcode.
    fn control_write<T: Send>(
        &self,
        handle: &Self::Handle,
        request: u32,
        data: &T,
    ) -> Result<(), Self::Error>;

    /// Issue a read-write control command (bidirectional).
    ///
    /// `request` is the platform-specific opcode.
    /// `data` is modified in place by the kernel.
    fn control_read_write<T: Send>(
        &self,
        handle: &Self::Handle,
        request: u32,
        data: &mut T,
    ) -> Result<(), Self::Error>;
}

/// Privilege and capability probing.
///
/// Abstracts over platform-specific privilege models:
/// - **Linux**: POSIX capabilities (`CAP_SYS_ADMIN`, `CAP_NET_RAW`, etc.)
/// - **macOS** (future): code-signing entitlements
/// - **Windows** (future): security token privileges (`SeDebugPrivilege`, etc.)
///
/// ToadStool uses this to determine at runtime what device operations are
/// available without attempting them and handling permission errors.
pub trait PrivilegeProbe: Send + Sync {
    /// Check whether the current process has a named privilege.
    ///
    /// `privilege` is a platform-agnostic capability name. Implementations map
    /// it to the platform equivalent:
    /// - `"sys_admin"` → Linux `CAP_SYS_ADMIN`, macOS root check, Windows Admin
    /// - `"net_raw"` → Linux `CAP_NET_RAW`, macOS BPF access
    /// - `"device_passthrough"` → Linux `CAP_SYS_RAWIO` + IOMMU, macOS IOKit
    fn has_privilege(&self, privilege: &str) -> bool;

    /// Return the full set of active privileges as platform-agnostic names.
    fn active_privileges(&self) -> Vec<&'static str>;

    /// Whether the process is running with elevated (root/admin) permissions.
    fn is_elevated(&self) -> bool;
}

/// Filesystem isolation — restrict a process's view of the filesystem.
///
/// Abstracts over platform-specific sandboxing mechanisms:
/// - **Linux**: mount namespaces (`mount --bind`, `pivot_root`, tmpfs)
/// - **macOS** (future): `sandbox_init` profiles, App Sandbox
/// - **Windows** (future): AppContainer / Job Object filesystem filters
///
/// Used by toadStool's sandbox manager to create isolated execution
/// environments for untrusted workloads.
pub trait FilesystemIsolation: Send + Sync {
    /// Error type for filesystem isolation operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Mount a host path into the isolated view.
    ///
    /// - `source`: host filesystem path
    /// - `target`: path inside the isolated namespace
    /// - `read_only`: whether writes should be blocked
    fn bind_mount(&self, source: &Path, target: &Path, read_only: bool) -> Result<(), Self::Error>;

    /// Create a temporary filesystem at `target` (not backed by host storage).
    fn mount_tmpfs(&self, target: &Path) -> Result<(), Self::Error>;

    /// Remove a previously created mount.
    fn unmount(&self, target: &Path) -> Result<(), Self::Error>;
}

/// Pass device handles between processes.
///
/// Abstracts over platform-specific handle-passing mechanisms:
/// - **Linux**: `SCM_RIGHTS` over Unix domain sockets (sendmsg/recvmsg)
/// - **macOS** (future): Mach port rights (`mach_port_insert_right`)
/// - **Windows** (future): `DuplicateHandle` with target process handle
///
/// Used by cylinder's ember client to receive VFIO device fds from a
/// privileged setup process.
pub trait FdPassing: Send + Sync {
    /// Error type for handle-passing operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Platform-agnostic device handle (fd on Unix, HANDLE on Windows).
    type Handle: Send;

    /// Receive a device handle from a connected peer.
    ///
    /// Blocks until a handle is available or returns error on disconnect.
    fn receive_handle(&self) -> Result<Self::Handle, Self::Error>;

    /// Send a device handle to a connected peer.
    fn send_handle(&self, handle: &Self::Handle) -> Result<(), Self::Error>;
}

/// System parameters — platform constants needed for hardware calculations.
///
/// Abstracts over platform-specific runtime parameters:
/// - **Linux**: `clock_ticks_per_second()` (sysconf), page size
/// - **macOS** (future): `sysctl` values
/// - **Windows** (future): `GetSystemInfo`
///
/// Used for jiffies→seconds conversion, page-aligned allocation, etc.
pub trait SystemParameters: Send + Sync {
    /// Clock ticks (jiffies) per second for CPU time calculations.
    ///
    /// Linux: typically 100 (HZ=100) or 250.
    fn clock_ticks_per_second(&self) -> u64;

    /// System page size in bytes (typically 4096).
    fn page_size(&self) -> usize;

    /// Huge page size in bytes, if available (typically 2MB or 1GB).
    ///
    /// Returns `None` if the platform doesn't support huge pages.
    fn huge_page_size(&self) -> Option<usize>;
}
