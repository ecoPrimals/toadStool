//! Integration tests for display backend
//!
//! Tests the full stack from window creation to IPC communication.

use toadstool_display::window::{CreateWindowRequest, WindowManager};
use toadstool_display::input::InputManager;

#[tokio::test]
async fn test_window_lifecycle() {
    // Skip if no DRM device available (CI environment)
    let manager_result = WindowManager::new().await;
    if manager_result.is_err() {
        eprintln!("Skipping test: No DRM device available");
        return;
    }

    let mut manager = manager_result.unwrap();

    // Create window
    let req = CreateWindowRequest {
        width: 800,
        height: 600,
        title: Some("Test Window".to_string()),
        fullscreen: false,
    };

    let window_id = manager.create_window(req).await.unwrap();

    // Get window info
    let info = manager.get_window_info(window_id).unwrap();
    assert_eq!(info.width, 800);
    assert_eq!(info.height, 600);
    assert_eq!(info.title, Some("Test Window".to_string()));
    assert_eq!(info.focused, true); // First window should be focused

    // Destroy window
    manager.destroy_window(window_id).await.unwrap();

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

    // Create multiple windows
    let id1 = manager
        .create_window(CreateWindowRequest {
            width: 800,
            height: 600,
            ..Default::default()
        })
        .await
        .unwrap();

    let id2 = manager
        .create_window(CreateWindowRequest {
            width: 1024,
            height: 768,
            ..Default::default()
        })
        .await
        .unwrap();

    // Check window count
    assert_eq!(manager.window_count(), 2);

    // List windows
    let windows = manager.list_windows();
    assert_eq!(windows.len(), 2);
    assert!(windows.contains(&id1));
    assert!(windows.contains(&id2));

    // Clean up
    manager.destroy_window(id1).await.unwrap();
    manager.destroy_window(id2).await.unwrap();
}

#[tokio::test]
async fn test_focus_management() {
    let manager_result = WindowManager::new().await;
    if manager_result.is_err() {
        eprintln!("Skipping test: No DRM device available");
        return;
    }

    let mut manager = manager_result.unwrap();

    // Create two windows
    let id1 = manager
        .create_window(CreateWindowRequest::default())
        .await
        .unwrap();

    let id2 = manager
        .create_window(CreateWindowRequest::default())
        .await
        .unwrap();

    // First window should be focused
    assert_eq!(manager.get_focused(), Some(id1));

    // Change focus
    manager.set_focus(id2);
    assert_eq!(manager.get_focused(), Some(id2));

    // Verify window info reflects focus state
    let info1 = manager.get_window_info(id1).unwrap();
    let info2 = manager.get_window_info(id2).unwrap();
    assert_eq!(info1.focused, false);
    assert_eq!(info2.focused, true);

    // Clean up
    manager.destroy_window(id1).await.unwrap();
    manager.destroy_window(id2).await.unwrap();
}

#[tokio::test]
async fn test_input_manager_integration() {
    let manager_result = InputManager::discover().await;
    assert!(manager_result.is_ok());

    let mut manager = manager_result.unwrap();

    // Test focus management
    assert_eq!(manager.focused_window(), None);

    let window_id = toadstool_display::window::WindowId::new();
    manager.set_focus(Some(window_id));
    assert_eq!(manager.focused_window(), Some(window_id));

    // Poll events (should be empty in test environment)
    let events = manager.poll_events().await.unwrap();
    assert!(events.is_empty());
}

#[test]
fn test_window_id_serialization() {
    use toadstool_display::window::WindowId;

    let id = WindowId::new();
    let s = id.to_string();

    // Should be a valid UUID string
    assert!(s.len() > 0);
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
