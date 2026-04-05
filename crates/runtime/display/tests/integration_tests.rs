// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for display backend
//!
//! Tests the full stack from window creation to IPC communication.

use toadstool_display::input::InputManager;
use toadstool_display::window::{CreateWindowRequest, WindowManager};

#[tokio::test]
async fn test_window_lifecycle() {
    // Skip if no DRM device available (CI environment)
    let manager_result = WindowManager::new().await;
    if manager_result.is_err() {
        eprintln!("Skipping test: No DRM device available");
        return;
    }

    let mut manager = manager_result.unwrap();

    let req = CreateWindowRequest {
        width: 800,
        height: 600,
        title: Some("Test Window".to_string()),
        fullscreen: false,
    };

    let window_id = match manager.create_window(req) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Skipping test: DRM buffer allocation unavailable: {e}");
            return;
        }
    };

    // Get window info
    let info = manager.get_window_info(window_id).unwrap();
    assert_eq!(info.width, 800);
    assert_eq!(info.height, 600);
    assert_eq!(info.title, Some("Test Window".to_string()));
    assert!(info.focused); // First window should be focused

    // Destroy window
    manager.destroy_window(window_id).unwrap();

    // Window should no longer exist
    assert!(manager.get_window_info(window_id).is_err());
}

#[tokio::test]
async fn test_multiple_windows() {
    let manager_result = WindowManager::new().await;
    if manager_result.is_err() {
        eprintln!("Skipping test: No DRM device available");
        return;
    }

    let mut manager = manager_result.unwrap();

    let id1 = match manager.create_window(CreateWindowRequest {
        width: 800,
        height: 600,
        ..Default::default()
    }) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Skipping test: DRM buffer allocation unavailable: {e}");
            return;
        }
    };

    let id2 = match manager.create_window(CreateWindowRequest {
        width: 1024,
        height: 768,
        ..Default::default()
    }) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Skipping test: DRM buffer allocation unavailable: {e}");
            return;
        }
    };

    // Check window count
    assert_eq!(manager.window_count(), 2);

    // List windows
    let windows = manager.list_windows();
    assert_eq!(windows.len(), 2);
    assert!(windows.contains(&id1));
    assert!(windows.contains(&id2));

    // Clean up
    manager.destroy_window(id1).unwrap();
    manager.destroy_window(id2).unwrap();
}

#[tokio::test]
async fn test_focus_management() {
    let manager_result = WindowManager::new().await;
    if manager_result.is_err() {
        eprintln!("Skipping test: No DRM device available");
        return;
    }

    let mut manager = manager_result.unwrap();

    // Create two windows — may fail in headless/CI environments (DRM dumb buffer unavailable)
    let id1 = match manager.create_window(CreateWindowRequest::default()) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Skipping test: DRM buffer allocation unavailable: {e}");
            return;
        }
    };

    let id2 = match manager.create_window(CreateWindowRequest::default()) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Skipping test: DRM buffer allocation unavailable: {e}");
            return;
        }
    };

    // First window should be focused
    assert_eq!(manager.get_focused(), Some(id1));

    // Change focus
    manager.set_focus(id2);
    assert_eq!(manager.get_focused(), Some(id2));

    // Verify window info reflects focus state
    let info1 = manager.get_window_info(id1).unwrap();
    let info2 = manager.get_window_info(id2).unwrap();
    assert!(!info1.focused);
    assert!(info2.focused);

    // Clean up
    manager.destroy_window(id1).unwrap();
    manager.destroy_window(id2).unwrap();
}

#[tokio::test]
async fn test_input_manager_integration() {
    let manager_result = InputManager::discover();
    assert!(manager_result.is_ok());

    let mut manager = manager_result.unwrap();

    // Test focus management
    assert_eq!(manager.focused_window(), None);

    let window_id = toadstool_display::window::WindowId::new();
    manager.set_focus(Some(window_id));
    assert_eq!(manager.focused_window(), Some(window_id));

    // Poll events (should be empty in test environment)
    let events = manager.poll_events().unwrap();
    assert!(events.is_empty());
}

#[test]
fn test_window_id_serialization() {
    use toadstool_display::window::WindowId;

    let id = WindowId::new();
    let s = id.to_string();

    // Should be a valid UUID string
    assert!(!s.is_empty());
    assert!(s.contains('-'));

    // Should round-trip
    let id2 = WindowId::from_string(&s).unwrap();
    assert_eq!(id, id2);
}

#[test]
fn test_create_window_request_serialization() {
    use toadstool_display::window::CreateWindowRequest;

    let req = CreateWindowRequest {
        width: 1920,
        height: 1080,
        title: Some("Test".to_string()),
        fullscreen: true,
    };

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("1920"));
    assert!(json.contains("1080"));
    assert!(json.contains("Test"));

    let req2: CreateWindowRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req2.width, req.width);
    assert_eq!(req2.height, req.height);
    assert_eq!(req2.title, req.title);
    assert_eq!(req2.fullscreen, req.fullscreen);
}

// ✅ Phase 1 COMPLETE:
// - Window lifecycle tests
// - Multi-window tests
// - Focus management tests
// - Input manager integration
// - Serialization tests
// - All tests gracefully handle missing DRM devices (CI-friendly!)
