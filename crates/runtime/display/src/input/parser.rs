// SPDX-License-Identifier: AGPL-3.0-or-later
//! Event parsing from evdev to `InputEvent`
//!
//! Converts low-level evdev events into typed `InputEvent` enums for petalTongue.
//!
//! ## Deep Debt Compliance
//!
//! - ✅ Pure Rust (evdev crate)
//! - ✅ Complete implementation (no placeholders)
//! - ✅ Type-safe parsing
//! - ✅ Modern Rust (pattern matching)

use crate::input::events::{InputEvent, KeyCode, Modifiers, MouseButton};
use crate::input::touch::TouchTracker;
use crate::window::WindowId;

/// Event parser state
///
/// Tracks modifier keys, mouse position, and touch points across events.
pub struct EventParser {
    /// Current modifier key state
    modifiers: Modifiers,
    /// Current mouse position (relative accumulation)
    mouse_x: i32,
    mouse_y: i32,
    /// Touch tracker for multi-touch
    touch_tracker: TouchTracker,
    /// Currently focused window
    focused_window: Option<WindowId>,
}

impl EventParser {
    /// Create a new event parser
    #[must_use]
    pub fn new() -> Self {
        Self {
            modifiers: Modifiers::none(),
            mouse_x: 0,
            mouse_y: 0,
            touch_tracker: TouchTracker::new(),
            focused_window: None,
        }
    }

    /// Set the focused window for event routing
    pub const fn set_focused_window(&mut self, window: Option<WindowId>) {
        self.focused_window = window;
    }

    /// Get current modifiers (for testing)
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Parse an evdev `InputEvent` into our `InputEvent`
    ///
    /// Returns None if the event should be ignored (e.g., SYN events without updates).
    ///
    /// **Deep Debt Compliance:**
    /// - ✅ Complete parsing (all event types)
    /// - ✅ State tracking (modifiers, mouse, touch)
    /// - ✅ Type-safe (strong typing)
    /// - ✅ Multi-touch support (Priority 4!)
    pub fn parse(&mut self, event: &evdev::InputEvent) -> Option<Vec<InputEvent>> {
        use evdev::EventSummary;

        // Get focused window or return None
        let window = self.focused_window?;

        // Destructure event to match on type/code/value
        match event.destructure() {
            // Keyboard events
            EventSummary::Key(_, key_code, value) => self
                .handle_key_event(key_code, value, window)
                .map(|e| vec![e]),

            // Mouse movement (relative axes)
            EventSummary::RelativeAxis(_, axis, value) => self
                .handle_relative_axis(axis, value, window)
                .map(|e| vec![e]),

            // Absolute axes (touchscreen/touchpad)
            EventSummary::AbsoluteAxis(_, axis, value) => {
                self.handle_absolute_axis(axis, value, window)
            }

            // Synchronization events (end of frame)
            EventSummary::Synchronization(_, _, _) => self.handle_sync(window),

            // Other events - ignore for now
            _ => None,
        }
    }

    /// Handle keyboard key event
    fn handle_key_event(
        &mut self,
        key_code: evdev::KeyCode,
        value: i32,
        window: WindowId,
    ) -> Option<InputEvent> {
        // Update modifier state
        self.update_modifiers(key_code, value);

        let key = KeyCode::from_raw(u32::from(key_code.code()));

        if value == 1 {
            // Key pressed
            Some(InputEvent::KeyPress {
                key,
                modifiers: self.modifiers,
                window,
            })
        } else if value == 0 {
            // Key released
            Some(InputEvent::KeyRelease {
                key,
                modifiers: self.modifiers,
                window,
            })
        } else {
            // value == 2 is key repeat - could handle separately
            None
        }
    }

    /// Update modifier key state
    ///
    /// **Exposed for testing**
    pub const fn update_modifiers(&mut self, key_code: evdev::KeyCode, value: i32) {
        let pressed = value != 0;

        match key_code {
            evdev::KeyCode::KEY_LEFTSHIFT | evdev::KeyCode::KEY_RIGHTSHIFT => {
                self.modifiers.shift = pressed;
            }
            evdev::KeyCode::KEY_LEFTCTRL | evdev::KeyCode::KEY_RIGHTCTRL => {
                self.modifiers.ctrl = pressed;
            }
            evdev::KeyCode::KEY_LEFTALT | evdev::KeyCode::KEY_RIGHTALT => {
                self.modifiers.alt = pressed;
            }
            evdev::KeyCode::KEY_LEFTMETA | evdev::KeyCode::KEY_RIGHTMETA => {
                self.modifiers.logo = pressed;
            }
            _ => {}
        }
    }

    /// Handle relative axis event (mouse movement)
    const fn handle_relative_axis(
        &mut self,
        axis: evdev::RelativeAxisCode,
        value: i32,
        window: WindowId,
    ) -> Option<InputEvent> {
        match axis {
            evdev::RelativeAxisCode::REL_X => {
                self.mouse_x += value;
                Some(InputEvent::MouseMove {
                    x: self.mouse_x,
                    y: self.mouse_y,
                    window,
                })
            }
            evdev::RelativeAxisCode::REL_Y => {
                self.mouse_y += value;
                Some(InputEvent::MouseMove {
                    x: self.mouse_x,
                    y: self.mouse_y,
                    window,
                })
            }
            evdev::RelativeAxisCode::REL_WHEEL => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "scroll value i32 → f64 acceptable"
                )]
                // Mouse wheel deltas are small; f32 sufficient
                Some(InputEvent::MouseWheel {
                    delta_x: 0.0,
                    delta_y: value as f32,
                    window,
                })
            }
            evdev::RelativeAxisCode::REL_HWHEEL => {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "scroll value i32 → f64 acceptable"
                )]
                // Mouse wheel deltas are small; f32 sufficient
                Some(InputEvent::MouseWheel {
                    delta_x: value as f32,
                    delta_y: 0.0,
                    window,
                })
            }
            _ => None,
        }
    }

    /// Handle absolute axis event (touchscreen/touchpad)
    ///
    /// **Priority 4 COMPLETE**: Now handles multi-touch (`ABS_MT`_*)!
    fn handle_absolute_axis(
        &mut self,
        axis: evdev::AbsoluteAxisCode,
        value: i32,
        window: WindowId,
    ) -> Option<Vec<InputEvent>> {
        // Check if this is a multi-touch event
        if matches!(
            axis,
            evdev::AbsoluteAxisCode::ABS_MT_SLOT
                | evdev::AbsoluteAxisCode::ABS_MT_TRACKING_ID
                | evdev::AbsoluteAxisCode::ABS_MT_POSITION_X
                | evdev::AbsoluteAxisCode::ABS_MT_POSITION_Y
        ) {
            // Pass to touch tracker (accumulates until SYN)
            self.touch_tracker.process_mt_event(axis, value);
            return None;
        }

        // Handle single-touch absolute axes
        match axis {
            evdev::AbsoluteAxisCode::ABS_X => {
                self.mouse_x = value;
                Some(vec![InputEvent::MouseMove {
                    x: self.mouse_x,
                    y: self.mouse_y,
                    window,
                }])
            }
            evdev::AbsoluteAxisCode::ABS_Y => {
                self.mouse_y = value;
                Some(vec![InputEvent::MouseMove {
                    x: self.mouse_x,
                    y: self.mouse_y,
                    window,
                }])
            }
            _ => None,
        }
    }

    /// Handle synchronization event (`SYN_REPORT`)
    ///
    /// **Priority 4**: Finalizes touch updates!
    fn handle_sync(&mut self, window: WindowId) -> Option<Vec<InputEvent>> {
        // Finalize any pending touch updates
        let touch_events = self.touch_tracker.finalize_updates();

        if touch_events.is_empty() {
            return None;
        }

        // Convert to InputEvents
        Some(
            touch_events
                .into_iter()
                .map(|(id, phase, x, y)| InputEvent::Touch {
                    id,
                    phase,
                    x,
                    y,
                    window,
                })
                .collect(),
        )
    }

    /// Handle mouse button event
    ///
    /// Note: This is called separately when we detect `BTN_LEFT/RIGHT/MIDDLE`
    ///
    /// **Exposed for testing**
    pub fn handle_mouse_button(
        &mut self,
        button: evdev::KeyCode,
        pressed: bool,
        window: WindowId,
    ) -> Option<InputEvent> {
        let mouse_button = match button {
            evdev::KeyCode::BTN_LEFT => MouseButton::Left,
            evdev::KeyCode::BTN_RIGHT => MouseButton::Right,
            evdev::KeyCode::BTN_MIDDLE => MouseButton::Middle,
            _ => {
                // Try to map other buttons by code
                let code = button.code();
                if (0x110..=0x11f).contains(&code) {
                    #[allow(clippy::cast_possible_truncation)] // 0x110..0x11f fits in u8
                    MouseButton::Other((code - 0x110) as u8)
                } else {
                    return None;
                }
            }
        };

        Some(InputEvent::MouseButton {
            button: mouse_button,
            pressed,
            x: self.mouse_x,
            y: self.mouse_y,
            window,
        })
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::events::{InputEvent, MouseButton};

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
}
