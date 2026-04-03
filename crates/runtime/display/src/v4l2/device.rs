// SPDX-License-Identifier: AGPL-3.0-only
//! V4L2 capture device — pure safe Rust.
//!
//! Wraps `/dev/video*` devices for reading video frames from HDMI capture cards.
//! Uses `mmap` streaming I/O for zero-copy frame delivery.
//!
//! All kernel FFI (ioctls) is delegated to [`super::ioctl`]; this file
//! contains zero `unsafe` blocks.

use crate::{DisplayError, Result};
use rustix::fd::OwnedFd;
use rustix::fs;
use std::os::unix::io::{AsFd, BorrowedFd};
use std::path::{Path, PathBuf};

use super::ioctl;
use super::types::*;

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

/// A V4L2 capture device.
pub struct CaptureDevice {
    path: PathBuf,
    fd: OwnedFd,
    format: Option<CaptureFormat>,
    buffers: Vec<MmapBuffer>,
    streaming: bool,
}

struct MmapBuffer {
    mmap: toadstool_hw_safe::DeviceMmap,
}

impl MmapBuffer {
    fn as_slice(&self) -> &[u8] {
        self.mmap.as_slice()
    }
}

impl AsFd for CaptureDevice {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

fn ioctl_err(op: &str, e: &std::io::Error) -> DisplayError {
    DisplayError::IoctlFailed(format!("{op}: {e}"))
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
        let cap = ioctl::querycap(&self.fd)
            .map_err(|e| ioctl_err("VIDIOC_QUERYCAP", &e))?;

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

        ioctl::s_fmt(&self.fd, &mut fmt)
            .map_err(|e| ioctl_err("VIDIOC_S_FMT", &e))?;

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

        ioctl::reqbufs(&self.fd, &mut req)
            .map_err(|e| ioctl_err("VIDIOC_REQBUFS", &e))?;

        for i in 0..req.count {
            let mut buf = v4l2_buffer {
                type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
                memory: V4L2_MEMORY_MMAP,
                index: i,
                ..v4l2_buffer::default()
            };

            ioctl::querybuf(&self.fd, &mut buf)
                .map_err(|e| ioctl_err("VIDIOC_QUERYBUF", &e))?;

            let mmap = toadstool_hw_safe::DeviceMmap::map_shared_rw(
                &self.fd,
                u64::from(buf.m_offset),
                buf.length as usize,
            )
            .map_err(|e| DisplayError::IoctlFailed(format!("mmap: {e}")))?;

            self.buffers.push(MmapBuffer { mmap });
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

            ioctl::qbuf(&self.fd, &mut buf)
                .map_err(|e| ioctl_err("VIDIOC_QBUF", &e))?;
        }

        ioctl::streamon(&self.fd, V4L2_BUF_TYPE_VIDEO_CAPTURE)
            .map_err(|e| ioctl_err("VIDIOC_STREAMON", &e))?;

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

        ioctl::dqbuf(&self.fd, &mut buf)
            .map_err(|e| ioctl_err("VIDIOC_DQBUF", &e))?;

        let idx = buf.index as usize;
        let used = buf.bytesused as usize;
        let copy_len = out.len().min(used);

        if idx < self.buffers.len() {
            let src = self.buffers[idx].as_slice();
            out[..copy_len].copy_from_slice(&src[..copy_len]);
        }

        let mut rebuf = v4l2_buffer {
            type_: V4L2_BUF_TYPE_VIDEO_CAPTURE,
            memory: V4L2_MEMORY_MMAP,
            index: buf.index,
            ..v4l2_buffer::default()
        };
        ioctl::qbuf(&self.fd, &mut rebuf)
            .map_err(|e| ioctl_err("VIDIOC_QBUF (re-queue)", &e))?;

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

        ioctl::streamoff(&self.fd, V4L2_BUF_TYPE_VIDEO_CAPTURE)
            .map_err(|e| ioctl_err("VIDIOC_STREAMOFF", &e))?;

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
