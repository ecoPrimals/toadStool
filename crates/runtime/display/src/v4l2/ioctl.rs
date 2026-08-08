// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code, reason = "V4L2 ioctls are kernel FFI — containment zone")]

//! Safe wrappers for V4L2 ioctls.
//!
//! One `unsafe` ioctl call per public function, hidden behind safe signatures.
//! Uses rustix 1.x `ioctl::opcode::*` const functions with concrete types at
//! each call site (const generics require known types, so we use macros to
//! eliminate boilerplate).
//!
//! Migrated from rustix 0.38 `ReadOpcode`/`WriteOpcode`/`ReadWriteOpcode` to
//! 1.x `ioctl::opcode::{read,write,read_write}` (S203: D-RUSTIX-DISPLAY-038).

use std::os::unix::io::{AsFd, AsRawFd};

use toadstool_hw_safe::ioctl_infra as ioctl;

use super::types::*;

fn ioctl_err(e: toadstool_hw_safe::ioctl_infra::Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(e.raw_os_error())
}

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

macro_rules! v4l2_getter {
    ($name:ident, $nr:literal, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(fd: impl AsFd) -> Result<$ty, std::io::Error> {
            validate_v4l2_fd(&fd)?;
            // SAFETY: `ioctl::ioctl` is the only FFI site in this module.
            // Invariants:
            // - `fd` refers to an open V4L2 character device (`validate_v4l2_fd` rejects negatives;
            //   callers must pass a handle obtained from `open` on a V4L2 node).
            // - The ioctl opcode is a compile-time `Getter` for `repr(C)` `$ty`, matching the
            //   kernel V4L2 ABI for request `b'V'`, nr `$nr`.
            // - Rust allocates a zeroed `$ty` on the stack; the kernel writes the output only.
            // Maintained by: safe public wrappers that require `AsFd` and use spec-locked types.
            // If violated: EINVAL/ENODEV from the kernel, or UB if `fd`/opcode/types are wrong.
            unsafe {
                ioctl::ioctl(
                    fd,
                    ioctl::Getter::<{ ioctl::opcode::read::<$ty>(b'V', $nr) }, $ty>::new(),
                )
            }
            .map_err(ioctl_err)
        }
    };
}

macro_rules! v4l2_updater {
    ($name:ident, $nr:literal, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(fd: impl AsFd, arg: &mut $ty) -> Result<(), std::io::Error> {
            validate_v4l2_fd(&fd)?;
            // SAFETY: `ioctl::ioctl` with `Updater` read/write semantics.
            // Invariants:
            // - `fd` is a valid V4L2 device handle (see `validate_v4l2_fd`).
            // - `arg` is `&mut $ty` where `$ty` is `#[repr(C)]` and matches the kernel struct
            //   for this ioctl; it remains borrowed for the full syscall (no concurrent alias).
            // - Opcode `read_write::<$ty>(b'V', $nr)` matches the V4L2 spec field layout.
            // Maintained by: callers pass stack- or heap-backed structs from `types.rs`.
            // If violated: kernel may reject the ioctl or corrupt adjacent memory on layout drift.
            unsafe {
                ioctl::ioctl(
                    fd,
                    ioctl::Updater::<{ ioctl::opcode::read_write::<$ty>(b'V', $nr) }, $ty>::new(
                        arg,
                    ),
                )
            }
            .map_err(ioctl_err)
        }
    };
}

macro_rules! v4l2_setter {
    ($name:ident, $nr:literal, $ty:ty, $doc:literal) => {
        #[doc = $doc]
        pub fn $name(fd: impl AsFd, val: $ty) -> Result<(), std::io::Error> {
            validate_v4l2_fd(&fd)?;
            // SAFETY: `ioctl::ioctl` with `Setter` write-only semantics.
            // Invariants:
            // - `fd` is a valid V4L2 device handle (see `validate_v4l2_fd`).
            // - `val` is passed by value with type `$ty` matching the kernel argument for
            //   `write::<$ty>(b'V', $nr)`; no Rust references cross the FFI boundary.
            // Maintained by: opcode constants and `types.rs` definitions track the V4L2 UAPI.
            // If violated: EINVAL from the kernel, or UB if `fd` is not a V4L2 device.
            unsafe {
                ioctl::ioctl(
                    fd,
                    ioctl::Setter::<{ ioctl::opcode::write::<$ty>(b'V', $nr) }, $ty>::new(val),
                )
            }
            .map_err(ioctl_err)
        }
    };
}

// ── Public wrappers (all safe) ──────────────────────────────────────────

v4l2_getter!(
    querycap,
    0,
    v4l2_capability,
    "`VIDIOC_QUERYCAP` — query device capabilities."
);
v4l2_updater!(
    s_fmt,
    5,
    v4l2_format,
    "`VIDIOC_S_FMT` — set capture format (kernel may negotiate)."
);
v4l2_updater!(
    reqbufs,
    8,
    v4l2_requestbuffers,
    "`VIDIOC_REQBUFS` — request mmap buffers."
);
v4l2_updater!(
    querybuf,
    9,
    v4l2_buffer,
    "`VIDIOC_QUERYBUF` — query buffer info for mmap."
);
v4l2_updater!(
    qbuf,
    15,
    v4l2_buffer,
    "`VIDIOC_QBUF` — queue a buffer for capture."
);
v4l2_updater!(
    dqbuf,
    17,
    v4l2_buffer,
    "`VIDIOC_DQBUF` — dequeue a filled buffer."
);
v4l2_setter!(streamon, 18, u32, "`VIDIOC_STREAMON` — start streaming.");
v4l2_setter!(streamoff, 19, u32, "`VIDIOC_STREAMOFF` — stop streaming.");
