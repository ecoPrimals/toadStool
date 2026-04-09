// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // V4L2 ioctls are kernel FFI — this is the containment zone

//! Safe wrappers for V4L2 ioctls.
//!
//! Three generic helpers (`v4l2_get`, `v4l2_update`, `v4l2_set`) concentrate
//! all `unsafe` into exactly 3 blocks — one per ioctl direction. Every public
//! function is safe code that delegates to the appropriate helper.

use std::os::unix::io::{AsFd, AsRawFd};

use rustix::ioctl;

use super::types::*;

const VIDIOC_MAGIC: u8 = b'V';

fn ioctl_err(e: rustix::io::Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(e.raw_os_error())
}

/// Reject obviously invalid fds before issuing ioctls (closed fds are not detectable here).
fn validate_v4l2_fd(fd: &impl AsFd) -> Result<(), std::io::Error> {
    let raw = fd.as_fd().as_raw_fd();
    if raw < 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "V4L2 ioctl: negative file descriptor",
        ));
    }
    Ok(())
}

/// Read-only V4L2 ioctl: kernel fills a new `T` (VIDIOC_QUERYCAP pattern).
fn v4l2_get<const NR: u8, T>(fd: impl AsFd) -> Result<T, std::io::Error> {
    validate_v4l2_fd(&fd)?;
    // SAFETY: fd is a valid V4L2 device (AsFd contract); opcode
    // (VIDIOC_MAGIC, NR) is a compile-time constant matching V4L2 spec;
    // Getter allocates output and kernel writes into it.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Getter::<ioctl::ReadOpcode<VIDIOC_MAGIC, NR, T>, T>::new(),
        )
    }
    .map_err(ioctl_err)
}

/// Read-write V4L2 ioctl: kernel reads `*arg` and may write back
/// (VIDIOC_S_FMT, REQBUFS, QUERYBUF, QBUF, DQBUF pattern).
fn v4l2_update<const NR: u8, T>(fd: impl AsFd, arg: &mut T) -> Result<(), std::io::Error> {
    validate_v4l2_fd(&fd)?;
    // SAFETY: fd valid; opcode matches V4L2 spec; arg points to a live,
    // correctly-typed repr(C) struct the kernel reads/writes.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Updater::<ioctl::ReadWriteOpcode<VIDIOC_MAGIC, NR, T>, T>::new(arg),
        )
    }
    .map_err(ioctl_err)
}

/// Write-only V4L2 ioctl: kernel reads the value
/// (VIDIOC_STREAMON, STREAMOFF pattern).
fn v4l2_set<const NR: u8, T>(fd: impl AsFd, val: T) -> Result<(), std::io::Error> {
    validate_v4l2_fd(&fd)?;
    // SAFETY: fd valid; opcode matches V4L2 spec; val is the correct type.
    unsafe {
        ioctl::ioctl(
            fd,
            ioctl::Setter::<ioctl::WriteOpcode<VIDIOC_MAGIC, NR, T>, T>::new(val),
        )
    }
    .map_err(ioctl_err)
}

// ── Public wrappers (all safe) ──────────────────────────────────────────

/// `VIDIOC_QUERYCAP` — query device capabilities.
pub fn querycap(fd: impl AsFd) -> Result<v4l2_capability, std::io::Error> {
    v4l2_get::<0, _>(fd)
}

/// `VIDIOC_S_FMT` — set capture format (kernel may negotiate).
pub fn s_fmt(fd: impl AsFd, fmt: &mut v4l2_format) -> Result<(), std::io::Error> {
    v4l2_update::<5, _>(fd, fmt)
}

/// `VIDIOC_REQBUFS` — request mmap buffers.
pub fn reqbufs(fd: impl AsFd, req: &mut v4l2_requestbuffers) -> Result<(), std::io::Error> {
    v4l2_update::<8, _>(fd, req)
}

/// `VIDIOC_QUERYBUF` — query buffer info for mmap.
pub fn querybuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    v4l2_update::<9, _>(fd, buf)
}

/// `VIDIOC_QBUF` — queue a buffer for capture.
pub fn qbuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    v4l2_update::<15, _>(fd, buf)
}

/// `VIDIOC_DQBUF` — dequeue a filled buffer.
pub fn dqbuf(fd: impl AsFd, buf: &mut v4l2_buffer) -> Result<(), std::io::Error> {
    v4l2_update::<17, _>(fd, buf)
}

/// `VIDIOC_STREAMON` — start streaming.
pub fn streamon(fd: impl AsFd, buf_type: u32) -> Result<(), std::io::Error> {
    v4l2_set::<18, _>(fd, buf_type)
}

/// `VIDIOC_STREAMOFF` — stop streaming.
pub fn streamoff(fd: impl AsFd, buf_type: u32) -> Result<(), std::io::Error> {
    v4l2_set::<19, _>(fd, buf_type)
}
