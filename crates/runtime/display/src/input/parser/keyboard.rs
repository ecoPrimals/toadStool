// SPDX-License-Identifier: AGPL-3.0-only
//! Keyboard key events and modifier tracking for [`super::EventParser`].

use crate::input::events::{InputEvent, KeyCode};
use crate::window::WindowId;

use super::EventParser;

impl EventParser {
    /// Handle keyboard key event
    pub(super) fn handle_key_event(
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
}
