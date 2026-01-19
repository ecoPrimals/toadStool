//! Input device abstraction layer
//!
//! Provides safe wrappers around evdev for keyboard, mouse, touch input.
//!
//! Uses `evdev` crate for 100% Pure Rust implementation.

pub mod device;
pub mod events;

// Re-exports
pub use device::{Device, DeviceInfo, DeviceType, DeviceCapability};
pub use events::{InputEvent, KeyCode, Modifiers, MouseButton, TouchPhase};

#[allow(unused_imports)]
use crate::{DisplayError, Result};
use crate::window::WindowId;

/// Input manager for device enumeration and event handling
///
/// Manages multiple input devices and routes events to appropriate windows.
///
/// ## Architecture
///
/// - Async event handling (tokio)
/// - Per-device event streams
/// - Automatic routing to focused window
/// - Hotplug support (future)
#[allow(dead_code)] // TODO: Phase 0 - Remove when fully implemented
pub struct InputManager {
    devices: Vec<Device>,
    focused_window: Option<WindowId>,
    // TODO: Add event channels
}

impl InputManager {
    /// Discover and initialize input devices
    ///
    /// **Self-knowledge**: Discovers own hardware at runtime!
    /// **No hardcoding**: Agnostic device discovery!
    pub async fn discover() -> Result<Self> {
        tracing::info!("🔍 Initializing input manager...");
        
        // Discover all input devices
        let device_infos = Device::discover_all()?;
        
        tracing::info!("Found {} input devices", device_infos.len());
        for info in &device_infos {
            tracing::debug!("  - {} ({:?})", info.name, info.device_type);
        }
        
        // TODO: Open devices and create event streams
        
        Ok(Self {
            devices: vec![],
            focused_window: None,
        })
    }
    
    /// Set focused window for input routing
    pub fn set_focus(&mut self, window: Option<WindowId>) {
        tracing::debug!("Input focus changed: {:?}", window);
        self.focused_window = window;
    }
    
    /// Get currently focused window
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }
}

// TODO: Phase 0 Implementation:
// - Device enumeration (done in device.rs)
// - Event stream (async with tokio)
// - Event parsing (done in events.rs)
// - Routing to focused window
// - Device hotplug (Phase 1)
