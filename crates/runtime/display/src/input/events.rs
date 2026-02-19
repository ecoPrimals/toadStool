//! Input event types and parsing
//!
//! Provides typed events for keyboard, mouse, touch input.

#[allow(unused_imports)]
use crate::window::WindowId;

/// Input event
///
/// All events are tagged with the target window for proper routing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum InputEvent {
    /// Keyboard key pressed
    KeyPress {
        /// Key code
        key: KeyCode,
        /// Modifier keys
        modifiers: Modifiers,
        /// Target window
        window: WindowId,
    },

    /// Keyboard key released
    KeyRelease {
        /// Key code
        key: KeyCode,
        /// Modifier keys
        modifiers: Modifiers,
        /// Target window
        window: WindowId,
    },

    /// Mouse moved
    MouseMove {
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Target window
        window: WindowId,
    },

    /// Mouse button event
    MouseButton {
        /// Button
        button: MouseButton,
        /// Pressed or released
        pressed: bool,
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Target window
        window: WindowId,
    },

    /// Mouse wheel scroll
    MouseWheel {
        /// Horizontal delta
        delta_x: f32,
        /// Vertical delta
        delta_y: f32,
        /// Target window
        window: WindowId,
    },

    /// Touch event
    Touch {
        /// Touch ID
        id: u32,
        /// Touch phase
        phase: TouchPhase,
        /// X coordinate
        x: i32,
        /// Y coordinate
        y: i32,
        /// Target window
        window: WindowId,
    },

    /// Window focused
    WindowFocused {
        /// Window that gained focus
        window: WindowId,
    },

    /// Window unfocused
    WindowUnfocused {
        /// Window that lost focus
        window: WindowId,
    },

    /// Window resized
    WindowResized {
        /// Resized window
        window: WindowId,
        /// New width
        width: u32,
        /// New height
        height: u32,
    },

    /// Window closed (user requested)
    WindowClosed {
        /// Window to close
        window: WindowId,
    },
}

/// Keyboard key code
///
/// Represents a physical key on the keyboard.
/// Mapped from Linux input event codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyCode(u32);

impl KeyCode {
    /// Create a new key code
    pub const fn new(code: u32) -> Self {
        Self(code)
    }

    /// Create from raw code
    pub const fn from_raw(code: u32) -> Self {
        Self(code)
    }

    /// Get raw code
    pub const fn raw(self) -> u32 {
        self.0
    }

    // Common key codes (Linux input codes)
    /// Escape key
    pub const ESC: Self = Self(1);
    /// Return/Enter key
    pub const RETURN: Self = Self(28);
    /// Space bar
    pub const SPACE: Self = Self(57);
    /// Left Shift
    pub const LEFT_SHIFT: Self = Self(42);
    /// Right Shift
    pub const RIGHT_SHIFT: Self = Self(54);
    /// Left Control
    pub const LEFT_CTRL: Self = Self(29);
    /// Right Control  
    pub const RIGHT_CTRL: Self = Self(97);
    /// Left Alt
    pub const LEFT_ALT: Self = Self(56);
    /// Right Alt
    pub const RIGHT_ALT: Self = Self(100);
    /// Left Meta/Super/Windows
    pub const LEFT_META: Self = Self(125);
    /// Right Meta/Super/Windows
    pub const RIGHT_META: Self = Self(126);

    // ── Navigation ──────────────────────────────────────────────────────────
    /// Arrow Up
    pub const UP: Self = Self(103);
    /// Arrow Down
    pub const DOWN: Self = Self(108);
    /// Arrow Left
    pub const LEFT: Self = Self(105);
    /// Arrow Right
    pub const RIGHT: Self = Self(106);
    /// Home
    pub const HOME: Self = Self(102);
    /// End
    pub const END: Self = Self(107);
    /// Page Up
    pub const PAGE_UP: Self = Self(104);
    /// Page Down
    pub const PAGE_DOWN: Self = Self(109);
    /// Insert
    pub const INSERT: Self = Self(110);
    /// Delete (forward delete)
    pub const DELETE: Self = Self(111);
    /// Backspace
    pub const BACKSPACE: Self = Self(14);
    /// Tab
    pub const TAB: Self = Self(15);
    /// Caps Lock
    pub const CAPS_LOCK: Self = Self(58);

    // ── Function keys ────────────────────────────────────────────────────────
    pub const F1: Self = Self(59);
    pub const F2: Self = Self(60);
    pub const F3: Self = Self(61);
    pub const F4: Self = Self(62);
    pub const F5: Self = Self(63);
    pub const F6: Self = Self(64);
    pub const F7: Self = Self(65);
    pub const F8: Self = Self(66);
    pub const F9: Self = Self(67);
    pub const F10: Self = Self(68);
    pub const F11: Self = Self(87);
    pub const F12: Self = Self(88);

    // ── Alphanumeric ─────────────────────────────────────────────────────────
    pub const A: Self = Self(30);
    pub const B: Self = Self(48);
    pub const C: Self = Self(46);
    pub const D: Self = Self(32);
    pub const E: Self = Self(18);
    pub const F: Self = Self(33);
    pub const G: Self = Self(34);
    pub const H: Self = Self(35);
    pub const I: Self = Self(23);
    pub const J: Self = Self(36);
    pub const K: Self = Self(37);
    pub const L: Self = Self(38);
    pub const M: Self = Self(50);
    pub const N: Self = Self(49);
    pub const O: Self = Self(24);
    pub const P: Self = Self(25);
    pub const Q: Self = Self(16);
    pub const R: Self = Self(19);
    pub const S: Self = Self(31);
    pub const T: Self = Self(20);
    pub const U: Self = Self(22);
    pub const V: Self = Self(47);
    pub const W: Self = Self(17);
    pub const X: Self = Self(45);
    pub const Y: Self = Self(21);
    pub const Z: Self = Self(44);

    pub const KEY_0: Self = Self(11);
    pub const KEY_1: Self = Self(2);
    pub const KEY_2: Self = Self(3);
    pub const KEY_3: Self = Self(4);
    pub const KEY_4: Self = Self(5);
    pub const KEY_5: Self = Self(6);
    pub const KEY_6: Self = Self(7);
    pub const KEY_7: Self = Self(8);
    pub const KEY_8: Self = Self(9);
    pub const KEY_9: Self = Self(10);
}

/// Keyboard modifiers
///
/// Tracks state of modifier keys (Shift, Ctrl, Alt, Meta/Super).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Modifiers {
    /// Shift key held
    pub shift: bool,
    /// Control key held
    pub ctrl: bool,
    /// Alt key held
    pub alt: bool,
    /// Meta/Super/Windows key held
    pub logo: bool,
}

impl Modifiers {
    /// Create with no modifiers
    pub const fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            logo: false,
        }
    }

    /// Check if any modifiers are active
    pub const fn any(self) -> bool {
        self.shift || self.ctrl || self.alt || self.logo
    }
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button (wheel click)
    Middle,
    /// Additional button 1 (side button)
    Button4,
    /// Additional button 2 (side button)
    Button5,
    /// Other button by index
    Other(u8),
}

/// Touch phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TouchPhase {
    /// Touch started
    Started,
    /// Touch moved
    Moved,
    /// Touch ended
    Ended,
    /// Touch cancelled
    Cancelled,
}

// Phase 2: Event Parsing Implementation
//
// Event parsing from evdev:
//
// ```rust
// fn parse_evdev_event(event: &evdev::InputEvent, window: WindowId) -> Option<InputEvent> {
//     use evdev::InputEventKind;
//
//     match event.kind() {
//         InputEventKind::Key(key) => {
//             let pressed = event.value() != 0;
//             let keycode = KeyCode::from_raw(key.code());
//
//             if pressed {
//                 Some(InputEvent::KeyPress {
//                     key: keycode,
//                     modifiers: get_current_modifiers(),
//                     window,
//                 })
//             } else {
//                 Some(InputEvent::KeyRelease {
//                     key: keycode,
//                     modifiers: get_current_modifiers(),
//                     window,
//                 })
//             }
//         }
//
//         InputEventKind::RelAxis(axis) => {
//             match axis.0 {
//                 evdev::RelativeAxisType::REL_X => {
//                     // Update mouse position, emit MouseMove
//                 }
//                 evdev::RelativeAxisType::REL_WHEEL => {
//                     // Emit MouseWheel
//                 }
//                 _ => None,
//             }
//         }
//
//         // ... other event types
//     }
// }
// ```
//
// Modifier tracking:
// - Keep global state for modifier keys
// - Update on KeyPress/KeyRelease of modifier keys
// - Include in all keyboard events
