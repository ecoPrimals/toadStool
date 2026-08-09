// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "mmap/mlock require unsafe — containment zone")]

//! [`toadstool_common::platform`] trait implementations for Linux.
//!
//! Provides concrete `MemoryMapper` and `PinnedMemory` implementations using
//! `rustix` syscalls. These are the "L3 backend" for the G68 platform traits.
//!
//! Data-only types (`WaitResult`, `FsStats`, `UnixAddr`) are available on all
//! platforms. All functions and trait impls are Linux-only (gated internally).

#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
use toadstool_common::platform;

#[cfg(target_os = "linux")]
use crate::safe_mmap::{MmapError, SafeMmapRegion};

/// Linux memory mapper — creates [`SafeMmapRegion`] handles from file paths.
///
/// Implements [`platform::MemoryMapper`] using `rustix::mm::mmap`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxMemoryMapper;

#[cfg(target_os = "linux")]
impl platform::MemoryMapper for LinuxMemoryMapper {
    type Mapping = SafeMmapRegion;
    type Error = MmapError;

    fn map_file(
        &self,
        path: &Path,
        _offset: u64,
        _length: usize,
        writable: bool,
    ) -> Result<Self::Mapping, Self::Error> {
        if writable {
            SafeMmapRegion::map_shared_rw(path)
        } else {
            SafeMmapRegion::map_shared_ro(path)
        }
    }

    fn map_anonymous(&self, length: usize) -> Result<Self::Mapping, Self::Error> {
        use std::ptr;

        if length == 0 {
            return Err(MmapError::ZeroSize {
                path: "<anonymous>".to_string(),
            });
        }

        // SAFETY: null hint, valid length, anonymous mapping (no fd).
        let raw = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                length,
                rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                rustix::mm::MapFlags::PRIVATE,
            )
        }
        .map_err(|e| MmapError::MmapFailed {
            path: "<anonymous>".to_string(),
            source: e.into(),
        })?;

        let ptr = std::ptr::NonNull::new(raw.cast()).ok_or_else(|| MmapError::NullPointer {
            path: "<anonymous>".to_string(),
        })?;

        Ok(SafeMmapRegion::from_anonymous(ptr, length))
    }
}

/// Linux memory pinner — mlock/munlock for DMA-safe page pinning.
///
/// Implements [`platform::PinnedMemory`] using `rustix::mm::mlock`/`munlock`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPinnedMemory;

#[cfg(target_os = "linux")]
impl platform::PinnedMemory for LinuxPinnedMemory {
    type Error = std::io::Error;

    fn pin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice, so ptr + len describe a valid memory range.
        // mlock requires *mut but only reads the pages (no modification).
        unsafe {
            rustix::mm::mlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }

    fn unpin(&self, region: &[u8]) -> Result<(), Self::Error> {
        // SAFETY: region is a valid slice from a previously locked mapping.
        unsafe {
            rustix::mm::munlock(
                region.as_ptr().cast_mut().cast::<std::ffi::c_void>(),
                region.len(),
            )
        }
        .map_err(std::io::Error::from)
    }
}

/// Linux event notification channel — eventfd + poll.
///
/// Implements [`platform::EventNotifier`] using `rustix::event::eventfd`/`poll`.
/// Used by VFIO IRQ wiring and NVPmu interrupt handling.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxEventNotifier;

/// Opaque event handle wrapping a Linux eventfd.
#[cfg(target_os = "linux")]
pub struct LinuxEvent {
    fd: rustix::fd::OwnedFd,
}

#[cfg(target_os = "linux")]
impl std::fmt::Debug for LinuxEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::os::fd::AsFd;
        f.debug_struct("LinuxEvent")
            .field("fd", &self.fd.as_fd())
            .finish()
    }
}

#[cfg(target_os = "linux")]
impl LinuxEvent {
    /// Access the underlying fd for integration with external epoll/VFIO wiring.
    pub fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.fd.as_fd()
    }

    /// Consume and return the owned fd (for passing to VFIO SET_IRQS).
    pub fn into_fd(self) -> std::os::fd::OwnedFd {
        self.fd
    }

    /// Access the underlying fd as a raw fd for ioctl integration.
    pub fn as_raw_fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.fd.as_raw_fd()
    }
}

// SAFETY: OwnedFd is Send; eventfd has no thread-local state.
#[cfg(target_os = "linux")]
unsafe impl Send for LinuxEvent {}
// SAFETY: eventfd read/write are atomic; concurrent poll is safe.
#[cfg(target_os = "linux")]
unsafe impl Sync for LinuxEvent {}

#[cfg(target_os = "linux")]
impl platform::EventNotifier for LinuxEventNotifier {
    type Error = std::io::Error;
    type Event = LinuxEvent;

    fn create(&self) -> Result<Self::Event, Self::Error> {
        let fd = rustix::event::eventfd(0, rustix::event::EventfdFlags::NONBLOCK)
            .map_err(std::io::Error::from)?;
        Ok(LinuxEvent { fd })
    }

    fn signal(&self, event: &Self::Event) -> Result<(), Self::Error> {
        let buf = 1u64.to_ne_bytes();
        rustix::io::write(&event.fd, &buf).map_err(std::io::Error::from)?;
        Ok(())
    }

    fn wait(
        &self,
        event: &Self::Event,
        timeout: std::time::Duration,
    ) -> Result<Option<u64>, Self::Error> {
        use std::os::fd::AsFd;

        let ts = rustix::time::Timespec {
            tv_sec: timeout.as_secs() as i64,
            tv_nsec: i64::from(timeout.subsec_nanos()),
        };
        let borrowed = event.fd.as_fd();
        let mut pfds = [rustix::event::PollFd::new(
            &borrowed,
            rustix::event::PollFlags::IN,
        )];

        match rustix::event::poll(&mut pfds, Some(&ts)) {
            Ok(n) if n > 0 => {
                let mut buf = [0u8; 8];
                match rustix::io::read(&event.fd, &mut buf) {
                    Ok(8) => Ok(Some(u64::from_ne_bytes(buf))),
                    _ => Ok(Some(1)),
                }
            }
            Ok(_) => Ok(None),
            Err(e) => Err(std::io::Error::from(e)),
        }
    }
}

/// Linux device file opener — wraps `rustix::fs::open` with platform flags.
///
/// Implements [`platform::DeviceFile`] for opening `/dev/*` nodes with
/// appropriate `O_RDWR | O_CLOEXEC` (and optionally `O_NONBLOCK`).
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxDeviceFile;

#[cfg(target_os = "linux")]
impl platform::DeviceFile for LinuxDeviceFile {
    type Error = std::io::Error;
    type Handle = rustix::fd::OwnedFd;

    fn open(
        &self,
        path: &Path,
        writable: bool,
        non_blocking: bool,
    ) -> Result<Self::Handle, Self::Error> {
        let mut flags = rustix::fs::OFlags::CLOEXEC;
        if writable {
            flags |= rustix::fs::OFlags::RDWR;
        } else {
            flags |= rustix::fs::OFlags::RDONLY;
        }
        if non_blocking {
            flags |= rustix::fs::OFlags::NONBLOCK;
        }

        rustix::fs::open(path, flags, rustix::fs::Mode::empty()).map_err(std::io::Error::from)
    }

    fn read(&self, handle: &Self::Handle, buf: &mut [u8]) -> Result<usize, Self::Error> {
        rustix::io::read(handle, buf).map_err(std::io::Error::from)
    }

    fn write(&self, handle: &Self::Handle, buf: &[u8]) -> Result<usize, Self::Error> {
        rustix::io::write(handle, buf).map_err(std::io::Error::from)
    }
}

/// Linux system parameters — clock ticks, page size, huge pages.
///
/// Implements [`platform::SystemParameters`] via `rustix::param`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxSystemParameters;

#[cfg(target_os = "linux")]
impl platform::SystemParameters for LinuxSystemParameters {
    fn clock_ticks_per_second(&self) -> u64 {
        rustix::param::clock_ticks_per_second()
    }

    fn page_size(&self) -> usize {
        rustix::param::page_size()
    }

    fn huge_page_size(&self) -> Option<usize> {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|info| {
                info.lines()
                    .find(|l| l.starts_with("Hugepagesize:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|kb| kb.parse::<usize>().ok())
                    .map(|kb| kb * 1024)
            })
    }
}

/// Linux privilege probe — capability-based privilege checking.
///
/// Implements [`platform::PrivilegeProbe`] using `rustix::thread::capabilities`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPrivilegeProbeBackend;

#[cfg(target_os = "linux")]
impl platform::PrivilegeProbe for LinuxPrivilegeProbeBackend {
    fn has_privilege(&self, privilege: &str) -> bool {
        let Ok(caps) = rustix::thread::capabilities(None) else {
            return false;
        };
        match privilege {
            "sys_admin" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::SYS_ADMIN),
            "net_raw" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::NET_RAW),
            "sys_rawio" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::SYS_RAWIO),
            "dac_override" => caps
                .effective
                .contains(rustix::thread::CapabilitySet::DAC_OVERRIDE),
            _ => false,
        }
    }

    fn active_privileges(&self) -> Vec<&'static str> {
        let Ok(caps) = rustix::thread::capabilities(None) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::SYS_ADMIN)
        {
            result.push("sys_admin");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::NET_RAW)
        {
            result.push("net_raw");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::SYS_RAWIO)
        {
            result.push("sys_rawio");
        }
        if caps
            .effective
            .contains(rustix::thread::CapabilitySet::DAC_OVERRIDE)
        {
            result.push("dac_override");
        }
        result
    }

    fn is_elevated(&self) -> bool {
        self.has_privilege("sys_admin")
    }
}

/// Linux filesystem isolation — mount namespace operations.
///
/// Implements [`platform::FilesystemIsolation`] using `rustix::mount`.
#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxFilesystemIsolation;

#[cfg(target_os = "linux")]
impl platform::FilesystemIsolation for LinuxFilesystemIsolation {
    type Error = std::io::Error;

    fn bind_mount(&self, source: &Path, target: &Path, read_only: bool) -> Result<(), Self::Error> {
        rustix::mount::mount_bind(source, target).map_err(std::io::Error::from)?;
        if read_only {
            rustix::mount::mount_remount(
                target,
                rustix::mount::MountFlags::RDONLY | rustix::mount::MountFlags::BIND,
                "",
            )
            .map_err(std::io::Error::from)?;
        }
        Ok(())
    }

    fn mount_tmpfs(&self, target: &Path) -> Result<(), Self::Error> {
        rustix::mount::mount(
            "tmpfs",
            target,
            "tmpfs",
            rustix::mount::MountFlags::empty(),
            Option::<&std::ffi::CStr>::None,
        )
        .map_err(std::io::Error::from)
    }

    fn mount_virtual(&self, target: &Path, fstype: &str) -> Result<(), Self::Error> {
        rustix::mount::mount(
            fstype,
            target,
            fstype,
            rustix::mount::MountFlags::empty(),
            Option::<&std::ffi::CStr>::None,
        )
        .map_err(std::io::Error::from)
    }

    fn unmount(&self, target: &Path) -> Result<(), Self::Error> {
        rustix::mount::unmount(target, rustix::mount::UnmountFlags::empty())
            .map_err(std::io::Error::from)
    }
}

// ─── Device I/O ──────────────────────────────────────────────────────────────

/// Safe device I/O operations on borrowed file descriptors.
///
/// Provides `read`, `write`, and `poll` wrappers using `rustix::io` so that
/// consumer crates don't need to depend on rustix directly.
#[cfg(target_os = "linux")]
pub struct LinuxDeviceIo;

#[cfg(target_os = "linux")]
impl LinuxDeviceIo {
    /// Read from a borrowed file descriptor into `buf`.
    ///
    /// Returns the number of bytes read, or an I/O error.
    #[inline]
    pub fn read(fd: std::os::fd::BorrowedFd<'_>, buf: &mut [u8]) -> std::io::Result<usize> {
        rustix::io::read(fd, buf).map_err(std::io::Error::from)
    }

    /// Write `data` to a borrowed file descriptor.
    ///
    /// Returns the number of bytes written, or an I/O error.
    #[inline]
    pub fn write(fd: std::os::fd::BorrowedFd<'_>, data: &[u8]) -> std::io::Result<usize> {
        rustix::io::write(fd, data).map_err(std::io::Error::from)
    }

    /// Poll a file descriptor for readability with an optional timeout.
    ///
    /// Returns `Ok(true)` if data is available, `Ok(false)` on timeout.
    /// Pass `None` for infinite blocking.
    pub fn poll_read(
        fd: std::os::fd::BorrowedFd<'_>,
        timeout_ms: Option<i32>,
    ) -> std::io::Result<bool> {
        let ts = timeout_ms.map(|ms| rustix::time::Timespec {
            tv_sec: i64::from(ms / 1000),
            tv_nsec: i64::from(ms % 1000) * 1_000_000,
        });
        let mut pfds = [rustix::event::PollFd::new(
            &fd,
            rustix::event::PollFlags::IN,
        )];
        let n = rustix::event::poll(&mut pfds, ts.as_ref()).map_err(std::io::Error::from)?;
        Ok(n > 0)
    }
}

// ─── VFIO BAR Mapping ────────────────────────────────────────────────────────

/// Maps a VFIO device BAR region into the process address space.
///
/// Returns a raw pointer to the mapped region. Caller is responsible for
/// unmapping via [`vfio_bar_unmap`] and ensuring volatile access semantics.
///
/// # Safety
///
/// The caller must ensure:
/// - `device_fd` is a valid VFIO device file descriptor
/// - `size` and `offset` were obtained from a valid VFIO_DEVICE_GET_REGION_INFO ioctl
/// - The returned pointer is not used after [`vfio_bar_unmap`] is called
#[cfg(target_os = "linux")]
pub unsafe fn vfio_bar_map(
    device_fd: std::os::fd::BorrowedFd<'_>,
    size: usize,
    offset: u64,
) -> std::io::Result<*mut u8> {
    // SAFETY: caller guarantees fd, size, offset are valid from VFIO region info.
    let ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            size,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            device_fd,
            offset,
        )
        .map_err(std::io::Error::from)?
    };
    Ok(ptr.cast())
}

/// Unmaps a previously mapped VFIO BAR region.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` was obtained from a previous call to [`vfio_bar_map`]
/// - `size` matches the size used in the original mapping
/// - No references to the mapped memory exist after this call
#[cfg(target_os = "linux")]
pub unsafe fn vfio_bar_unmap(ptr: *mut u8, size: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr and size are from a previous vfio_bar_map.
    unsafe { rustix::mm::munmap(ptr.cast(), size).map_err(std::io::Error::from) }
}

// ─── Raw Memory Locking ──────────────────────────────────────────────────────

/// Lock a raw memory region into physical RAM (prevents swap).
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` is valid for `len` bytes
/// - The memory region was properly allocated
/// - `unlock_memory` is called before freeing the allocation
#[cfg(target_os = "linux")]
pub unsafe fn lock_memory(ptr: *mut u8, len: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr is valid for len bytes.
    unsafe { rustix::mm::mlock(ptr.cast(), len).map_err(std::io::Error::from) }
}

/// Unlock a previously locked memory region, allowing it to be swapped.
///
/// # Safety
///
/// The caller must ensure:
/// - `ptr` and `len` match a previous call to [`lock_memory`]
#[cfg(target_os = "linux")]
pub unsafe fn unlock_memory(ptr: *mut u8, len: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr/len match a prior lock_memory call.
    unsafe { rustix::mm::munlock(ptr.cast(), len).map_err(std::io::Error::from) }
}

// ─── Ioctl Infrastructure Re-exports ─────────────────────────────────────────

/// Re-export rustix ioctl infrastructure for consumer crates.
///
/// Consumer crates implement these traits on their device-specific adapter
/// types without importing rustix directly.
#[cfg(target_os = "linux")]
pub mod ioctl_infra {
    pub use rustix::io::Errno;
    pub use rustix::io::Result as IoResult;
    pub use rustix::ioctl::{Getter, Ioctl, IoctlOutput, Opcode, Setter, Updater, ioctl, opcode};
}

// ─── SCM_RIGHTS (FD Passing over Unix Sockets) ──────────────────────────────

/// Receive data and file descriptors via SCM_RIGHTS from a Unix socket.
///
/// Returns `(bytes_read, received_fds)`. Up to `max_fds` file descriptors
/// will be extracted from the ancillary control message.
#[cfg(target_os = "linux")]
pub fn recv_with_fds(
    sock: impl std::os::fd::AsFd,
    buf: &mut [u8],
    max_fds: usize,
) -> std::io::Result<(usize, Vec<std::os::fd::OwnedFd>)> {
    use std::mem::MaybeUninit;
    let mut iov = [rustix::io::IoSliceMut::new(buf)];

    let space_size = max_fds * (std::mem::size_of::<std::os::fd::RawFd>() + 16) + 32;
    let mut recv_space: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); space_size];
    let mut control = rustix::net::RecvAncillaryBuffer::new(&mut recv_space);

    let msg = rustix::net::recvmsg(
        sock,
        &mut iov,
        &mut control,
        rustix::net::RecvFlags::empty(),
    )
    .map_err(std::io::Error::from)?;

    let mut fds = Vec::new();
    for ancillary in control.drain() {
        if let rustix::net::RecvAncillaryMessage::ScmRights(iter) = ancillary {
            fds.extend(iter);
        }
    }

    Ok((msg.bytes, fds))
}

// ─── Extended Device I/O ─────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
impl LinuxDeviceIo {
    /// Positional read from a file descriptor (does not change file offset).
    #[inline]
    pub fn pread(
        fd: std::os::fd::BorrowedFd<'_>,
        buf: &mut [u8],
        offset: u64,
    ) -> std::io::Result<usize> {
        rustix::io::pread(fd, buf, offset).map_err(std::io::Error::from)
    }

    /// Positional write to a file descriptor (does not change file offset).
    #[inline]
    pub fn pwrite(
        fd: std::os::fd::BorrowedFd<'_>,
        data: &[u8],
        offset: u64,
    ) -> std::io::Result<usize> {
        rustix::io::pwrite(fd, data, offset).map_err(std::io::Error::from)
    }
}

// ─── General mmap/munmap ─────────────────────────────────────────────────────

/// Map a device file region into memory.
///
/// # Safety
///
/// The caller must ensure `fd` is valid and `size`/`offset` describe a valid region.
#[cfg(target_os = "linux")]
pub unsafe fn mmap_device(
    fd: std::os::fd::BorrowedFd<'_>,
    size: usize,
    offset: u64,
    writable: bool,
) -> std::io::Result<*mut u8> {
    let prot = if writable {
        rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE
    } else {
        rustix::mm::ProtFlags::READ
    };
    // SAFETY: caller guarantees fd, size, offset are valid.
    let ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            size,
            prot,
            rustix::mm::MapFlags::SHARED,
            fd,
            offset,
        )
        .map_err(std::io::Error::from)?
    };
    Ok(ptr.cast())
}

/// Unmap a previously mapped device memory region.
///
/// # Safety
///
/// The caller must ensure `ptr` and `size` match a previous `mmap_device` call.
#[cfg(target_os = "linux")]
pub unsafe fn munmap_device(ptr: *mut u8, size: usize) -> std::io::Result<()> {
    // SAFETY: caller guarantees ptr/size from prior mmap.
    unsafe { rustix::mm::munmap(ptr.cast(), size).map_err(std::io::Error::from) }
}

// ─── Device Node Creation ────────────────────────────────────────────────────

/// Create a character device node at `path`.
///
/// Wraps `mknodat(CWD, ...)` with `CharacterDevice` type.
#[cfg(target_os = "linux")]
pub fn mknod_char(path: &Path, mode: u32, major: u32, minor: u32) -> std::io::Result<()> {
    rustix::fs::mknodat(
        rustix::fs::CWD,
        path,
        rustix::fs::FileType::CharacterDevice,
        rustix::fs::Mode::from_raw_mode(mode as rustix::fs::RawMode),
        rustix::fs::makedev(major, minor),
    )
    .map_err(std::io::Error::from)
}

/// Open a file with specified flags (wraps `rustix::fs::open`).
#[cfg(target_os = "linux")]
pub fn open_path(path: &Path, rdwr: bool, sync: bool) -> std::io::Result<std::os::fd::OwnedFd> {
    let mut flags = rustix::fs::OFlags::CLOEXEC;
    if rdwr {
        flags |= rustix::fs::OFlags::RDWR;
    } else {
        flags |= rustix::fs::OFlags::RDONLY;
    }
    if sync {
        flags |= rustix::fs::OFlags::SYNC;
    }
    rustix::fs::open(path, flags, rustix::fs::Mode::empty()).map_err(std::io::Error::from)
}

// ─── Kernel Module Loading ───────────────────────────────────────────────────

/// Load a kernel module from an open `.ko` file via `finit_module(2)`.
#[cfg(target_os = "linux")]
pub fn finit_module(
    ko_file: &impl std::os::fd::AsFd,
    params: &std::ffi::CStr,
    flags: i32,
) -> std::io::Result<()> {
    rustix::system::finit_module(ko_file, params, flags).map_err(std::io::Error::from)
}

/// Unload a kernel module by name via `delete_module(2)`.
#[cfg(target_os = "linux")]
pub fn delete_module(name: &std::ffi::CStr, flags: i32) -> std::io::Result<()> {
    rustix::system::delete_module(name, flags).map_err(std::io::Error::from)
}

// ─── Clock ───────────────────────────────────────────────────────────────────

/// Get monotonic clock time in nanoseconds.
#[cfg(target_os = "linux")]
pub fn clock_monotonic_ns() -> u64 {
    let ts = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    ts.tv_sec as u64 * 1_000_000_000 + ts.tv_nsec as u64
}

// ─── Filesystem Stats ────────────────────────────────────────────────────────

/// Filesystem statistics (capacity/free space).
#[derive(Debug, Clone)]
pub struct FsStats {
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Available bytes to unprivileged users.
    pub available_bytes: u64,
    /// Total number of inodes.
    pub total_inodes: u64,
    /// Available inodes.
    pub available_inodes: u64,
}

/// Query filesystem statistics for a path (wraps `statvfs`).
#[cfg(target_os = "linux")]
pub fn fs_stats(path: &Path) -> std::io::Result<FsStats> {
    let st = rustix::fs::statvfs(path).map_err(std::io::Error::from)?;
    Ok(FsStats {
        total_bytes: st.f_blocks * st.f_frsize,
        available_bytes: st.f_bavail * st.f_frsize,
        total_inodes: st.f_files,
        available_inodes: st.f_favail,
    })
}

// ─── Unix Socket sendmsg with FD passing ─────────────────────────────────────

/// Create a Unix DGRAM socket.
#[cfg(target_os = "linux")]
pub fn unix_dgram_socket() -> std::io::Result<std::os::fd::OwnedFd> {
    rustix::net::socket(
        rustix::net::AddressFamily::UNIX,
        rustix::net::SocketType::DGRAM,
        None,
    )
    .map_err(std::io::Error::from)
}

/// Unix socket address (filesystem or abstract).
pub enum UnixAddr {
    /// Filesystem-based socket path.
    Path(std::path::PathBuf),
    /// Linux abstract namespace socket name.
    Abstract(Vec<u8>),
}

/// Send a message with optional file descriptors (SCM_RIGHTS) over a Unix socket.
#[cfg(target_os = "linux")]
pub fn sendmsg_with_fds(
    sock: impl std::os::fd::AsFd,
    addr: &UnixAddr,
    data: &[u8],
    fds: &[std::os::fd::BorrowedFd<'_>],
) -> std::io::Result<()> {
    use std::mem::MaybeUninit;

    let unix_addr = match addr {
        UnixAddr::Path(p) => rustix::net::SocketAddrUnix::new(p).map_err(std::io::Error::from)?,
        UnixAddr::Abstract(name) => {
            rustix::net::SocketAddrUnix::new_abstract_name(name).map_err(std::io::Error::from)?
        }
    };

    let iov = [rustix::io::IoSlice::new(data)];

    if fds.is_empty() {
        rustix::net::sendmsg_addr(
            sock,
            &unix_addr,
            &iov,
            &mut rustix::net::SendAncillaryBuffer::default(),
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::from)?;
    } else {
        let space_size = fds.len() * (std::mem::size_of::<std::os::fd::RawFd>() + 16) + 32;
        let mut space: Vec<MaybeUninit<u8>> = vec![MaybeUninit::uninit(); space_size];
        let mut cmsg_buf = rustix::net::SendAncillaryBuffer::new(&mut space);
        cmsg_buf.push(rustix::net::SendAncillaryMessage::ScmRights(fds));
        rustix::net::sendmsg_addr(
            sock,
            &unix_addr,
            &iov,
            &mut cmsg_buf,
            rustix::net::SendFlags::empty(),
        )
        .map_err(std::io::Error::from)?;
    }
    Ok(())
}

// ─── File Seek ───────────────────────────────────────────────────────────────

/// Seek to the end of a file descriptor.
#[cfg(target_os = "linux")]
pub fn seek_end(fd: std::os::fd::BorrowedFd<'_>) -> std::io::Result<u64> {
    rustix::fs::seek(fd, rustix::fs::SeekFrom::End(0)).map_err(std::io::Error::from)
}

