// SPDX-License-Identifier: AGPL-3.0-only

use super::EventParser;
use crate::input::events::{InputEvent, Modifiers, MouseButton};
use crate::window::WindowId;

#[test]
fn test_event_parser_creation() {
    let parser = EventParser::new();
    assert_eq!(parser.mouse_x, 0);
    assert_eq!(parser.mouse_y, 0);
    assert!(!parser.modifiers.any());
}

#[test]
fn test_modifier_tracking() {
    let mut parser = EventParser::new();

    // Press shift
    parser.update_modifiers(evdev::KeyCode::KEY_LEFTSHIFT, 1);
    assert!(parser.modifiers.shift);
    assert!(!parser.modifiers.ctrl);

    // Press ctrl
    parser.update_modifiers(evdev::KeyCode::KEY_LEFTCTRL, 1);
    assert!(parser.modifiers.shift);
    assert!(parser.modifiers.ctrl);

    // Release shift
    parser.update_modifiers(evdev::KeyCode::KEY_LEFTSHIFT, 0);
    assert!(!parser.modifiers.shift);
    assert!(parser.modifiers.ctrl);
}

#[test]
fn test_mouse_position_tracking() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    // Move mouse
    parser.mouse_x += 10;
    parser.mouse_y += 20;

    assert_eq!(parser.mouse_x, 10);
    assert_eq!(parser.mouse_y, 20);
}

#[test]
fn test_set_focused_window_and_modifiers() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    assert_eq!(parser.modifiers(), Modifiers::none());
    parser.set_focused_window(Some(window));
    assert!(parser.focused_window.is_some());

    parser.set_focused_window(None);
    assert!(parser.focused_window.is_none());
}

#[test]
fn test_handle_key_event_press() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_key_event(evdev::KeyCode::KEY_A, 1, window);
    assert!(
        matches!(result, Some(InputEvent::KeyPress { key, .. }) if key.raw() == u32::from(evdev::KeyCode::KEY_A.code()))
    );
}

#[test]
fn test_handle_key_event_release() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_key_event(evdev::KeyCode::KEY_A, 0, window);
    assert!(
        matches!(result, Some(InputEvent::KeyRelease { key, .. }) if key.raw() == u32::from(evdev::KeyCode::KEY_A.code()))
    );
}

#[test]
fn test_handle_key_event_repeat_returns_none() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_key_event(evdev::KeyCode::KEY_A, 2, window);
    assert!(result.is_none());
}

#[test]
fn test_handle_relative_axis_rel_x() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_relative_axis(evdev::RelativeAxisCode::REL_X, 10, window);
    match &result {
        Some(InputEvent::MouseMove { x, y, .. }) => {
            assert_eq!(*x, 10);
            assert_eq!(*y, 0);
        }
        _ => panic!("expected MouseMove, got {result:?}"),
    }
    assert_eq!(parser.mouse_x, 10);
}

#[test]
fn test_handle_relative_axis_rel_y() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_relative_axis(evdev::RelativeAxisCode::REL_Y, 20, window);
    match &result {
        Some(InputEvent::MouseMove { x, y, .. }) => {
            assert_eq!(*x, 0);
            assert_eq!(*y, 20);
        }
        _ => panic!("expected MouseMove, got {result:?}"),
    }
    assert_eq!(parser.mouse_y, 20);
}

#[test]
fn test_handle_relative_axis_rel_wheel() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_relative_axis(evdev::RelativeAxisCode::REL_WHEEL, 1, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseWheel {
            delta_x: 0.0,
            delta_y: 1.0,
            ..
        })
    ));
}

#[test]
fn test_handle_relative_axis_rel_hwheel() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_relative_axis(evdev::RelativeAxisCode::REL_HWHEEL, -1, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseWheel {
            delta_x: -1.0,
            delta_y: 0.0,
            ..
        })
    ));
}

#[test]
fn test_handle_absolute_axis_abs_x() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_absolute_axis(evdev::AbsoluteAxisCode::ABS_X, 100, window);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(
            events[0],
            InputEvent::MouseMove { x: 100, y: 0, .. }
        ));
    }
    assert_eq!(parser.mouse_x, 100);
}

#[test]
fn test_handle_absolute_axis_abs_y() {
    let mut parser = EventParser::new();
    parser.mouse_x = 50;
    let window = WindowId::new();

    let result = parser.handle_absolute_axis(evdev::AbsoluteAxisCode::ABS_Y, 200, window);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(
            events[0],
            InputEvent::MouseMove { x: 50, y: 200, .. }
        ));
    }
    assert_eq!(parser.mouse_y, 200);
}

#[test]
fn test_handle_absolute_axis_mt_returns_none() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result =
        parser.handle_absolute_axis(evdev::AbsoluteAxisCode::ABS_MT_POSITION_X, 100, window);
    assert!(result.is_none());
}

#[test]
fn test_handle_mouse_button_left() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_mouse_button(evdev::KeyCode::BTN_LEFT, true, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseButton {
            button: MouseButton::Left,
            pressed: true,
            ..
        })
    ));
}

#[test]
fn test_handle_mouse_button_right() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_mouse_button(evdev::KeyCode::BTN_RIGHT, false, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseButton {
            button: MouseButton::Right,
            pressed: false,
            ..
        })
    ));
}

#[test]
fn test_handle_mouse_button_middle() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_mouse_button(evdev::KeyCode::BTN_MIDDLE, true, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseButton {
            button: MouseButton::Middle,
            ..
        })
    ));
}

#[test]
fn test_handle_mouse_button_other() {
    let mut parser = EventParser::new();
    parser.mouse_x = 10;
    parser.mouse_y = 20;
    let window = WindowId::new();

    // BTN_SIDE (0x113) maps to MouseButton::Other(3) since (0x113 - 0x110) = 3
    let result = parser.handle_mouse_button(evdev::KeyCode::BTN_SIDE, true, window);
    assert!(matches!(
        result,
        Some(InputEvent::MouseButton {
            button: MouseButton::Other(3),
            pressed: true,
            x: 10,
            y: 20,
            ..
        })
    ));
}

#[test]
fn test_handle_mouse_button_invalid_returns_none() {
    let mut parser = EventParser::new();
    let window = WindowId::new();

    let result = parser.handle_mouse_button(evdev::KeyCode::KEY_A, true, window);
    assert!(result.is_none());
}

#[test]
fn test_parse_no_focused_window_returns_none() {
    let mut parser = EventParser::new();
    assert!(parser.focused_window.is_none());

    let event = evdev::InputEvent::new(1, evdev::KeyCode::KEY_A.code(), 1);
    let result = parser.parse(&event);
    assert!(result.is_none());
}

#[test]
fn test_parse_key_press_with_focused_window() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    let event = evdev::InputEvent::new(1, evdev::KeyCode::KEY_A.code(), 1);
    let result = parser.parse(&event);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(events[0], InputEvent::KeyPress { .. }));
    }
}

#[test]
fn test_parse_sync_no_touch_pending_returns_none() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    let event = evdev::InputEvent::new(0, 0, 0);
    let result = parser.parse(&event);
    assert!(result.is_none());
}

#[test]
fn test_parse_relative_axis_rel_x() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    let event = evdev::InputEvent::new(2, 0, 15); // EV_REL, REL_X
    let result = parser.parse(&event);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(events[0], InputEvent::MouseMove { x: 15, .. }));
    }
}

#[test]
fn test_parse_relative_axis_rel_wheel() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    let event = evdev::InputEvent::new(2, 8, 1); // EV_REL, REL_WHEEL
    let result = parser.parse(&event);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(
            events[0],
            InputEvent::MouseWheel { delta_y: 1.0, .. }
        ));
    }
}

#[test]
fn test_parse_absolute_axis_abs_x() {
    let mut parser = EventParser::new();
    parser.set_focused_window(Some(WindowId::new()));

    let event = evdev::InputEvent::new(3, 0, 320); // EV_ABS, ABS_X
    let result = parser.parse(&event);
    assert!(matches!(result, Some(ref v) if v.len() == 1));
    if let Some(events) = result {
        assert!(matches!(events[0], InputEvent::MouseMove { x: 320, .. }));
    }
}

#[test]
fn test_default_impl() {
    let parser = EventParser::default();
    assert_eq!(parser.mouse_x, 0);
    assert_eq!(parser.mouse_y, 0);
}
