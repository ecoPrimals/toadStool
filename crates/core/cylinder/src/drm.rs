// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure Rust DRM ioctl interface — uses `rustix` for mmap/munmap and ioctl.
//!
//! All ioctl numbers and structures are defined here from the Linux
//! kernel headers (GPL-2.0-only) via clean-room constant extraction.
//!
//! Memory mapping uses `rustix::mm` (safe wrappers).
//! ioctl uses `rustix::ioctl` — zero libc, zero inline asm.

use crate::error::{DriverError, DriverResult};
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr::NonNull;

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

        if let Some(v) = std::env::var(socket_env::TOADSTOOL_DRI_RENDER_PREFIX).ok().filter(|s| !s.is_empty()) {
            return v;
        }
        #[expect(deprecated, reason = "legacy env-var fallback for migration")]
        if let Some(v) = std::env::var(socket_env::CORALREEF_DRI_RENDER_PREFIX).ok().filter(|s| !s.is_empty()) {
            tracing::warn!("deprecated env var CORALREEF_DRI_RENDER_PREFIX — migrate to TOADSTOOL_DRI_RENDER_PREFIX");
            return v;
        }
        "/dev/dri/renderD".into()
    })
}

const DRI_RENDER_FIRST: u32 = 128;
const DRI_RENDER_LAST: u32 = 191;

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
#[allow(dead_code, reason = "used by AMD/NV backends absorbed in subsequent Phase C batches")]
pub(crate) struct MappedRegion {
    ptr: NonNull<u8>,
    len: usize,
}

#[allow(dead_code, reason = "used by AMD/NV backends absorbed in subsequent Phase C batches")]
impl MappedRegion {
    /// Map a file descriptor region into memory using `rustix::mm::mmap`.
    pub(crate) fn new(
        len: usize,
        prot: rustix::mm::ProtFlags,
        flags: rustix::mm::MapFlags,
        fd: RawFd,
        offset: u64,
    ) -> DriverResult<Self> {
        use std::os::unix::io::BorrowedFd;

        if len == 0 {
            return Err(DriverError::MmapFailed("mmap length must be > 0".into()));
        }
        // SAFETY:
        // 1. Validity:   fd is a valid open DRM device, offset is kernel-provided
        // 2. Alignment:  mmap returns page-aligned memory
        // 3. Lifetime:   the mapping is owned by this struct, unmapped in Drop
        // 4. Exclusivity: single owner; &mut access gated by &mut self
        let ptr = unsafe {
            rustix::mm::mmap(
                std::ptr::null_mut(),
                len,
                prot,
                flags,
                BorrowedFd::borrow_raw(fd),
                offset,
            )
        }
        .map_err(|e| DriverError::MmapFailed(format!("mmap failed: {e}").into()))?;
        let ptr = NonNull::new(ptr.cast::<u8>())
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
        // SAFETY: ptr was returned by a successful rustix::mm::mmap in new()
        unsafe {
            let _ = rustix::mm::munmap(self.ptr.as_ptr().cast::<std::ffi::c_void>(), self.len);
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
    let opcode = request as rustix::ioctl::Opcode;
    let ioctl_cmd = DrmIoctlCmd::new(opcode, arg);
    // SAFETY:
    // 1. fd is a valid open DRM device file descriptor (caller)
    // 2. ioctl_cmd wraps a properly aligned &mut T
    // 3. synchronous ioctl; all data outlives the call
    unsafe { rustix::ioctl::ioctl(BorrowedFd::borrow_raw(fd), ioctl_cmd) }.map_err(|e| {
        DriverError::IoctlFailed {
            name,
            errno: e.raw_os_error(),
        }
    })
}

struct DrmIoctlCmd<'a, T> {
    opcode: rustix::ioctl::Opcode,
    arg: &'a mut T,
}

impl<'a, T> DrmIoctlCmd<'a, T> {
    #[inline]
    fn new(opcode: rustix::ioctl::Opcode, arg: &'a mut T) -> Self {
        Self { opcode, arg }
    }
}

// SAFETY: opcode/arg are paired by drm_ioctl_named call sites
unsafe impl<T> rustix::ioctl::Ioctl for DrmIoctlCmd<'_, T> {
    type Output = ();
    const IS_MUTATING: bool = true;

    fn opcode(&self) -> rustix::ioctl::Opcode {
        self.opcode
    }

    fn as_ptr(&mut self) -> *mut std::ffi::c_void {
        std::ptr::from_mut(self.arg).cast()
    }

    /// # Safety
    /// Caller guarantees `out` points to valid ioctl return data.
    unsafe fn output_from_ptr(
        _output: rustix::ioctl::IoctlOutput,
        _ptr: *mut std::ffi::c_void,
    ) -> rustix::io::Result<Self::Output> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    fn temp_mmap_file(size: usize) -> (File, std::path::PathBuf) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cylinder_drm_mmap_test_{unique}"));
        let mut f = File::create(&path).expect("create temp file");
        f.write_all(&vec![0u8; size]).expect("write temp file");
        f.sync_all().expect("sync temp file");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("reopen temp file");
        (file, path)
    }

    #[test]
    fn ioctl_numbers_are_consistent() {
        assert_eq!(DRM_IOCTL_VERSION & 0xFF, 0);
    }

    #[test]
    fn drm_iowr_pub_constructs_valid_ioctl() {
        assert_eq!(drm_iowr_pub(0x00, 32), DRM_IOCTL_VERSION);
    }

    #[test]
    fn drm_iow_pub_constructs_valid_ioctl() {
        assert_eq!(drm_iow_pub(0x09, 8), DRM_IOCTL_GEM_CLOSE);
    }

    #[test]
    fn drm_gem_close_struct_size() {
        assert_eq!(std::mem::size_of::<DrmGemClose>(), 8);
    }

    #[test]
    fn mapped_region_zero_length_fails() {
        let file = File::open("/dev/zero").unwrap();
        let fd = file.as_raw_fd();
        let result = MappedRegion::new(
            0,
            rustix::mm::ProtFlags::READ,
            rustix::mm::MapFlags::SHARED,
            fd,
            0,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mmap length must be > 0"));
    }

    #[test]
    fn mapped_region_slice_at_out_of_bounds() {
        let (file, path) = temp_mmap_file(4096);
        let fd = file.as_raw_fd();
        let region = MappedRegion::new(
            4096,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            fd,
            0,
        )
        .unwrap();
        let result = region.slice_at(0, 4097);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mapped_region_slice_at_overflow() {
        let (file, path) = temp_mmap_file(4096);
        let fd = file.as_raw_fd();
        let region = MappedRegion::new(
            4096,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            fd,
            0,
        )
        .unwrap();
        let result = region.slice_at(usize::MAX, 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("overflow"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn mapped_region_slice_at_mut_out_of_bounds() {
        let (file, path) = temp_mmap_file(4096);
        let fd = file.as_raw_fd();
        let mut region = MappedRegion::new(
            4096,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            fd,
            0,
        )
        .unwrap();
        let result = region.slice_at_mut(4090, 100);
        assert!(result.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn drm_version_struct_layout() {
        assert_eq!(std::mem::size_of::<DrmVersion>(), 64);
    }

    #[test]
    fn drm_version_parsing_trim_nul() {
        let mut name_buf = [0u8; 64];
        name_buf[..6].copy_from_slice(b"amdgpu");
        let ver = DrmVersion {
            name_len: 6,
            ..Default::default()
        };
        let len = usize::try_from(ver.name_len)
            .unwrap_or(name_buf.len())
            .min(name_buf.len());
        let name = String::from_utf8_lossy(&name_buf[..len])
            .trim_end_matches('\0')
            .to_string();
        assert_eq!(name, "amdgpu");
    }

    #[test]
    fn drm_render_node_index_range() {
        assert_eq!(DRI_RENDER_FIRST, 128);
        assert_eq!(DRI_RENDER_LAST, 191);
    }

    #[test]
    fn drm_device_not_found_on_invalid_path() {
        assert!(DrmDevice::open("/dev/dri/renderD999").is_err());
    }

    #[test]
    fn enumerate_render_nodes_returns_vec() {
        let nodes = enumerate_render_nodes();
        for info in &nodes {
            assert!(!info.path.is_empty());
            assert!(!info.driver.is_empty());
        }
    }

    #[test]
    fn drm_device_info_has_driver_and_path() {
        let info = DrmDeviceInfo {
            path: "/dev/dri/renderD128".to_string(),
            driver: "amdgpu".to_string(),
            version_major: 3,
            version_minor: 57,
        };
        let debug = format!("{info:?}");
        assert!(debug.contains("amdgpu"));
        assert!(debug.contains("renderD128"));
    }

    #[test]
    fn open_by_driver_nonexistent_fails() {
        assert!(DrmDevice::open_by_driver("nonexistent_drm_driver_xyz").is_err());
    }

    #[test]
    fn drm_gem_close_default() {
        let close = DrmGemClose::default();
        assert_eq!(close.handle, 0);
        assert_eq!(close.pad, 0);
    }

    #[test]
    fn mapped_region_slice_at_valid_range() {
        let (file, path) = temp_mmap_file(4096);
        let fd = file.as_raw_fd();
        let region = MappedRegion::new(
            4096,
            rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
            rustix::mm::MapFlags::SHARED,
            fd,
            0,
        )
        .unwrap();
        let slice = region.slice_at(0, 256).unwrap();
        assert_eq!(slice.len(), 256);
        let _ = std::fs::remove_file(path);
    }
}
