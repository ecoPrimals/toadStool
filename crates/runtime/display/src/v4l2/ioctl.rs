// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // V4L2 ioctls are kernel FFI — this is the containment zone

//! Safe wrappers for V4L2 ioctls.
//!
//! Each function wraps a single `ioctl(2)` call with proper typing so that
//! consumers in `device.rs` never write `unsafe` directly.

use std::os::unix::io::AsFd;

use rustix::ioctl;

use super::types::*;

const VIDIOC_MAGIC: u8 = b'V';

/// `VIDIOC_QUERYCAP` — query device capabilities.
pub fn querycap(fd: impl AsFd) -> Result<v4l2_capability, std::io::Error> {
    // SAFETY: fd is valid (AsFd); Getter allocates output and kernel writes into it.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Getter::<ioctl::ReadOpcode<VIDIOC_MAGIC, 0, v4l2_capability>, v4l2_capability>::new(),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_S_FMT` — set capture format (kernel may negotiate).
pub fn s_fmt(fd: impl AsFd, fmt: &mut v4l2_format) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; Updater passes fmt by mut ref; kernel reads and may write back.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 5, v4l2_format>, v4l2_format>::new(fmt),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_REQBUFS` — request mmap buffers.
pub fn reqbufs(
    fd: impl AsFd,
    req: &mut v4l2_requestbuffers,
) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; kernel reads req and writes back count.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<
                ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 8, v4l2_requestbuffers>,
                v4l2_requestbuffers,
            >::new(req),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_QUERYBUF` — query buffer info for mmap.
pub fn querybuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; kernel fills buf with offset/length for mmap.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<
                ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 9, v4l2_buffer>,
                v4l2_buffer,
            >::new(buf),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_QBUF` — queue a buffer for capture.
pub fn qbuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; kernel reads buf to enqueue.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<
                ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 15, v4l2_buffer>,
                v4l2_buffer,
            >::new(buf),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_DQBUF` — dequeue a filled buffer.
pub fn dqbuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; kernel fills buf with dequeued frame info.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<
                ioctl::ReadWriteOpcode<VIDIOC_MAGIC, 17, v4l2_buffer>,
                v4l2_buffer,
            >::new(buf),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_STREAMON` — start streaming.
pub fn streamon(fd: impl AsFd, buf_type: u32) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; Setter writes the buffer type to the kernel.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Setter::<ioctl::WriteOpcode<VIDIOC_MAGIC, 18, u32>, u32>::new(buf_type),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}

/// `VIDIOC_STREAMOFF` — stop streaming.
pub fn streamoff(fd: impl AsFd, buf_type: u32) -> Result<(), std::io::Error> {
    // SAFETY: fd is valid; Setter writes the buffer type to the kernel.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Setter::<ioctl::WriteOpcode<VIDIOC_MAGIC, 19, u32>, u32>::new(buf_type),
        )
    }
    .map_err(|e| std::io::Error::from_raw_os_error(e.raw_os_error()))
}
