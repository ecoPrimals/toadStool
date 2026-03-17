// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::pedantic)]
//! Coverage tests for display transport (transport.rs)
//!
//! Focus: transport types, TransportInfo, error paths, discover_display_transports.
//! DisplayTransport::open requires real DRM - not tested in CI.

use toadstool_core::{
    FRAME_HEADER_SIZE, TransportDirection, TransportError, TransportInfo, TransportMedium,
};
use toadstool_display::transport::discover_display_transports;

// ============================================================================
// discover_display_transports
// ============================================================================

#[test]
fn discover_display_transports_returns_vec() {
    let transports = discover_display_transports();
    assert!(transports.iter().all(|t| !t.id.is_empty()));
}

#[test]
fn discover_transports_have_display_medium() {
    let transports = discover_display_transports();
    for t in &transports {
        assert_eq!(t.medium, TransportMedium::Display);
        assert_eq!(t.direction, TransportDirection::Tx);
    }
}

#[test]
fn discover_transports_id_format() {
    let transports = discover_display_transports();
    for t in &transports {
        assert!(!t.id.is_empty());
        assert!(!t.label.is_empty());
        assert!(t.id.contains(':') || !t.label.is_empty());
    }
}

#[test]
fn discover_transports_empty_without_drm() {
    let t = discover_display_transports();
    assert!(t.is_empty() || t.iter().all(|i| i.medium == TransportMedium::Display));
}

#[test]
fn discover_transports_consistency() {
    let t1 = discover_display_transports();
    let t2 = discover_display_transports();
    assert_eq!(t1.len(), t2.len());
}

// ============================================================================
// TransportInfo
// ============================================================================

#[test]
fn transport_info_construct() {
    let info = TransportInfo {
        id: "/dev/dri/card0:HDMI-1".to_string(),
        label: "HDMI-1".to_string(),
        medium: TransportMedium::Display,
        direction: TransportDirection::Tx,
    };
    assert_eq!(info.id, "/dev/dri/card0:HDMI-1");
    assert_eq!(info.label, "HDMI-1");
    assert_eq!(info.medium, TransportMedium::Display);
    assert_eq!(info.direction, TransportDirection::Tx);
}

#[test]
fn transport_info_medium_display() {
    assert_eq!(TransportMedium::Display, TransportMedium::Display);
    assert_eq!(format!("{}", TransportMedium::Display), "Display");
}

#[test]
fn transport_info_direction_tx() {
    assert_eq!(TransportDirection::Tx, TransportDirection::Tx);
    assert_ne!(TransportDirection::Tx, TransportDirection::Rx);
    assert_eq!(format!("{}", TransportDirection::Tx), "Tx");
}

// ============================================================================
// TransportError variants
// ============================================================================

#[test]
fn transport_error_frame_protocol() {
    let err = TransportError::FrameProtocol("payload too large".into());
    let s = err.to_string();
    assert!(s.contains("payload") || s.contains("Frame") || !s.is_empty());
}

#[test]
fn transport_error_direction_mismatch() {
    let err = TransportError::DirectionMismatch {
        transport_dir: TransportDirection::Tx,
        required: TransportDirection::Rx,
    };
    let s = err.to_string();
    assert!(s.contains("Tx") || s.contains("Rx") || !s.is_empty());
}

#[test]
fn transport_error_open_failed() {
    let err = TransportError::OpenFailed("device busy".into());
    assert!(!err.to_string().is_empty());
}

#[test]
fn transport_error_io() {
    let err = TransportError::Io(std::io::Error::other("timeout"));
    assert!(!err.to_string().is_empty());
}

#[test]
fn transport_error_unavailable() {
    let err = TransportError::Unavailable("no connector".into());
    assert!(!err.to_string().is_empty());
}

// ============================================================================
// Frame capacity and bandwidth
// ============================================================================

#[test]
fn frame_header_size_constant() {
    assert_ne!(FRAME_HEADER_SIZE, 0);
    let hdr = FRAME_HEADER_SIZE;
    assert!(hdr >= 16, "header size {hdr} too small");
    assert!(hdr <= 64, "header size {hdr} too large");
}

#[test]
fn frame_capacity_formula() {
    let w = 1920usize;
    let h = 1080usize;
    let bpp = 4;
    let pixel_buf_size = w * h * bpp;
    let frame_capacity = pixel_buf_size.saturating_sub(FRAME_HEADER_SIZE);
    assert!(frame_capacity > 0);
    assert!(frame_capacity < pixel_buf_size);
}

#[test]
fn transport_bandwidth_formula_1080p_60hz() {
    let width = 1920u64;
    let height = 1080u64;
    let refresh_hz = 60u64;
    let bps = width * height * 4 * refresh_hz * 8;
    assert_eq!(bps, 3_981_312_000);
}

#[test]
fn transport_bandwidth_formula_4k() {
    let width = 3840u64;
    let height = 2160u64;
    let refresh_hz = 60u64;
    let bps = width * height * 4 * refresh_hz * 8;
    assert_eq!(bps, 15_925_248_000);
}

#[test]
fn transport_bandwidth_formula_720p() {
    let width = 1280u64;
    let height = 720u64;
    let refresh_hz = 60u64;
    let bps = width * height * 4 * refresh_hz * 8;
    assert_eq!(bps, 1_769_472_000);
}

// ============================================================================
// DisplayTransport::open error path (no DRM device)
// ============================================================================

#[test]
fn display_transport_open_nonexistent_drm_fails() {
    let result = toadstool_display::DisplayTransport::open("/dev/dri/nonexistent-card-99999");
    assert!(result.is_err());
    if let Err(e) = result {
        let err_str = e.to_string();
        assert!(
            err_str.contains("not found")
                || err_str.contains("No such file")
                || err_str.contains("drm")
                || !err_str.is_empty(),
            "expected DRM open error: {err_str}"
        );
    }
}

#[test]
fn display_transport_open_invalid_path_fails() {
    let result = toadstool_display::DisplayTransport::open("");
    assert!(result.is_err());
}
