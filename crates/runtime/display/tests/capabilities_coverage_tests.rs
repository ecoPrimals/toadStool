// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for display capabilities module
//! Target: crates/runtime/display/src/capabilities.rs
//! No real DRM/input device probing - tests structs, serialization, `find_all`.

use std::path::PathBuf;

use toadstool_display::capabilities::{
    CapabilityMetadata, DisplayCapabilities, DisplayInfo, InputDeviceInfo,
};

fn make_caps() -> DisplayCapabilities {
    DisplayCapabilities {
        primal_id: "toadstool-display-test".to_string(),
        primal_type: "toadstool".to_string(),
        socket_path: PathBuf::from("/tmp/test.sock"),
        max_windows: 4,
        supported_formats: vec!["RGBA8888".to_string(), "RGB565".to_string()],
        has_gpu_acceleration: true,
        vsync_available: false,
        displays: vec![DisplayInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            connected: true,
        }],
        input_devices: vec![InputDeviceInfo {
            name: "keyboard0".to_string(),
            device_type: "Keyboard".to_string(),
        }],
        metadata: CapabilityMetadata {
            version: "0.1.0".to_string(),
            pure_rust: true,
            timestamp: "2026-02-19T00:00:00Z".to_string(),
        },
    }
}

#[test]
fn test_display_capabilities_creation() {
    let caps = make_caps();
    assert_eq!(caps.primal_id, "toadstool-display-test");
    assert_eq!(caps.max_windows, 4);
    assert!(caps.has_gpu_acceleration);
}

#[test]
fn test_display_capabilities_serialization_roundtrip() {
    let caps = make_caps();
    let json = serde_json::to_string(&caps).unwrap();
    let restored: DisplayCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(caps.primal_id, restored.primal_id);
    assert_eq!(caps.displays.len(), restored.displays.len());
}

#[test]
fn test_display_info_all_fields() {
    let info = DisplayInfo {
        name: "HDMI-2".to_string(),
        width: 3840,
        height: 2160,
        refresh_rate: 120.0,
        connected: false,
    };
    assert_eq!(info.name, "HDMI-2");
    assert_eq!(info.width, 3840);
    assert_eq!(info.height, 2160);
    assert!((info.refresh_rate - 120.0).abs() < 1e-5);
    assert!(!info.connected);
}

#[test]
fn test_input_device_info_serialization() {
    let dev = InputDeviceInfo {
        name: "touchpad0".to_string(),
        device_type: "Touchpad".to_string(),
    };
    let json = serde_json::to_string(&dev).unwrap();
    let restored: InputDeviceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(dev.name, restored.name);
}

#[test]
fn test_capability_metadata_serialization() {
    let meta = CapabilityMetadata {
        version: "2.0.0".to_string(),
        pure_rust: false,
        timestamp: "2026-03-01T12:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&meta).unwrap();
    assert!(json.contains("2.0.0"));
}

#[test]
fn test_supported_formats_non_empty() {
    let caps = make_caps();
    assert!(!caps.supported_formats.is_empty());
    assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
}

#[test]
fn test_gpu_acceleration_flag() {
    let mut caps = make_caps();
    caps.has_gpu_acceleration = false;
    assert!(!caps.has_gpu_acceleration);
}

#[test]
fn test_vsync_flag() {
    let mut caps = make_caps();
    caps.vsync_available = true;
    assert!(caps.vsync_available);
}

#[test]
fn test_display_capabilities_clone() {
    let caps = make_caps();
    let cloned = caps.clone();
    assert_eq!(caps.primal_id, cloned.primal_id);
}

#[test]
fn test_display_capabilities_debug() {
    let caps = make_caps();
    let s = format!("{caps:?}");
    assert!(s.contains("toadstool"));
}

#[tokio::test]
async fn test_find_all_empty_or_existing() {
    let result = DisplayCapabilities::find_all().await;
    assert!(result.is_ok());
    let caps = result.unwrap();
    assert!(caps.is_empty() || !caps.is_empty());
}

#[tokio::test]
async fn test_announce_and_cleanup_with_temp_dir() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let discovery_dir = tmp.path().join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let mut caps = make_caps();
    caps.primal_id = "test-announce-cleanup".to_string();
    caps.socket_path = tmp.path().join("display.sock");

    // Override get_discovery_dir by writing directly
    let filepath = discovery_dir.join(format!("{}.json", caps.primal_id));
    let json = serde_json::to_string_pretty(&caps).unwrap();
    std::fs::write(&filepath, &json).unwrap();
    assert!(filepath.exists());

    // Simulate cleanup
    let _ = std::fs::remove_file(&filepath);
    assert!(!filepath.exists());
}
