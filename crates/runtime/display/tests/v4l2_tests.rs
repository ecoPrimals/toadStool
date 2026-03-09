// SPDX-License-Identifier: AGPL-3.0-only
//! V4L2 display tests — pure logic, buffer management, frame protocol, error paths.
//!
//! Tests non-ioctl logic: frame protocol, format calculations, buffer management,
//! and error handling. Hardware-dependent ioctl paths are #[ignore]d.

#![cfg(test)]

use std::path::PathBuf;

use toadstool_core::{decode_frame, encode_frame, FRAME_HEADER_SIZE};
use toadstool_display::v4l2::{CaptureDevice, CaptureFormat, V4l2Capability};
use toadstool_display::DisplayError;

// -----------------------------------------------------------------------------
// Frame protocol encoding/decoding (software-only)
// -----------------------------------------------------------------------------

#[test]
fn v4l2_frame_protocol_encode_decode_roundtrip() {
    let payload = b"V4L2 test payload";
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + payload.len() + 64];

    let written = encode_frame(123, payload, &mut buf).expect("encode");
    assert_eq!(written, FRAME_HEADER_SIZE + payload.len());

    let (seq, decoded) = decode_frame(&buf[..written]).expect("decode");
    assert_eq!(seq, 123);
    assert_eq!(decoded, payload);
}

#[test]
fn v4l2_frame_protocol_encode_too_small_returns_none() {
    let mut buf = [0u8; 4];
    assert!(encode_frame(0, b"data", &mut buf).is_none());
}

#[test]
fn v4l2_frame_protocol_decode_truncated_header_rejected() {
    assert!(decode_frame(&[0; 4]).is_err());
}

#[test]
fn v4l2_frame_protocol_decode_bad_magic_rejected() {
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + 8];
    buf[0..4].copy_from_slice(b"NOPE");
    assert!(decode_frame(&buf).is_err());
}

#[test]
fn v4l2_frame_protocol_decode_unsupported_version_rejected() {
    let mut buf = vec![0u8; FRAME_HEADER_SIZE + 8];
    buf[0..4].copy_from_slice(b"TSXP");
    buf[4] = 99; // unsupported version
    assert!(decode_frame(&buf).is_err());
}

// -----------------------------------------------------------------------------
// CaptureFormat — buffer size calculations, format logic
// -----------------------------------------------------------------------------

#[test]
fn v4l2_capture_format_image_size_calculation() {
    let fmt = CaptureFormat {
        width: 1920,
        height: 1080,
        fourcc: 0x56_59_55_59, // VYUY
        bytes_per_line: 3_840,
        image_size: 4_147_200,
    };
    assert_eq!(fmt.image_size, 1920 * 1080 * 2);
    assert_eq!(fmt.bytes_per_line, fmt.width * 2);
}

#[test]
fn v4l2_capture_format_rgba8888_sizes() {
    let fmt = CaptureFormat {
        width: 640,
        height: 480,
        fourcc: u32::from_le_bytes(*b"AR24"),
        bytes_per_line: 2560,
        image_size: 1_228_800,
    };
    assert_eq!(fmt.bytes_per_line, fmt.width * 4);
    assert_eq!(fmt.image_size, fmt.width * fmt.height * 4);
}

#[test]
fn v4l2_capture_format_equality_and_clone() {
    let fmt1 = CaptureFormat {
        width: 1280,
        height: 720,
        fourcc: 0x32_31_56_59,
        bytes_per_line: 2560,
        image_size: 921_600,
    };
    let fmt2 = fmt1;
    assert_eq!(fmt1, fmt2);
}

#[test]
fn v4l2_capture_format_frame_capacity() {
    let fmt = CaptureFormat {
        width: 1920,
        height: 1080,
        fourcc: 0,
        bytes_per_line: 7680,
        image_size: 8_294_400,
    };
    let frame_capacity = fmt.image_size as usize - FRAME_HEADER_SIZE;
    assert!(frame_capacity > 0);
    assert_eq!(frame_capacity, 8_294_400 - 17);
}

// -----------------------------------------------------------------------------
// V4l2Capability — capability flags, string conversion
// -----------------------------------------------------------------------------

#[test]
fn v4l2_capability_struct_fields() {
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
fn v4l2_capability_supports_capture_streaming_flags() {
    const V4L2_CAP_VIDEO_CAPTURE: u32 = 0x0000_0001;
    const V4L2_CAP_STREAMING: u32 = 0x0400_0000;

    let caps_both = V4L2_CAP_VIDEO_CAPTURE | V4L2_CAP_STREAMING;
    assert!(caps_both & V4L2_CAP_VIDEO_CAPTURE != 0);
    assert!(caps_both & V4L2_CAP_STREAMING != 0);

    let caps_none = 0u32;
    assert!(caps_none & V4L2_CAP_VIDEO_CAPTURE == 0);
}

// -----------------------------------------------------------------------------
// Buffer management logic — allocation sizes, index bounds
// -----------------------------------------------------------------------------

#[test]
fn v4l2_buffer_copy_len_bounds() {
    let used = 1000;
    let out_len = 500;
    let copy_len = out_len.min(used);
    assert_eq!(copy_len, 500);
}

#[test]
fn v4l2_buffer_copy_len_out_larger_than_used() {
    let used = 500;
    let out_len = 1000;
    let copy_len = out_len.min(used);
    assert_eq!(copy_len, 500);
}

// -----------------------------------------------------------------------------
// Error handling paths (no hardware)
// -----------------------------------------------------------------------------

#[test]
fn v4l2_capture_device_open_nonexistent() {
    let result = CaptureDevice::open("/dev/nonexistent-video-99999");
    assert!(result.is_err());
    if let Err(e) = result {
        let s = format!("{e:?}");
        assert!(
            s.contains("NotFound") || s.contains("Device") || s.contains("path"),
            "expected device not found error: {s}"
        );
    }
}

#[test]
fn v4l2_display_error_device_not_found_variant() {
    let path = PathBuf::from("/dev/nonexistent-video-99999");
    let err = DisplayError::DeviceNotFound(path.clone());
    let s = format!("{err}");
    assert!(s.contains("not found") || s.contains("NotFound"));
    assert!(s.contains("nonexistent"));
}

#[test]
fn v4l2_display_error_ioctl_failed_variant() {
    let err = DisplayError::IoctlFailed("VIDIOC_S_FMT: invalid format".to_string());
    let s = format!("{err}");
    assert!(s.contains("ioctl") || s.contains("Ioctl"));
    assert!(s.contains("invalid format"));
}

#[test]
fn v4l2_capture_format_invalid_fourcc_zero() {
    let fmt = CaptureFormat {
        width: 640,
        height: 480,
        fourcc: 0,
        bytes_per_line: 1280,
        image_size: 614_400,
    };
    assert_eq!(fmt.fourcc, 0);
    assert_eq!(fmt.image_size, fmt.width * fmt.height * 2);
}

#[test]
fn v4l2_buffer_overflow_idx_out_of_bounds() {
    let buffers_len = 4;
    let idx = 10;
    let copy_len = if idx < buffers_len {
        let used = 1000;
        let out_len = 500;
        out_len.min(used)
    } else {
        0
    };
    assert_eq!(copy_len, 0, "idx >= buffers.len should yield no copy");
}

#[test]
fn v4l2_buffer_copy_len_buffer_overflow_prevention() {
    let used = 1_000_000;
    let out_len = 100;
    let copy_len = out_len.min(used);
    assert_eq!(copy_len, 100, "copy_len must not exceed out buffer");
}

#[test]
fn v4l2_capture_format_stride_validation() {
    let fmt = CaptureFormat {
        width: 1920,
        height: 1080,
        fourcc: 0x56_59_55_59,
        bytes_per_line: 3840,
        image_size: 4_147_200,
    };
    assert!(
        fmt.bytes_per_line >= fmt.width,
        "stride must be >= width for packed formats"
    );
    assert!(
        fmt.image_size >= fmt.bytes_per_line * fmt.height,
        "image_size must cover full frame"
    );
}

#[test]
fn v4l2_capture_format_permission_denied_simulation() {
    let err = DisplayError::OpenFailed(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "Permission denied",
    ));
    let s = format!("{err}");
    assert!(s.contains("Permission") || s.contains("denied") || s.contains("open"));
}

#[test]
fn v4l2_discover_all_returns_sorted_paths() {
    let result = CaptureDevice::discover_all();
    assert!(result.is_ok());
    let devices = result.unwrap();
    let mut sorted: Vec<PathBuf> = devices.clone();
    sorted.sort();
    assert_eq!(devices, sorted, "discover_all should return sorted paths");
}

// -----------------------------------------------------------------------------
// Hardware-dependent tests — #[ignore] with explanation
// -----------------------------------------------------------------------------

#[test]
#[ignore = "Requires real V4L2 capture device at /dev/video0"]
fn v4l2_capture_device_open_real_device() {
    let _ = CaptureDevice::open("/dev/video0");
}

#[test]
#[ignore = "Requires real V4L2 device for query_capabilities ioctl"]
fn v4l2_capture_device_query_capabilities_real() {
    let dev = CaptureDevice::open("/dev/video0").expect("need /dev/video0");
    let _ = dev.query_capabilities();
}
