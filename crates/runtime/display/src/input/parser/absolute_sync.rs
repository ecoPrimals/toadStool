// SPDX-License-Identifier: AGPL-3.0-only
//! Absolute axes (pointer / single-touch) and `SYN_REPORT` touch finalization for
//! [`super::EventParser`].

use crate::input::events::InputEvent;
use crate::window::WindowId;

use super::EventParser;

impl EventParser {
    /// Handle absolute axis event (touchscreen/touchpad)
    ///
    /// **Priority 4 COMPLETE**: Now handles multi-touch (`ABS_MT`_*)!
    pub(super) fn handle_absolute_axis(
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
    pub(super) fn handle_sync(&mut self, window: WindowId) -> Option<Vec<InputEvent>> {
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
}
