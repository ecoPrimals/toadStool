// SPDX-License-Identifier: AGPL-3.0-or-later
//! Input device management
//!
//! Safe wrappers around evdev input device operations (100% Pure Rust!).

use crate::{DisplayError, Result};
use std::path::{Path, PathBuf};

/// Input device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Keyboard
    Keyboard,
    /// Mouse/Pointer
    Mouse,
    /// Touchscreen
    Touchscreen,
    /// Touchpad
    Touchpad,
    /// Game controller
    Gamepad,
    /// Other/Unknown
    Other,
}

/// Input device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device path (e.g., `/dev/input/event3`)
    pub path: PathBuf,
    /// Device name (from kernel)
    pub name: String,
    /// Device type (detected from capabilities)
    pub device_type: DeviceType,
    /// Supported capabilities
    pub capabilities: Vec<DeviceCapability>,
}

/// Device capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCapability {
    /// Keys (keyboard)
    Keys,
    /// Relative pointer movement (mouse)
    RelativePointer,
    /// Absolute pointer position (touchscreen/touchpad)
    AbsolutePointer,
    /// Multi-touch
    MultiTouch,
    /// Scroll wheel
    Scroll,
    /// Force feedback (rumble/haptics)
    ForceFeedback,
}

/// Input device handle
///
/// Provides safe access to input device events.
///
/// ## Implementation
///
/// Uses `evdev` crate for 100% Pure Rust input handling.
/// NO libevdev! NO libinput! Pure Rust all the way!
///
/// ## Async
///
/// All event reading is async-compatible with tokio.
///
/// ## Safety
///
/// File descriptors are properly managed. No unsafe in public API.
/// The evdev crate handles all low-level details safely.
///
/// ## Example
///
/// ```rust,no_run
/// # use toadstool_display::input::Device;
/// let device = Device::open("/dev/input/event3")?;
/// println!("Opened: {}", device.name());
/// # Ok::<(), toadstool_display::DisplayError>(())
/// ```
#[expect(clippy::struct_field_names)] // device_type and evdev_device are standard evdev terminology
pub struct Device {
    path: PathBuf,
    name: String,
    device_type: DeviceType,
    capabilities: Vec<DeviceCapability>,
    evdev_device: evdev::Device,
}

impl Device {
    /// Open an input device
    ///
    /// # Arguments
    ///
    /// * `path` - Path to input device (e.g., `/dev/input/event3`)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Device doesn't exist
    /// - Permission denied (may need to be in `input` group or use sudo)
    /// - Not an evdev device
    ///
    /// # Deep Debt Compliance
    ///
    /// - ✅ Pure Rust (evdev crate, no FFI!)
    /// - ✅ Runtime discovery (detects type from hardware)
    /// - ✅ Complete implementation (no placeholders!)
    /// - ✅ Zero unsafe (evdev handles all low-level I/O)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::input::Device;
    /// let device = Device::open("/dev/input/event3")?;
    /// println!("Device: {} ({:?})", device.name(), device.device_type());
    /// # Ok::<(), toadstool_display::DisplayError>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if !path.exists() {
            return Err(DisplayError::InputError(format!(
                "Device not found: {}",
                path.display()
            )));
        }

        tracing::debug!("📂 Opening input device: {}", path.display());

        // REAL IMPLEMENTATION: Open actual evdev device! (Pure Rust, no FFI!)
        let evdev_device = evdev::Device::open(&path).map_err(|e| {
            DisplayError::InputError(format!("Failed to open {}: {}", path.display(), e))
        })?;

        // Get device name from hardware
        let name = evdev_device.name().unwrap_or("Unknown Device").to_string();

        // Detect device type from capabilities (runtime self-knowledge!)
        let device_type = Self::detect_type(&evdev_device);

        // Query device capabilities
        let capabilities = Self::detect_capabilities(&evdev_device);

        tracing::info!(
            "✅ Opened: {} ({:?}) at {} [{} capabilities]",
            name,
            device_type,
            path.display(),
            capabilities.len()
        );

        Ok(Self {
            path,
            name,
            device_type,
            capabilities,
            evdev_device,
        })
    }

    /// Detect device type from capabilities
    ///
    /// Uses heuristics based on what the device supports:
    /// - Keys + no axes = Keyboard
    /// - Relative axes + buttons = Mouse  
    /// - Absolute axes + multi-touch = Touchscreen
    /// - Absolute axes + no multi-touch = Touchpad
    /// - Buttons + axes = Gamepad
    ///
    /// **Deep Debt Compliance:**
    /// - ✅ Runtime discovery (no hardcoding!)
    /// - ✅ Capability-based detection (self-knowledge!)
    /// - ✅ Agnostic heuristics (works with any device)
    fn detect_type(device: &evdev::Device) -> DeviceType {
        // Query actual hardware capabilities
        let has_keys = device
            .supported_keys()
            .is_some_and(|keys| keys.iter().count() > 0);

        let has_rel_axes = device
            .supported_relative_axes()
            .is_some_and(|axes| axes.iter().count() > 0);

        let has_abs_axes = device
            .supported_absolute_axes()
            .is_some_and(|axes| axes.iter().count() > 0);

        // Check for mouse buttons (BTN_LEFT, BTN_RIGHT, BTN_MIDDLE)
        let has_mouse_buttons = device.supported_keys().is_some_and(|keys| {
            keys.contains(evdev::KeyCode::BTN_LEFT)
                || keys.contains(evdev::KeyCode::BTN_RIGHT)
                || keys.contains(evdev::KeyCode::BTN_MIDDLE)
        });

        // Check for multi-touch (ABS_MT_SLOT, ABS_MT_POSITION_X, ABS_MT_POSITION_Y)
        let has_multitouch = device.supported_absolute_axes().is_some_and(|axes| {
            axes.contains(evdev::AbsoluteAxisCode::ABS_MT_SLOT)
                || axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_X)
                || axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_Y)
        });

        // Check for gamepad buttons (BTN_GAMEPAD, BTN_SOUTH, BTN_A, etc.)
        // Note: Some button constants may not exist in evdev 0.13
        let has_gamepad_buttons = device.supported_keys().is_some_and(|keys| {
            // Check for any gamepad-like buttons
            keys.iter().any(|key| {
                let code = key.code();
                // BTN_GAMEPAD = 0x130, BTN_SOUTH = 0x130, BTN_A through BTN_TRIGGER range
                (0x130..=0x13f).contains(&code)
            })
        });

        // Detection heuristics (ordered by specificity)
        if has_multitouch && has_abs_axes {
            DeviceType::Touchscreen
        } else if has_abs_axes && !has_mouse_buttons && has_keys {
            // Touchpad: absolute axes, keyboard-like keys (not mouse buttons)
            DeviceType::Touchpad
        } else if has_rel_axes && has_mouse_buttons {
            DeviceType::Mouse
        } else if has_keys && !has_rel_axes && !has_abs_axes {
            DeviceType::Keyboard
        } else if has_gamepad_buttons || (has_abs_axes && has_mouse_buttons && !has_rel_axes) {
            DeviceType::Gamepad
        } else {
            DeviceType::Other
        }
    }

    /// Detect device capabilities
    ///
    /// **Deep Debt Compliance:**
    /// - ✅ Runtime capability discovery
    /// - ✅ No assumptions about hardware
    /// - ✅ Self-knowledge from actual device
    fn detect_capabilities(device: &evdev::Device) -> Vec<DeviceCapability> {
        let mut caps = Vec::new();

        // Check for keys
        if device
            .supported_keys()
            .is_some_and(|k| k.iter().count() > 0)
        {
            caps.push(DeviceCapability::Keys);
        }

        // Check for relative pointer (REL_X, REL_Y - mouse movement)
        if device.supported_relative_axes().is_some_and(|axes| {
            axes.contains(evdev::RelativeAxisCode::REL_X)
                || axes.contains(evdev::RelativeAxisCode::REL_Y)
        }) {
            caps.push(DeviceCapability::RelativePointer);
        }

        // Check for absolute pointer (ABS_X, ABS_Y)
        if device.supported_absolute_axes().is_some_and(|axes| {
            axes.contains(evdev::AbsoluteAxisCode::ABS_X)
                || axes.contains(evdev::AbsoluteAxisCode::ABS_Y)
        }) {
            caps.push(DeviceCapability::AbsolutePointer);
        }

        // Check for multi-touch (ABS_MT_SLOT, ABS_MT_POSITION_X)
        if device.supported_absolute_axes().is_some_and(|axes| {
            axes.contains(evdev::AbsoluteAxisCode::ABS_MT_SLOT)
                || axes.contains(evdev::AbsoluteAxisCode::ABS_MT_POSITION_X)
        }) {
            caps.push(DeviceCapability::MultiTouch);
        }

        // Check for scroll wheel (REL_WHEEL, REL_HWHEEL)
        if device.supported_relative_axes().is_some_and(|axes| {
            axes.contains(evdev::RelativeAxisCode::REL_WHEEL)
                || axes.contains(evdev::RelativeAxisCode::REL_HWHEEL)
        }) {
            caps.push(DeviceCapability::Scroll);
        }

        // Check for force feedback
        if device
            .supported_ff()
            .is_some_and(|ff| ff.iter().count() > 0)
        {
            caps.push(DeviceCapability::ForceFeedback);
        }

        caps
    }

    /// Get device name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get device type
    #[must_use]
    pub const fn device_type(&self) -> DeviceType {
        self.device_type
    }

    /// Get device path
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Discover all input devices on the system
    ///
    /// **Deep Debt Compliance:**
    /// - ✅ Self-knowledge! (discovers own hardware)
    /// - ✅ No hardcoding! (scans /dev/input/)
    /// - ✅ Runtime discovery! (agnostic)
    /// - ✅ Pure Rust! (evdev crate)
    ///
    /// # Errors
    ///
    /// Returns an error if `/dev/input` cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::input::Device;
    /// let devices = Device::discover_all()?;
    /// for info in devices {
    ///     println!("Found: {} at {}", info.name, info.path.display());
    /// }
    /// # Ok::<(), toadstool_display::DisplayError>(())
    /// ```
    pub fn discover_all() -> Result<Vec<DeviceInfo>> {
        tracing::info!("🔍 Discovering input devices (self-knowledge)...");

        let mut devices = Vec::new();

        // Capability-based discovery: scan /dev/input/
        let input_dir = Path::new("/dev/input");
        if !input_dir.exists() {
            tracing::warn!("No /dev/input directory - no input devices available");
            return Ok(devices);
        }

        // Read directory entries
        for entry in std::fs::read_dir(input_dir)
            .map_err(|e| DisplayError::InputError(format!("Failed to read /dev/input: {e}")))?
        {
            let entry = entry?;
            let path = entry.path();

            // Only event* devices (not mice, mouse*, js*, etc.)
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with("event")
            {
                tracing::trace!("  Found: {}", path.display());

                match Self::open(&path) {
                    Ok(device) => {
                        devices.push(device.info());
                    }
                    Err(e) => {
                        tracing::debug!("  Skipped {} ({})", path.display(), e);
                    }
                }
            }
        }

        tracing::info!("✅ Discovered {} input device(s)", devices.len());

        // Log device types for debugging
        for dev in &devices {
            tracing::debug!("  - {} ({:?})", dev.name, dev.device_type);
        }

        Ok(devices)
    }

    /// Get device information
    ///
    /// **Deep Debt Compliance:**
    /// - ✅ Real capabilities (from hardware)
    /// - ✅ No placeholders
    #[must_use]
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            path: self.path.clone(),
            name: self.name.clone(),
            device_type: self.device_type,
            capabilities: self.capabilities.clone(),
        }
    }

    /// Get underlying evdev device (mutable)
    ///
    /// Provides mutable access to raw evdev device for event streaming.
    pub const fn evdev_device_mut(&mut self) -> &mut evdev::Device {
        &mut self.evdev_device
    }

    /// Get device vendor and product IDs
    ///
    /// Useful for device-specific handling if needed.
    #[must_use]
    pub fn device_ids(&self) -> Option<(u16, u16)> {
        let input_id = self.evdev_device.input_id();
        Some((input_id.vendor(), input_id.product()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_device_type_variants() {
        assert_eq!(DeviceType::Keyboard, DeviceType::Keyboard);
        assert_eq!(DeviceType::Mouse, DeviceType::Mouse);
        assert_eq!(DeviceType::Touchscreen, DeviceType::Touchscreen);
        assert_eq!(DeviceType::Touchpad, DeviceType::Touchpad);
        assert_eq!(DeviceType::Gamepad, DeviceType::Gamepad);
        assert_eq!(DeviceType::Other, DeviceType::Other);
        assert_ne!(DeviceType::Keyboard, DeviceType::Mouse);
    }

    #[test]
    fn test_device_capability_variants() {
        assert_eq!(DeviceCapability::Keys, DeviceCapability::Keys);
        assert_eq!(
            DeviceCapability::RelativePointer,
            DeviceCapability::RelativePointer
        );
        assert_eq!(
            DeviceCapability::AbsolutePointer,
            DeviceCapability::AbsolutePointer
        );
        assert_eq!(DeviceCapability::MultiTouch, DeviceCapability::MultiTouch);
        assert_eq!(DeviceCapability::Scroll, DeviceCapability::Scroll);
        assert_eq!(
            DeviceCapability::ForceFeedback,
            DeviceCapability::ForceFeedback
        );
    }

    #[test]
    fn test_device_info_creation() {
        let info = DeviceInfo {
            path: PathBuf::from("/dev/input/event0"),
            name: "Test Keyboard".to_string(),
            device_type: DeviceType::Keyboard,
            capabilities: vec![DeviceCapability::Keys],
        };
        assert_eq!(info.path, PathBuf::from("/dev/input/event0"));
        assert_eq!(info.name, "Test Keyboard");
        assert_eq!(info.device_type, DeviceType::Keyboard);
        assert_eq!(info.capabilities.len(), 1);
        assert_eq!(info.capabilities[0], DeviceCapability::Keys);
    }

    #[test]
    fn test_device_info_clone() {
        let info = DeviceInfo {
            path: PathBuf::from("/dev/input/event1"),
            name: "Test Mouse".to_string(),
            device_type: DeviceType::Mouse,
            capabilities: vec![DeviceCapability::RelativePointer, DeviceCapability::Scroll],
        };
        let cloned = info.clone();
        assert_eq!(info.path, cloned.path);
        assert_eq!(info.name, cloned.name);
        assert_eq!(info.device_type, cloned.device_type);
        assert_eq!(info.capabilities.len(), cloned.capabilities.len());
    }

    #[test]
    fn test_device_open_nonexistent_path() {
        let result = Device::open("/dev/input/nonexistent-event-99999");
        assert!(result.is_err());
        if let Err(e) = result {
            let err_str = e.to_string();
            assert!(err_str.contains("not found") || err_str.contains("Device"));
        }
    }

    #[test]
    fn test_discover_all_returns_result() {
        let result = Device::discover_all();
        assert!(result.is_ok());
        let devices = result.unwrap();
        assert!(
            devices
                .iter()
                .all(|d| d.path.to_string_lossy().starts_with("/dev/input/"))
        );
    }
}
