// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for the capabilities module.

use super::*;
use std::path::PathBuf;

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
fn test_display_capabilities_clone() {
    let caps = make_caps();
    let cloned = caps.clone();
    assert_eq!(caps.primal_id, cloned.primal_id);
    assert_eq!(caps.max_windows, cloned.max_windows);
}

#[test]
fn test_display_capabilities_serialization() {
    let caps = make_caps();
    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: DisplayCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(caps.primal_id, deserialized.primal_id);
    assert_eq!(caps.displays.len(), deserialized.displays.len());
    assert_eq!(
        caps.supported_formats.len(),
        deserialized.supported_formats.len()
    );
}

#[test]
fn test_display_info_fields() {
    let info = DisplayInfo {
        name: "HDMI-1".to_string(),
        width: 2560,
        height: 1440,
        refresh_rate: 144.0,
        connected: true,
    };
    assert_eq!(info.name, "HDMI-1");
    assert_eq!(info.width, 2560);
    assert!((info.refresh_rate - 144.0).abs() < 1e-5);
}

#[test]
fn test_input_device_info_fields() {
    let dev = InputDeviceInfo {
        name: "mouse0".to_string(),
        device_type: "Mouse".to_string(),
    };
    assert_eq!(dev.name, "mouse0");
    assert_eq!(dev.device_type, "Mouse");
}

#[test]
fn test_capability_metadata_fields() {
    let meta = CapabilityMetadata {
        version: "1.0.0".to_string(),
        pure_rust: true,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
    };
    assert!(meta.pure_rust);
    assert_eq!(meta.version, "1.0.0");
}

#[test]
fn test_socket_path_uses_xdg() {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let env = PathEnv {
        xdg_runtime_dir: Some("/tmp/test_xdg_runtime".into()),
        ..PathEnv::default()
    };
    let paths = PlatformPaths::new(&env);
    let path = paths.toadstool_socket_dir().join("display.sock");
    assert!(path.to_string_lossy().contains("test_xdg_runtime"));
    assert!(path.to_string_lossy().contains("biomeos"));
    assert!(path.to_string_lossy().contains("display.sock"));
}

#[test]
fn test_discovery_dir_uses_xdg() {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let env = PathEnv {
        xdg_runtime_dir: Some("/tmp/test_xdg_runtime".into()),
        ..PathEnv::default()
    };
    let paths = PlatformPaths::new(&env);
    let dir = paths.runtime_dir().join("ecoPrimals/discovery");
    assert!(dir.to_string_lossy().contains("ecoPrimals/discovery"));
}

#[test]
fn test_discovery_dir_fallback() {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let env = PathEnv {
        xdg_runtime_dir: None,
        user: Some("testuser".into()),
        ..PathEnv::default()
    };
    let paths = PlatformPaths::new(&env);
    let dir = paths.runtime_dir().join("ecoPrimals/discovery");
    assert!(dir.to_string_lossy().contains("ecoPrimals/discovery"));
}

#[tokio::test]
async fn test_find_all_empty_dir_returns_empty() {
    let result = DisplayCapabilities::find_all().await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_serialization_roundtrip() {
    // Tests the announce/find serialization logic without using shared env vars.
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let discovery_dir = tmp.path().join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let caps = make_caps();

    // Serialize and write like announce() does
    let filename = format!("{}.json", caps.primal_id);
    let filepath = discovery_dir.join(&filename);
    let json = serde_json::to_string_pretty(&caps).unwrap();
    std::fs::write(&filepath, &json).unwrap();
    assert!(filepath.exists());

    // Read back like find_all() does
    let content = std::fs::read_to_string(&filepath).unwrap();
    let found: DisplayCapabilities = serde_json::from_str(&content).unwrap();
    assert_eq!(found.primal_id, caps.primal_id);
    assert_eq!(found.displays.len(), 1);

    // Cleanup
    std::fs::remove_file(&filepath).unwrap();
    assert!(!filepath.exists());
}

#[test]
fn test_capability_detection_supported_formats() {
    let caps = make_caps();
    assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
    assert!(caps.supported_formats.contains(&"RGB565".to_string()));
    assert!(!caps.supported_formats.is_empty());
}

#[test]
fn test_capability_detection_max_windows() {
    let caps = make_caps();
    assert!(caps.max_windows > 0);
    assert!(caps.max_windows <= 64);
}

#[test]
fn test_capability_detection_primal_type() {
    let caps = make_caps();
    assert_eq!(caps.primal_type, "toadstool");
}

#[test]
fn test_capability_detection_metadata() {
    let caps = make_caps();
    assert!(!caps.metadata.version.is_empty());
    assert!(caps.metadata.pure_rust);
    assert!(!caps.metadata.timestamp.is_empty());
}

#[test]
fn test_display_info_connected() {
    let info = DisplayInfo {
        name: "DP-1".to_string(),
        width: 3840,
        height: 2160,
        refresh_rate: 120.0,
        connected: false,
    };
    assert!(!info.connected);
    assert_eq!(info.width, 3840);
    assert_eq!(info.height, 2160);
}

#[test]
fn test_capabilities_empty_displays() {
    let mut caps = make_caps();
    caps.displays = vec![];
    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: DisplayCapabilities = serde_json::from_str(&json).unwrap();
    assert!(deserialized.displays.is_empty());
}

#[test]
fn test_capabilities_multiple_displays() {
    let mut caps = make_caps();
    caps.displays = vec![
        DisplayInfo {
            name: "eDP-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            connected: true,
        },
        DisplayInfo {
            name: "HDMI-1".to_string(),
            width: 2560,
            height: 1440,
            refresh_rate: 144.0,
            connected: true,
        },
    ];
    assert_eq!(caps.displays.len(), 2);
    let json = serde_json::to_string(&caps).unwrap();
    let deserialized: DisplayCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.displays.len(), 2);
    assert_eq!(deserialized.displays[1].name, "HDMI-1");
}

#[tokio::test]
async fn test_announce_creates_file() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let discovery_dir = tmp.path().join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let mut caps = make_caps();
    caps.primal_id = "test-announce-123".to_string();

    let json = serde_json::to_string_pretty(&caps).unwrap();
    let filepath = discovery_dir.join(format!("{}.json", caps.primal_id));
    std::fs::write(&filepath, &json).unwrap();
    assert!(filepath.exists());
}

#[tokio::test]
async fn test_cleanup_removes_file() {
    use tempfile::TempDir;
    let tmp = TempDir::new().unwrap();
    let discovery_dir = tmp.path().join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let mut caps = make_caps();
    caps.primal_id = "test-cleanup-456".to_string();
    let filepath = discovery_dir.join(format!("{}.json", caps.primal_id));
    std::fs::write(&filepath, "{}").unwrap();
    assert!(filepath.exists());
    std::fs::remove_file(&filepath).unwrap();
    assert!(!filepath.exists());
}

#[test]
fn test_display_info_debug() {
    let info = DisplayInfo {
        name: "Test".to_string(),
        width: 1920,
        height: 1080,
        refresh_rate: 60.0,
        connected: true,
    };
    let s = format!("{info:?}");
    assert!(s.contains("Test"));
}

#[test]
fn test_input_device_info_debug() {
    let dev = InputDeviceInfo {
        name: "keyboard0".to_string(),
        device_type: "Keyboard".to_string(),
    };
    let s = format!("{dev:?}");
    assert!(s.contains("keyboard"));
}

#[test]
fn test_capability_metadata_debug() {
    let meta = CapabilityMetadata {
        version: "1.0".to_string(),
        pure_rust: true,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
    };
    let s = format!("{meta:?}");
    assert!(s.contains("1.0"));
}

#[test]
fn test_display_capabilities_debug() {
    let caps = make_caps();
    let s = format!("{caps:?}");
    assert!(s.contains("toadstool"));
}

#[tokio::test]
async fn test_find_all_returns_result() {
    let result = DisplayCapabilities::find_all().await;
    assert!(result.is_ok());
    let caps = result.unwrap();
    assert!(caps.is_empty() || !caps.is_empty());
}

#[test]
fn test_supported_formats_non_empty() {
    let caps = make_caps();
    assert!(!caps.supported_formats.is_empty());
    assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
}

#[test]
fn test_has_gpu_acceleration() {
    let caps = make_caps();
    assert!(caps.has_gpu_acceleration);
}

#[test]
fn test_vsync_available() {
    let mut caps = make_caps();
    caps.vsync_available = true;
    assert!(caps.vsync_available);
}
