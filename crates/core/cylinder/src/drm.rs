// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure Rust DRM ioctl interface — uses `toadstool_hw_safe` for mmap/munmap and ioctl.
//!
//! All ioctl numbers and structures are defined here from the Linux
//! kernel headers (GPL-2.0-only) via clean-room constant extraction.
//!
//! Memory mapping uses `toadstool_hw_safe::mmap_device` / `munmap_device`.
//! ioctl uses `toadstool_hw_safe::ioctl_infra` — zero libc, zero inline asm.

use crate::error::{DriverError, DriverResult};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr::NonNull;
use toadstool_hw_safe::ioctl_infra::{IoResult, Ioctl, IoctlOutput, Opcode, ioctl as raw_ioctl};
use toadstool_hw_safe::{mmap_device, munmap_device};

/// Linux ioctl direction flags (shared with UVM ioctls).
pub(crate) const _IOC_NONE: u32 = 0;
pub(crate) const IOC_WRITE: u32 = 1;
pub(crate) const IOC_READ: u32 = 2;

pub(crate) const IOC_NRBITS: u32 = 8;
pub(crate) const IOC_TYPEBITS: u32 = 8;
pub(crate) const IOC_SIZEBITS: u32 = 14;

pub(crate) const IOC_NRSHIFT: u32 = 0;
pub(crate) const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
pub(crate) const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
pub(crate) const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;

const DRM_IOCTL_BASE: u32 = b'd' as u32;

/// DRM render node path prefix. Override with `TOADSTOOL_DRI_RENDER_PREFIX`.
fn dri_render_prefix() -> &'static str {
    static PREFIX: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PREFIX.get_or_init(|| {
        use toadstool_common::interned_strings::socket_env;

        std::env::var(socket_env::TOADSTOOL_DRI_RENDER_PREFIX)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/dev/dri/renderD".into())
    })
}

pub(crate) const DRI_RENDER_FIRST: u32 = 128;
pub(crate) const DRI_RENDER_LAST: u32 = 191;

const fn drm_ioctl(dir: u32, nr: u32, size: u32) -> u64 {
    ((dir << IOC_DIRSHIFT)
        | (DRM_IOCTL_BASE << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)) as u64
}

const fn _drm_io(nr: u32) -> u64 {
    drm_ioctl(_IOC_NONE, nr, 0)
}

const fn drm_iowr(nr: u32, size: u32) -> u64 {
    drm_ioctl(IOC_READ | IOC_WRITE, nr, size)
}

const fn drm_iow(nr: u32, size: u32) -> u64 {
    drm_ioctl(IOC_WRITE, nr, size)
}

const fn _drm_ior(nr: u32, size: u32) -> u64 {
    drm_ioctl(IOC_READ, nr, size)
}

/// `DRM_IOCTL_VERSION`
pub const DRM_IOCTL_VERSION: u64 = drm_iowr(0x00, 32);

/// `DRM_IOCTL_GEM_CLOSE` (generic, not vendor-specific)
pub const DRM_IOCTL_GEM_CLOSE: u64 = drm_iow(0x09, 8);

/// Argument for `DRM_IOCTL_GEM_CLOSE`.
#[repr(C)]
#[derive(Default)]
pub struct DrmGemClose {
    /// GEM buffer handle to close.
    pub handle: u32,
    /// Padding for alignment (must be zero).
    pub pad: u32,
}

/// Public helper for submodules to construct IOWR ioctl numbers.
#[must_use]
pub const fn drm_iowr_pub(nr: u32, size: u32) -> u64 {
    drm_iowr(nr, size)
}

/// Public helper for submodules to construct IOW ioctl numbers.
#[must_use]
pub const fn drm_iow_pub(nr: u32, size: u32) -> u64 {
    drm_iow(nr, size)
}

/// DRM version info returned by the kernel.
#[repr(C)]
#[derive(Debug, Default)]
pub struct DrmVersion {
    /// Major version number.
    pub version_major: i32,
    /// Minor version number.
    pub version_minor: i32,
    /// Patch level.
    pub version_patchlevel: i32,
    /// Length of the driver name string.
    pub name_len: u64,
    /// Pointer to driver name buffer (userspace-provided).
    pub name: u64,
    /// Length of the date string.
    pub date_len: u64,
    /// Pointer to date buffer.
    pub date: u64,
    /// Length of the description string.
    pub desc_len: u64,
    /// Pointer to description buffer.
    pub desc: u64,
}

/// RAII wrapper around a memory-mapped region. Unmaps on drop.
#[derive(Debug)]
pub(crate) struct MappedRegion {
    ptr: NonNull<u8>,
    len: usize,
}

impl MappedRegion {
    /// Map a file descriptor region into memory using `mmap_device`.
    pub(crate) fn new(len: usize, writable: bool, fd: RawFd, offset: u64) -> DriverResult<Self> {
        use std::os::unix::io::BorrowedFd;

        if len == 0 {
            return Err(DriverError::MmapFailed("mmap length must be > 0".into()));
        }
        // SAFETY:
        // 1. Validity:   fd is a valid open DRM device, offset is kernel-provided
        // 2. Alignment:  mmap returns page-aligned memory
        // 3. Lifetime:   the mapping is owned by this struct, unmapped in Drop
        // 4. Exclusivity: single owner; &mut access gated by &mut self
        let ptr = unsafe { mmap_device(BorrowedFd::borrow_raw(fd), len, offset, writable) }
            .map_err(|e| DriverError::MmapFailed(format!("mmap failed: {e}").into()))?;
        let ptr = NonNull::new(ptr)
            .ok_or_else(|| DriverError::MmapFailed("mmap returned null".into()))?;
        Ok(Self { ptr, len })
    }

    /// View the mapped region as a byte slice.
    #[must_use]
    pub(crate) const fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr is non-null from successful mmap, u8 alignment is 1
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// View the mapped region as a mutable byte slice.
    #[must_use]
    pub(crate) const fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: ptr is non-null from successful mmap, &mut self guarantees exclusivity
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Bounds-checked subslice.
    pub(crate) fn slice_at(&self, offset: usize, len: usize) -> DriverResult<&[u8]> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| DriverError::MmapFailed("slice range overflow".into()))?;
        if end > self.len {
            return Err(DriverError::MmapFailed(
                format!(
                    "slice out of bounds: offset={offset}, len={len}, region_len={}",
                    self.len
                )
                .into(),
            ));
        }
        Ok(&self.as_slice()[offset..end])
    }

    /// Bounds-checked mutable subslice.
    pub(crate) fn slice_at_mut(&mut self, offset: usize, len: usize) -> DriverResult<&mut [u8]> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| DriverError::MmapFailed("slice range overflow".into()))?;
        if end > self.len {
            return Err(DriverError::MmapFailed(
                format!(
                    "slice out of bounds: offset={offset}, len={len}, region_len={}",
                    self.len
                )
                .into(),
            ));
        }
        Ok(&mut self.as_mut_slice()[offset..end])
    }
}

impl Drop for MappedRegion {
    fn drop(&mut self) {
        // SAFETY: ptr was returned by a successful mmap_device in new()
        unsafe {
            let _ = munmap_device(self.ptr.as_ptr(), self.len);
        }
    }
}

/// Metadata about a discovered DRM render node.
#[derive(Debug, Clone)]
pub struct DrmDeviceInfo {
    /// Render node path (e.g. `/dev/dri/renderD128`).
    pub path: String,
    /// Kernel driver name (e.g. `"amdgpu"`, `"nouveau"`, `"nvidia"`).
    pub driver: String,
    /// DRM driver major version.
    pub version_major: i32,
    /// DRM driver minor version.
    pub version_minor: i32,
}

/// A DRM render node file descriptor.
pub struct DrmDevice {
    file: File,
    /// Render node path (e.g. `/dev/dri/renderD128`).
    pub path: String,
}

impl DrmDevice {
    /// Open a DRM render node.
    pub fn open(path: &str) -> DriverResult<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        Ok(Self {
            file,
            path: path.to_string(),
        })
    }

    /// Open the first available render node.
    pub fn open_default() -> DriverResult<Self> {
        let prefix = dri_render_prefix();
        for idx in DRI_RENDER_FIRST..=DRI_RENDER_LAST {
            let path = format!("{prefix}{idx}");
            if let Ok(dev) = Self::open(&path) {
                return Ok(dev);
            }
        }
        Err(DriverError::DeviceNotFound(
            "no DRM render node found".into(),
        ))
    }

    /// Open the first render node matching a specific driver name.
    pub fn open_by_driver(driver_name: &str) -> DriverResult<Self> {
        for info in enumerate_render_nodes() {
            if info.driver == driver_name {
                return Self::open(&info.path);
            }
        }
        Err(DriverError::DeviceNotFound(
            format!("no DRM render node with driver '{driver_name}'").into(),
        ))
    }

    /// Raw file descriptor for ioctl calls.
    #[must_use]
    pub fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Query the DRM driver name.
    pub fn driver_name(&self) -> DriverResult<String> {
        let (_ver, name) = drm_version(self.fd())?;
        Ok(name)
    }

    /// Query full device info (driver name + version).
    pub fn device_info(&self) -> DriverResult<DrmDeviceInfo> {
        let (ver, name) = drm_version(self.fd())?;
        Ok(DrmDeviceInfo {
            path: self.path.clone(),
            driver: name,
            version_major: ver.version_major,
            version_minor: ver.version_minor,
        })
    }
}

/// Enumerate all available DRM render nodes with their driver info.
///
/// Scans `/dev/dri/renderD128` through `renderD191` and returns metadata
/// for every node that can be opened and queried.
#[must_use]
pub fn enumerate_render_nodes() -> Vec<DrmDeviceInfo> {
    let prefix = dri_render_prefix();
    let mut devices = Vec::new();
    for idx in DRI_RENDER_FIRST..=DRI_RENDER_LAST {
        let path = format!("{prefix}{idx}");
        if let Ok(dev) = DrmDevice::open(&path)
            && let Ok(info) = dev.device_info()
        {
            devices.push(info);
        }
    }
    devices
}

/// Close a GEM buffer object. Safe wrapper around `DRM_IOCTL_GEM_CLOSE`.
pub fn gem_close(fd: RawFd, handle: u32) -> DriverResult<()> {
    let mut args = DrmGemClose { handle, pad: 0 };
    drm_ioctl_named(fd, DRM_IOCTL_GEM_CLOSE, &mut args, "gem_close")
}

/// Query the DRM driver version.
pub(crate) fn drm_version(fd: RawFd) -> DriverResult<(DrmVersion, String)> {
    let mut name_buf = [0u8; 64];
    let mut ver = DrmVersion {
        name_len: name_buf.len() as u64,
        name: name_buf.as_mut_ptr() as u64,
        ..Default::default()
    };
    drm_ioctl_named(fd, DRM_IOCTL_VERSION, &mut ver, "drm_version")?;
    let len = usize::try_from(ver.name_len)
        .unwrap_or(name_buf.len())
        .min(name_buf.len());
    let name = String::from_utf8_lossy(&name_buf[..len])
        .trim_end_matches('\0')
        .to_string();
    Ok((ver, name))
}

/// Perform a named DRM ioctl on a `#[repr(C)]` structure.
///
/// # Correctness
///
/// `request` must be the ioctl opcode for `T`, and `T` must be `#[repr(C)]`
/// matching the kernel ABI. `fd` must be an open DRM/NV device descriptor
/// appropriate for that ioctl.
pub(crate) fn drm_ioctl_named<T>(
    fd: RawFd,
    request: u64,
    arg: &mut T,
    name: &'static str,
) -> DriverResult<()> {
    use std::os::unix::io::BorrowedFd;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Linux ioctl request codes are u32; our u64 constants fit"
    )]
    let opcode = request as Opcode;
    let ioctl_cmd = DrmIoctlCmd::new(opcode, arg);
    // SAFETY:
    // 1. fd is a valid open DRM device file descriptor (caller)
    // 2. ioctl_cmd wraps a properly aligned &mut T
    // 3. synchronous ioctl; all data outlives the call
    unsafe { raw_ioctl(BorrowedFd::borrow_raw(fd), ioctl_cmd) }.map_err(|e| {
        DriverError::IoctlFailed {
            name,
            errno: e.raw_os_error(),
        }
    })
}

struct DrmIoctlCmd<'a, T> {
    opcode: Opcode,
    arg: &'a mut T,
}

impl<'a, T> DrmIoctlCmd<'a, T> {
    #[inline]
    fn new(opcode: Opcode, arg: &'a mut T) -> Self {
        Self { opcode, arg }
    }
}

// SAFETY: opcode/arg are paired by drm_ioctl_named call sites
unsafe impl<T> Ioctl for DrmIoctlCmd<'_, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> Opcode {
        self.opcode
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        std::ptr::from_mut(self.arg).cast()
    }

    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        _output: IoctlOutput,
        _ptr: *mut std::ffi::c_void,
    ) -> IoResult<Self::Output> {
        // SAFETY: No-op — body is intentionally empty (trait method stub for ioctl infrastructure).
        Ok(())
    }
}
