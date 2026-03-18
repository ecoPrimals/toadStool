// SPDX-License-Identifier: AGPL-3.0-or-later
//! V4L2 capture device — pure Rust via `rustix` ioctls.
//!
//! Wraps `/dev/video*` devices for reading video frames from HDMI capture cards.
//! Uses `mmap` streaming I/O for zero-copy frame delivery.

#![allow(unsafe_code)] // V4L2 ioctls/mmap require unsafe; each block has // SAFETY: comment.

use crate::{DisplayError, Result};
use rustix::fd::OwnedFd;
use rustix::fs;
use std::os::unix::io::{AsFd, BorrowedFd};
use std::path::{Path, PathBuf};

// ---- V4L2 ioctl numbers (Linux UAPI) ----

const VIDIOC_MAGIC: u8 = b'V';

/// V4L2 capability flags.
#[derive(Debug, Clone)]
pub struct V4l2Capability {
    /// Driver name (e.g. "uvcvideo").
    pub driver: String,
    /// Card / device name.
    pub card: String,
    /// Bus info.
    pub bus_info: String,
    /// Device capabilities bitfield.
    pub capabilities: u32,
}

/// Capture pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureFormat {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `FourCC` pixel format code.
    pub fourcc: u32,
    /// Bytes per line (stride).
    pub bytes_per_line: u32,
    /// Total image size in bytes.
    pub image_size: u32,
}

/// V4L2 capability flags we care about.
const V4L2_CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
const V4L2_CAP_STREAMING: u32 = 0x0400_0000;

/// V4L2 buffer type for single-plane video capture.
const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
/// V4L2 memory type for mmap.
const V4L2_MEMORY_MMAP: u32 = 1;

// ---- Raw V4L2 structures (Linux UAPI layout) ----
//
// All V4L2 structs below are #[repr(C)] with only primitive types (u8, u32, i32, i64, arrays).
// Zero-initialization is valid for all fields: 0 is valid for integers, null for pointers,
// and padding bytes are unobserved. Default is used for zero-initialization.

#[repr(C)]
struct v4l2_capability {
    driver: [u8; 16],
    card: [u8; 32],
    bus_info: [u8; 32],
    version: u32,
    capabilities: u32,
    device_caps: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Default)]
struct v4l2_pix_format {
    width: u32,
    height: u32,
    pixelformat: u32,
    field: u32,
    bytesperline: u32,
    sizeimage: u32,
    colorspace: u32,
    priv_: u32,
    flags: u32,
    // union padding
    hsv_enc_or_ycbcr_enc: u32,
    quantization: u32,
    xfer_func: u32,
}

#[repr(C)]
struct v4l2_format {
    type_: u32,
    fmt: v4l2_pix_format,
    // padding to cover the union
    _pad: [u8; 128],
}

impl Default for v4l2_format {
    fn default() -> Self {
        Self {
            type_: 0,
            fmt: v4l2_pix_format::default(),
            _pad: [0; 128],
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct v4l2_requestbuffers {
    count: u32,
    type_: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
    // padding
    _pad: [u32; 4],
}

#[repr(C)]
#[derive(Default)]
struct v4l2_buffer {
    index: u32,
    type_: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    // timeval
    tv_sec: i64,
    tv_usec: i64,
    // v4l2_timecode
    timecode: [u32; 4],
    sequence: u32,
    memory: u32,
    // union m — for mmap, offset is at this position
    m_offset: u32,
    length: u32,
    reserved2: u32,
    // union — request_fd or reserved
    request_fd_or_reserved: i32,
}

/// A V4L2 capture device.
pub struct CaptureDevice {
    path: PathBuf,
    fd: OwnedFd,
    format: Option<CaptureFormat>,
    buffers: Vec<MmapBuffer>,
    streaming: bool,
}

struct MmapBuffer {
    ptr: *mut u8,
    len: usize,
}

impl MmapBuffer {
    /// Return a slice view of the mmap'd memory.
    ///
    /// SAFETY: ptr and len come from a successful mmap; we own this mapping exclusively.
    /// The slice is valid for the lifetime of self. Callers must not hold the slice across
    /// operations that could invalidate the buffer (e.g. `stop_streaming`).
    const fn as_slice(&self) -> &[u8] {
        if self.ptr.is_null() || self.len == 0 {
            return &[];
        }
        // SAFETY: ptr and len from mmap in request_buffers; valid for buffer lifetime.
        // No safe way to create slice from mmap'd memory without unsafe.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

// SAFETY: MmapBuffer is Send/Sync because: ptr/len are only accessed via &mut CaptureDevice
// (exclusive access); no shared mutable access across threads. The owned mmap region has no
// thread-safety issues when moved. Safe Rust cannot express "owned mmap region" without unsafe.
unsafe impl Send for MmapBuffer {}
unsafe impl Sync for MmapBuffer {}

impl Drop for MmapBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: munmap is unsafe: wrong ptr/len can corrupt process or cause UB. Invariants:
            // ptr and len come from a successful mmap; we own this mapping exclusively. No safe
            // munmap in std; rustix::mm::munmap is the correct low-level API.
            unsafe {
                rustix::mm::munmap(self.ptr.cast(), self.len).ok();
            }
        }
    }
}

impl AsFd for CaptureDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl CaptureDevice {
    /// Open a V4L2 capture device.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or the device cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(DisplayError::DeviceNotFound(path));
        }

        let fd = fs::open(
            &path,
            fs::OFlags::RDWR | fs::OFlags::NONBLOCK | fs::OFlags::CLOEXEC,
            fs::Mode::empty(),
        )
        .map_err(|e| {
            DisplayError::OpenFailed(std::io::Error::from_raw_os_error(e.raw_os_error()))
        })?;

        Ok(Self {
            path,
            fd,
            format: None,
            buffers: Vec::new(),
            streaming: false,
        })
    }

    /// Query device capabilities.
    ///
    /// # Errors
    ///
    /// Returns an error if the `VIDIOC_QUERYCAP` ioctl fails.
    pub fn query_capabilities(&self) -> Result<V4l2Capability> {
        // VIDIOC_QUERYCAP = _IOR('V', 0, struct v4l2_capability)
        // SAFETY: ioctl is an FFI call to the kernel; the kernel writes into a v4l2_capability
        // buffer we provide. Invariants: fd is a valid open V4L2 device; the Getter passes a
        // properly sized/aligned buffer. Safe Rust has no ioctl abstraction for V4L2.
        let cap = unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Getter::<
                    rustix::ioctl::ReadOpcode<VIDIOC_MAGIC, 0, v4l2_capability>,
                    v4l2_capability,
                >::new(),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_QUERYCAP: {e}")))?;

        let to_string = |bytes: &[u8]| {
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };

        Ok(V4l2Capability {
            driver: to_string(&cap.driver),
            card: to_string(&cap.card),
            bus_info: to_string(&cap.bus_info),
            capabilities: cap.capabilities,
        })
    }

    /// Whether this device supports video capture with streaming I/O.
    ///
    /// # Errors
    ///
    /// Returns an error if capability query fails.
    pub fn supports_capture_streaming(&self) -> Result<bool> {
        let caps = self.query_capabilities()?;
        Ok(caps.capabilities & V4L2_CAP_VIDEO_CAPTURE != 0
            && caps.capabilities & V4L2_CAP_STREAMING != 0)
    }

    /// Set the capture format. Returns the negotiated format (driver may adjust).
    ///
    /// # Errors
    ///
    /// Returns an error if the `VIDIOC_S_FMT` ioctl fails.
    pub fn set_format(&mut self, width: u32, height: u32, fourcc: u32) -> Result<CaptureFormat> {
        let mut fmt = v4l2_format {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            ..v4l2_format::default()
        };
        fmt.fmt.width = width;
        fmt.fmt.height = height;
        fmt.fmt.pixelformat = fourcc;

        // VIDIOC_S_FMT = _IOWR('V', 5, struct v4l2_format)
        // SAFETY: ioctl is an FFI call; kernel reads our fmt and may write back. Invariants: fd
        // is valid, fmt is properly initialized. No safe Rust API for V4L2 ioctls.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Updater::<
                    rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 5, v4l2_format>,
                    v4l2_format,
                >::new(&mut fmt),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_S_FMT: {e}")))?;

        let cf = CaptureFormat {
            width: fmt.fmt.width,
            height: fmt.fmt.height,
            fourcc: fmt.fmt.pixelformat,
            bytes_per_line: fmt.fmt.bytesperline,
            image_size: fmt.fmt.sizeimage,
        };
        self.format = Some(cf);
        Ok(cf)
    }

    /// Request mmap buffers and map them.
    ///
    /// # Errors
    ///
    /// Returns an error if `VIDIOC_REQBUFS` or buffer mapping fails.
    pub fn request_buffers(&mut self, count: u32) -> Result<u32> {
        let mut req = v4l2_requestbuffers {
            count,
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            ..v4l2_requestbuffers::default()
        };

        // VIDIOC_REQBUFS = _IOWR('V', 8, struct v4l2_requestbuffers)
        // SAFETY: ioctl FFI; kernel reads req and writes back. Invariants: fd valid, req initialized.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Updater::<
                    rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 8, v4l2_requestbuffers>,
                    v4l2_requestbuffers,
                >::new(&mut req),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_REQBUFS: {e}")))?;

        // mmap each buffer
        for i in 0..req.count {
            let mut buf = v4l2_buffer {
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                memory: V4L2_MEMORY_MMAP,
                index: i,
                ..v4l2_buffer::default()
            };

            // VIDIOC_QUERYBUF = _IOWR('V', 9, struct v4l2_buffer)
            // SAFETY: ioctl FFI; kernel fills buf with buffer info. Invariants: fd valid, buf init'd.
            unsafe {
                rustix::ioctl::ioctl(
                    &self.fd,
                    rustix::ioctl::Updater::<
                        rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 9, v4l2_buffer>,
                        v4l2_buffer,
                    >::new(&mut buf),
                )
            }
            .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_QUERYBUF: {e}")))?;

            // SAFETY: mmap returns raw pointer; wrong args can cause UB or security issues.
            // Invariants: fd is valid V4L2 device, buf.m_offset/length from kernel via QUERYBUF.
            // No safe mmap in std; rustix exposes the syscall.
            let ptr = unsafe {
                rustix::mm::mmap(
                    std::ptr::null_mut(),
                    buf.length as usize,
                    rustix::mm::ProtFlags::READ | rustix::mm::ProtFlags::WRITE,
                    rustix::mm::MapFlags::SHARED,
                    &self.fd,
                    u64::from(buf.m_offset),
                )
            }
            .map_err(|e| DisplayError::IoctlFailed(format!("mmap: {e}")))?;

            self.buffers.push(MmapBuffer {
                ptr: ptr.cast(),
                len: buf.length as usize,
            });
        }

        Ok(req.count)
    }

    /// Queue all buffers and start streaming.
    ///
    /// # Errors
    ///
    /// Returns an error if `VIDIOC_QBUF` or `VIDIOC_STREAMON` fails.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "buffer index from hardware fits u32"
    )]
    pub fn start_streaming(&mut self) -> Result<()> {
        for i in 0..self.buffers.len() {
            let mut buf = v4l2_buffer {
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                memory: V4L2_MEMORY_MMAP,
                index: i as u32,
                ..v4l2_buffer::default()
            };

            // VIDIOC_QBUF = _IOWR('V', 15, struct v4l2_buffer)
            // SAFETY: ioctl FFI; kernel reads buf to queue. Invariants: fd valid, buf initialized.
            unsafe {
                rustix::ioctl::ioctl(
                    &self.fd,
                    rustix::ioctl::Updater::<
                        rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 15, v4l2_buffer>,
                        v4l2_buffer,
                    >::new(&mut buf),
                )
            }
            .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_QBUF: {e}")))?;
        }

        // VIDIOC_STREAMON = _IOW('V', 18, int)
        // SAFETY: ioctl FFI; kernel starts streaming. Invariants: fd valid, buffer type is u32.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Setter::<
                    rustix::ioctl::WriteOpcode<VIDIOC_MAGIC, 18, u32>,
                    u32,
                >::new(V4L2_BUF_TYPE_VIDEO_CAPTURE),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_STREAMON: {e}")))?;

        self.streaming = true;
        Ok(())
    }

    /// Dequeue a buffer (blocking until a frame is ready), copy its contents
    /// into `out`, then re-queue the buffer. Returns bytes written.
    ///
    /// # Errors
    ///
    /// Returns an error if not streaming or if `VIDIOC_DQBUF`/`VIDIOC_QBUF` fails.
    pub fn read_frame(&mut self, out: &mut [u8]) -> Result<usize> {
        if !self.streaming {
            return Err(DisplayError::IoctlFailed("not streaming".into()));
        }

        let mut buf = v4l2_buffer {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            ..v4l2_buffer::default()
        };

        // VIDIOC_DQBUF = _IOWR('V', 17, struct v4l2_buffer)
        // SAFETY: ioctl FFI; kernel fills buf with dequeued buffer info. Invariants: fd valid.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Updater::<
                    rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 17, v4l2_buffer>,
                    v4l2_buffer,
                >::new(&mut buf),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_DQBUF: {e}")))?;

        let idx = buf.index as usize;
        let used = buf.bytesused as usize;
        let copy_len = out.len().min(used);

        if idx < self.buffers.len() {
            let src = self.buffers[idx].as_slice();
            out[..copy_len].copy_from_slice(&src[..copy_len]);
        }

        // Re-queue
        let mut rebuf = v4l2_buffer {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            index: buf.index,
            ..v4l2_buffer::default()
        };
        // SAFETY: ioctl FFI; kernel reads rebuf to re-queue. Invariants: fd valid, rebuf init'd.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Updater::<
                    rustix::ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 15, v4l2_buffer>,
                    v4l2_buffer,
                >::new(&mut rebuf),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_QBUF (re-queue): {e}")))?;

        Ok(copy_len)
    }

    /// Stop streaming and unmap buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if `VIDIOC_STREAMOFF` fails.
    pub fn stop_streaming(&mut self) -> Result<()> {
        if !self.streaming {
            return Ok(());
        }

        // VIDIOC_STREAMOFF = _IOW('V', 19, int)
        // SAFETY: ioctl FFI; kernel stops streaming. Invariants: fd valid, buffer type is u32.
        unsafe {
            rustix::ioctl::ioctl(
                &self.fd,
                rustix::ioctl::Setter::<
                    rustix::ioctl::WriteOpcode<VIDIOC_MAGIC, 19, u32>,
                    u32,
                >::new(V4L2_BUF_TYPE_VIDEO_CAPTURE),
            )
        }
        .map_err(|e| DisplayError::IoctlFailed(format!("VIDIOC_STREAMOFF: {e}")))?;

        self.buffers.clear();
        self.streaming = false;
        Ok(())
    }

    /// Get the device path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Current negotiated format (after `set_format`).
    #[must_use]
    pub const fn format(&self) -> Option<&CaptureFormat> {
        self.format.as_ref()
    }

    /// Discover all V4L2 video devices on the system.
    ///
    /// # Errors
    ///
    /// Returns an error if `/dev` cannot be read.
    pub fn discover_all() -> Result<Vec<PathBuf>> {
        let dev_dir = Path::new("/dev");
        let mut devices = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dev_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("video")
                {
                    devices.push(path);
                }
            }
        }

        devices.sort();
        Ok(devices)
    }
}

impl Drop for CaptureDevice {
    fn drop(&mut self) {
        let _ = self.stop_streaming();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_v4l2_capability_struct() {
        let cap = V4l2Capability {
            driver: "uvcvideo".to_string(),
            card: "Integrated Camera".to_string(),
            bus_info: "usb-0000:00:14.0-1".to_string(),
            capabilities: 0x85_20_00_01,
        };
        assert_eq!(cap.driver, "uvcvideo");
        assert_eq!(cap.card, "Integrated Camera");
        assert!(!cap.bus_info.is_empty());
    }

    #[test]
    fn test_capture_format_struct() {
        let fmt = CaptureFormat {
            width: 1920,
            height: 1080,
            fourcc: 0x56_59_55_59, // VYUY
            bytes_per_line: 3_840,
            image_size: 4_147_200,
        };
        assert_eq!(fmt.width, 1920);
        assert_eq!(fmt.height, 1080);
        assert_eq!(fmt.bytes_per_line, 3840);
        assert_eq!(fmt.image_size, 1920 * 1080 * 2);
    }

    #[test]
    fn test_capture_format_equality() {
        let fmt1 = CaptureFormat {
            width: 640,
            height: 480,
            fourcc: 0x32_31_56_59,
            bytes_per_line: 1_280,
            image_size: 614_400,
        };
        let fmt2 = fmt1;
        assert_eq!(fmt1, fmt2);
    }

    #[test]
    fn test_capture_device_open_nonexistent() {
        let result = CaptureDevice::open("/dev/nonexistent-video-99999");
        assert!(result.is_err());
        if let Err(e) = result {
            let s = format!("{e:?}");
            assert!(s.contains("NotFound") || s.contains("Device") || s.contains("path"));
        }
    }

    #[test]
    fn test_discover_all_returns_sorted_paths() {
        let result = CaptureDevice::discover_all();
        assert!(result.is_ok());
        let devices = result.unwrap();
        let sorted: Vec<_> = devices.iter().map(PathBuf::clone).collect();
        let mut sorted = sorted;
        sorted.sort();
        assert_eq!(devices, sorted, "discover_all should return sorted paths");
    }
}
