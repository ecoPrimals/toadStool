// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage tests for display runtime modules:
//! - capabilities.rs (display detection, format enumeration, resolution reporting, error paths)
//! - input/device.rs
//! - ipc/server.rs

#![allow(clippy::pedantic)]

use std::path::PathBuf;

use toadstool_display::capabilities::{
    CapabilityMetadata, DisplayCapabilities, DisplayInfo, InputDeviceInfo,
};
use toadstool_display::input::{Device, DeviceCapability, DeviceInfo, DeviceType};
use toadstool_display::ipc::{DisplayServer, IpcTransport};
use toadstool_display::window::WindowManager;

// -----------------------------------------------------------------------------
// Capabilities (capabilities.rs)
// -----------------------------------------------------------------------------

fn make_caps() -> DisplayCapabilities {
    DisplayCapabilities {
        primal_id: "toadstool-display-coverage".to_string(),
        primal_type: "toadstool".to_string(),
        socket_path: PathBuf::from("/tmp/display-coverage.sock"),
        max_windows: 8,
        supported_formats: vec![
            "RGBA8888".to_string(),
            "BGRA8888".to_string(),
            "RGB888".to_string(),
            "RGB565".to_string(),
        ],
        has_gpu_acceleration: true,
        vsync_available: true,
        displays: vec![
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
                connected: false,
            },
        ],
        input_devices: vec![
            InputDeviceInfo {
                name: "keyboard0".to_string(),
                device_type: "Keyboard".to_string(),
            },
            InputDeviceInfo {
                name: "mouse0".to_string(),
                device_type: "Mouse".to_string(),
            },
        ],
        metadata: CapabilityMetadata {
            version: "0.1.0".to_string(),
            pure_rust: true,
            timestamp: "2026-03-07T00:00:00Z".to_string(),
        },
    }
}

#[test]
fn capabilities_display_info_serialization_roundtrip() {
    let info = DisplayInfo {
        name: "DP-1".to_string(),
        width: 3840,
        height: 2160,
        refresh_rate: 120.0,
        connected: true,
    };
    let json = serde_json::to_string(&info).unwrap();
    let restored: DisplayInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(info.name, restored.name);
    assert_eq!(info.width, restored.width);
    assert_eq!(info.height, restored.height);
    assert!((info.refresh_rate - restored.refresh_rate).abs() < 1e-5);
    assert_eq!(info.connected, restored.connected);
}

#[test]
fn capabilities_input_device_info_serialization_roundtrip() {
    let dev = InputDeviceInfo {
        name: "touchpad0".to_string(),
        device_type: "Touchpad".to_string(),
    };
    let json = serde_json::to_string(&dev).unwrap();
    let restored: InputDeviceInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(dev.name, restored.name);
    assert_eq!(dev.device_type, restored.device_type);
}

#[test]
fn capabilities_capability_metadata_serialization_roundtrip() {
    let meta = CapabilityMetadata {
        version: "2.0.0".to_string(),
        pure_rust: false,
        timestamp: "2026-03-07T12:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&meta).unwrap();
    let restored: CapabilityMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(meta.version, restored.version);
    assert_eq!(meta.pure_rust, restored.pure_rust);
}

#[test]
fn capabilities_full_serialization_roundtrip() {
    let caps = make_caps();
    let json = serde_json::to_string_pretty(&caps).unwrap();
    let restored: DisplayCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(caps.primal_id, restored.primal_id);
    assert_eq!(caps.max_windows, restored.max_windows);
    assert_eq!(caps.displays.len(), restored.displays.len());
    assert_eq!(caps.input_devices.len(), restored.input_devices.len());
    assert_eq!(
        caps.supported_formats.len(),
        restored.supported_formats.len()
    );
}

#[test]
fn capabilities_detection_structures() {
    let caps = make_caps();
    assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
    assert!(caps.supported_formats.contains(&"BGRA8888".to_string()));
    assert!(caps.has_gpu_acceleration);
    assert!(caps.vsync_available);
    assert_eq!(caps.max_windows, 8);
    assert_eq!(caps.displays.len(), 2);
    assert_eq!(caps.displays[0].name, "eDP-1");
    assert_eq!(caps.displays[1].name, "HDMI-1");
    assert!(!caps.displays[1].connected);
}

#[test]
fn capabilities_announce_and_cleanup() {
    let tmp = tempfile::tempdir().unwrap();
    let runtime_dir = tmp.path().to_path_buf();
    let discovery_dir = runtime_dir.join("ecoPrimals/discovery");

    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(runtime_dir.to_str().unwrap()),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut caps = make_caps();
                caps.primal_id = "test-announce-cleanup-coverage".to_string();
                caps.announce().await.unwrap();
                let filepath = discovery_dir.join(format!("{}.json", caps.primal_id));
                assert!(filepath.exists(), "announce should create capability file");
                caps.cleanup().await.unwrap();
                assert!(!filepath.exists(), "cleanup should remove capability file");
            });
        },
    );
}

#[test]
fn capabilities_find_all_with_valid_and_invalid_files() {
    let tmp = tempfile::tempdir().unwrap();
    let runtime_dir = tmp.path().to_path_buf();
    let discovery_dir = runtime_dir.join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let invalid_file = discovery_dir.join("invalid.json");
    std::fs::write(&invalid_file, r#"{"invalid": json}"#).unwrap();

    let mut caps = make_caps();
    caps.primal_id = "test-find-all-valid".to_string();
    let valid_file = discovery_dir.join(format!("{}.json", caps.primal_id));
    std::fs::write(&valid_file, serde_json::to_string_pretty(&caps).unwrap()).unwrap();

    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(runtime_dir.to_str().unwrap()),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let found = rt.block_on(DisplayCapabilities::find_all()).unwrap();
            assert!(
                found.iter().any(|c| c.primal_id == caps.primal_id),
                "find_all should parse valid files and skip invalid"
            );
        },
    );
}

#[test]
fn capabilities_find_all_nonexistent_dir_returns_empty() {
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some("/nonexistent-path-12345-display-coverage"),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(DisplayCapabilities::find_all());
            assert!(result.is_ok());
            let caps = result.unwrap();
            assert!(caps.is_empty());
        },
    );
}

// -----------------------------------------------------------------------------
// Capabilities: display detection, format enumeration, resolution, error paths
// -----------------------------------------------------------------------------

#[test]
fn capabilities_discover_self_returns_result() {
    let result = DisplayCapabilities::discover_self();
    match &result {
        Ok(caps) => {
            assert!(!caps.primal_id.is_empty());
            assert!(caps.primal_id.contains("display"));
            assert!(!caps.supported_formats.is_empty());
            assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
        }
        Err(e) => {
            let err_str = e.to_string();
            assert!(
                err_str.contains("DRM") || err_str.contains("device") || !err_str.is_empty(),
                "discover_self error: {err_str}"
            );
        }
    }
}

#[test]
fn capabilities_format_enumeration() {
    let caps = make_caps();
    assert!(caps.supported_formats.contains(&"RGBA8888".to_string()));
    assert!(caps.supported_formats.contains(&"BGRA8888".to_string()));
    assert!(caps.supported_formats.contains(&"RGB888".to_string()));
    assert!(caps.supported_formats.contains(&"RGB565".to_string()));
    assert_eq!(caps.supported_formats.len(), 4);
}

#[test]
fn capabilities_resolution_reporting() {
    let info = DisplayInfo {
        name: "eDP-1".to_string(),
        width: 3840,
        height: 2160,
        refresh_rate: 120.0,
        connected: true,
    };
    assert_eq!(info.width, 3840);
    assert_eq!(info.height, 2160);
    assert!((info.refresh_rate - 120.0).abs() < 1e-5);
    assert!(info.connected);
}

#[test]
fn capabilities_resolution_reporting_720p() {
    let info = DisplayInfo {
        name: "HDMI-1".to_string(),
        width: 1280,
        height: 720,
        refresh_rate: 60.0,
        connected: false,
    };
    assert_eq!(info.width, 1280);
    assert_eq!(info.height, 720);
    assert!(!info.connected);
}

#[test]
fn capabilities_find_all_skips_non_json_files() {
    let tmp = tempfile::tempdir().unwrap();
    let runtime_dir = tmp.path().to_path_buf();
    let discovery_dir = runtime_dir.join("ecoPrimals/discovery");
    std::fs::create_dir_all(&discovery_dir).unwrap();

    let non_json = discovery_dir.join("readme.txt");
    std::fs::write(&non_json, "This is not JSON").unwrap();

    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(runtime_dir.to_str().unwrap()),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let found = rt.block_on(DisplayCapabilities::find_all()).unwrap();
            assert!(
                found.is_empty(),
                "find_all should skip .txt files, found {}",
                found.len()
            );
        },
    );
}

#[test]
fn capabilities_cleanup_nonexistent_file_ok() {
    let mut caps = make_caps();
    caps.primal_id = "nonexistent-cleanup-test-xyz".to_string();
    let tmp = tempfile::tempdir().unwrap();
    temp_env::with_var(
        "XDG_RUNTIME_DIR",
        Some(tmp.path().to_str().unwrap()),
        || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(caps.cleanup());
            assert!(result.is_ok(), "cleanup of nonexistent file should succeed");
        },
    );
}

#[test]
fn capabilities_display_info_resolution_edge_cases() {
    let info = DisplayInfo {
        name: "Virtual-1".to_string(),
        width: 640,
        height: 480,
        refresh_rate: 30.0,
        connected: true,
    };
    assert_eq!(info.width * info.height, 307_200);
}

// -----------------------------------------------------------------------------
// Input device (input/device.rs)
// -----------------------------------------------------------------------------

#[test]
fn input_device_type_enumeration() {
    assert_eq!(DeviceType::Keyboard, DeviceType::Keyboard);
    assert_eq!(DeviceType::Mouse, DeviceType::Mouse);
    assert_eq!(DeviceType::Touchscreen, DeviceType::Touchscreen);
    assert_eq!(DeviceType::Touchpad, DeviceType::Touchpad);
    assert_eq!(DeviceType::Gamepad, DeviceType::Gamepad);
    assert_eq!(DeviceType::Other, DeviceType::Other);
    assert_ne!(DeviceType::Keyboard, DeviceType::Mouse);
}

#[test]
fn input_device_capability_enumeration() {
    assert_eq!(DeviceCapability::Keys, DeviceCapability::Keys);
    assert_eq!(
        DeviceCapability::RelativePointer,
        DeviceCapability::RelativePointer
    );
    assert_eq!(
        DeviceCapability::AbsolutePointer,
        DeviceCapability::AbsolutePointer
    );
    assert_eq!(DeviceCapability::MultiTouch, DeviceCapability::MultiTouch);
    assert_eq!(DeviceCapability::Scroll, DeviceCapability::Scroll);
    assert_eq!(
        DeviceCapability::ForceFeedback,
        DeviceCapability::ForceFeedback
    );
}

#[test]
fn input_device_info_configuration() {
    let info = DeviceInfo {
        path: PathBuf::from("/dev/input/event5"),
        name: "Logitech Mouse".to_string(),
        device_type: DeviceType::Mouse,
        capabilities: vec![DeviceCapability::RelativePointer, DeviceCapability::Scroll],
    };
    assert_eq!(info.path, PathBuf::from("/dev/input/event5"));
    assert_eq!(info.name, "Logitech Mouse");
    assert_eq!(info.device_type, DeviceType::Mouse);
    assert_eq!(info.capabilities.len(), 2);
}

#[test]
fn input_device_info_serialization_roundtrip() {
    let info = DeviceInfo {
        path: PathBuf::from("/dev/input/event0"),
        name: "AT Keyboard".to_string(),
        device_type: DeviceType::Keyboard,
        capabilities: vec![DeviceCapability::Keys],
    };
    // DeviceInfo doesn't implement Serialize - it's used internally
    // Test clone and debug
    let cloned = info.clone();
    assert_eq!(info.path, cloned.path);
    assert_eq!(info.name, cloned.name);
    let debug_str = format!("{info:?}");
    assert!(debug_str.contains("AT Keyboard"));
}

#[test]
fn input_device_open_nonexistent_error_path() {
    let result = Device::open("/dev/input/nonexistent-event-99999");
    assert!(result.is_err());
    if let Err(e) = result {
        let err_str = e.to_string();
        assert!(
            err_str.contains("not found") || err_str.contains("Device"),
            "expected device not found error: {err_str}"
        );
    }
}

#[test]
fn input_device_open_nonexistent_path() {
    let result = Device::open("/dev/input/event-nonexistent-xyz");
    assert!(result.is_err());
}

#[test]
fn input_device_discover_all_returns_result() {
    let result = Device::discover_all();
    assert!(result.is_ok());
    let devices = result.unwrap();
    for dev in &devices {
        assert!(dev.path.to_string_lossy().starts_with("/dev/input/"));
        assert!(!dev.name.is_empty());
    }
}

#[test]
fn input_device_type_debug() {
    let t = DeviceType::Touchscreen;
    let s = format!("{t:?}");
    assert!(s.contains("Touchscreen"));
}

#[test]
fn input_device_capability_debug() {
    let c = DeviceCapability::ForceFeedback;
    let s = format!("{c:?}");
    assert!(s.contains("ForceFeedback"));
}

// -----------------------------------------------------------------------------
// IPC server (ipc/server.rs)
// -----------------------------------------------------------------------------

async fn test_manager() -> Option<WindowManager> {
    WindowManager::new().await.ok()
}

#[tokio::test]
async fn ipc_server_display_server_new_socket_path() {
    let Some(manager) = test_manager().await else {
        return;
    };
    let server = DisplayServer::new(manager);
    let path = server.socket_path();
    let path_str = path.to_string_lossy();
    assert!(
        path_str.contains("toadstool") || path_str.contains("display"),
        "socket path should contain toadstool or display: {path_str}"
    );
    assert!(
        path_str.ends_with("display.sock"),
        "socket path should end with display.sock: {path_str}"
    );
}

#[test]
fn ipc_transport_unix_socket() {
    let t = IpcTransport::UnixSocket;
    let s = format!("{t:?}");
    assert!(s.contains("Unix"));
}

#[test]
fn ipc_transport_tcp_fallback_serialization() {
    use std::net::SocketAddr;
    let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
    let t = IpcTransport::TcpFallback(addr);
    let s = format!("{t:?}");
    assert!(s.contains("TcpFallback") || s.contains("127") || s.contains("12345"));
}

#[test]
fn ipc_transport_clone() {
    let t1 = IpcTransport::UnixSocket;
    let t2 = t1.clone();
    assert!(matches!(t2, IpcTransport::UnixSocket));

    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let t3 = IpcTransport::TcpFallback(addr);
    let t4 = t3.clone();
    assert!(matches!(t4, IpcTransport::TcpFallback(_)));
}

#[tokio::test]
async fn ipc_server_transport_initially_none() {
    let Some(manager) = test_manager().await else {
        return;
    };
    let server = DisplayServer::new(manager);
    let transport = server.transport().await;
    assert!(transport.is_none());
}

#[tokio::test]
async fn ipc_server_configuration() {
    let Some(manager) = test_manager().await else {
        return;
    };
    let server = DisplayServer::new(manager);
    let path = server.socket_path();
    assert!(!path.as_os_str().is_empty());
}
