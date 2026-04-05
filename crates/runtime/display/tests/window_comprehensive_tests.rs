// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for window manager functionality

use toadstool_display::window::{CreateWindowRequest, Size, WindowId, WindowManager};

/// Try to create a window, returning None if DRM ioctl unavailable (headless/CI).
fn try_create(manager: &mut WindowManager, req: CreateWindowRequest) -> Option<WindowId> {
    match manager.create_window(req) {
        Ok(id) => Some(id),
        Err(e) => {
            eprintln!("Skipping: DRM buffer unavailable: {e}");
            None
        }
    }
}

#[tokio::test]
async fn test_window_manager_new() {
    let result = WindowManager::new().await;
    if result.is_err() {
        eprintln!("Skipping: No DRM device");
        return;
    }

    let manager = result.unwrap();
    assert_eq!(manager.window_count(), 0);
    assert_eq!(manager.get_focused(), None);
    assert!(manager.list_windows().is_empty());
}

#[tokio::test]
#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
async fn test_window_info_after_creation() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let req = CreateWindowRequest {
        width: 1024,
        height: 768,
        title: Some("Test".to_string()),
        fullscreen: false,
    };

    let Some(id) = try_create(&mut manager, req) else {
        return;
    };
    let info = manager.get_window_info(id).unwrap();

    assert_eq!(info.width, 1024);
    assert_eq!(info.height, 768);
    assert_eq!(info.scale_factor, 1.0);
    assert_eq!(info.title, Some("Test".to_string()));
    assert!(info.focused);
}

#[tokio::test]
async fn test_window_resize() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let Some(id) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };

    manager
        .resize_window(
            id,
            Size {
                width: 800,
                height: 600,
            },
        )
        .unwrap();

    let info = manager.get_window_info(id).unwrap();
    assert_eq!(info.width, 800);
    assert_eq!(info.height, 600);
}

#[tokio::test]
async fn test_window_focus_changes() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let Some(id1) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };
    let Some(id2) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };
    let Some(id3) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };

    assert_eq!(manager.get_focused(), Some(id1));

    manager.set_focus(id2);
    assert_eq!(manager.get_focused(), Some(id2));
    assert!(manager.get_window_info(id2).unwrap().focused);
    assert!(!manager.get_window_info(id1).unwrap().focused);

    manager.set_focus(id3);
    assert_eq!(manager.get_focused(), Some(id3));
    assert!(!manager.get_window_info(id2).unwrap().focused);
}

#[tokio::test]
async fn test_window_list_after_operations() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let Some(id1) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };
    let Some(id2) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };
    let Some(id3) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };

    assert_eq!(manager.window_count(), 3);
    let windows = manager.list_windows();
    assert_eq!(windows.len(), 3);
    assert!(windows.contains(&id1));
    assert!(windows.contains(&id2));
    assert!(windows.contains(&id3));

    manager.destroy_window(id2).unwrap();
    assert_eq!(manager.window_count(), 2);
    let windows = manager.list_windows();
    assert!(!windows.contains(&id2));
}

#[tokio::test]
async fn test_window_destroy_focused() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let Some(id1) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };
    let Some(id2) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };

    manager.set_focus(id1);
    assert_eq!(manager.get_focused(), Some(id1));

    manager.destroy_window(id1).unwrap();
    assert_eq!(manager.get_focused(), Some(id2));
}

#[tokio::test]
async fn test_window_destroy_last() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };

    let Some(id) = try_create(&mut manager, CreateWindowRequest::default()) else {
        return;
    };

    manager.destroy_window(id).unwrap();
    assert_eq!(manager.window_count(), 0);
    assert_eq!(manager.get_focused(), None);
}

#[tokio::test]
async fn test_window_not_found_errors() {
    let Ok(mut manager) = WindowManager::new().await else {
        eprintln!("Skipping: No DRM device");
        return;
    };
    let fake_id = WindowId::new();

    assert!(manager.get_window_info(fake_id).is_err());
    assert!(manager.destroy_window(fake_id).is_err());
    assert!(
        manager
            .resize_window(
                fake_id,
                Size {
                    width: 100,
                    height: 100
                }
            )
            .is_err()
    );
}

#[test]
fn test_window_id_parse_errors() {
    assert!(WindowId::from_string("not-a-uuid").is_err());
    assert!(WindowId::from_string("").is_err());
    assert!(WindowId::from_string("123").is_err());
}

#[test]
fn test_create_request_variations() {
    let req1 = CreateWindowRequest {
        width: 3840,
        height: 2160,
        title: Some("4K Window".to_string()),
        fullscreen: true,
    };
    assert_eq!(req1.width, 3840);
    assert!(req1.fullscreen);

    let req2 = CreateWindowRequest {
        width: 640,
        height: 480,
        title: None,
        fullscreen: false,
    };
    assert_eq!(req2.title, None);
    assert!(!req2.fullscreen);
}
