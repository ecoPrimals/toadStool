// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    missing_docs,
    dead_code,
    reason = "kernel ABI structs: field names match kernel headers, padding is structural"
)]
//! V4L2 kernel ABI structs (`#[repr(C)]`, zero-init safe).

/// V4L2 buffer type for single-plane video capture.
pub const V4L2_BUF_TYPE_VIDEO_CAPTURE: u32 = 1;
/// V4L2 memory type for mmap.
pub const V4L2_MEMORY_MMAP: u32 = 1;
/// V4L2 capability flag: supports video capture.
pub const V4L2_CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
/// V4L2 capability flag: supports streaming I/O.
pub const V4L2_CAP_STREAMING: u32 = 0x0400_0000;

#[repr(C)]
pub struct v4l2_capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Default)]
pub struct v4l2_pix_format {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub priv_: u32,
    pub flags: u32,
    pub hsv_enc_or_ycbcr_enc: u32,
    pub quantization: u32,
    pub xfer_func: u32,
}

#[repr(C)]
pub struct v4l2_format {
    pub type_: u32,
    pub fmt: v4l2_pix_format,
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
pub struct v4l2_requestbuffers {
    pub count: u32,
    pub type_: u32,
    pub memory: u32,
    pub capabilities: u32,
    pub flags: u8,
    pub reserved: [u8; 3],
    _pad: [u32; 4],
}

#[repr(C)]
#[derive(Default)]
pub struct v4l2_buffer {
    pub index: u32,
    pub type_: u32,
    pub bytesused: u32,
    pub flags: u32,
    pub field: u32,
    pub tv_sec: i64,
    pub tv_usec: i64,
    pub timecode: [u32; 4],
    pub sequence: u32,
    pub memory: u32,
    pub m_offset: u32,
    pub length: u32,
    pub reserved2: u32,
    pub request_fd_or_reserved: i32,
}
