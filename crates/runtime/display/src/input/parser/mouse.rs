// SPDX-License-Identifier: AGPL-3.0-only
//! Relative-axis (pointer) and mouse-button handling for [`super::EventParser`].

use crate::input::events::{InputEvent, MouseButton};
use crate::window::WindowId;

use super::EventParser;

impl EventParser {
    /// Handle relative axis event (mouse movement)
    pub(super) const fn handle_relative_axis(
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
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "0x110..0x11f range fits in u8"
                    )]
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
