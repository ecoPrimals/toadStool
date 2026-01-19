//! Input device management
//!
//! Safe wrappers around evdev input device operations.

#[allow(unused_imports)]
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
    /// Device type
    pub device_type: DeviceType,
    /// Supported capabilities
    pub capabilities: Vec<DeviceCapability>,
}

/// Device capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceCapability {
    /// Keys (keyboard)
    Keys,
    /// Relative pointer movement
    RelativePointer,
    /// Absolute pointer position
    AbsolutePointer,
    /// Multi-touch
    MultiTouch,
    /// Scroll wheel
    Scroll,
    /// Force feedback
    ForceFeedback,
}

/// Input device handle
///
/// Provides safe access to input device events.
///
/// ## Async
///
/// All event reading is async-compatible with tokio.
///
/// ## Safety
///
/// File descriptors are properly managed. No unsafe in public API.
#[allow(dead_code)] // TODO: Phase 0 - Remove when fully implemented
pub struct Device {
    path: PathBuf,
    name: String,
    device_type: DeviceType,
    // TODO: Add evdev device handle
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
    /// - Permission denied
    /// - Not an evdev device
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        if !path.exists() {
            return Err(DisplayError::InputError(
                format!("Device not found: {}", path.display())
            ));
        }
        
        tracing::debug!("Opening input device: {}", path.display());
        
        // TODO: Phase 0 - Implement device opening
        // let device = evdev::Device::open(&path)?;
        // let name = device.name()?;
        // let device_type = Self::detect_type(&device)?;
        
        Ok(Self {
            path,
            name: "TODO".into(),
            device_type: DeviceType::Other,
        })
    }
    
    /// Discover all input devices on the system
    ///
    /// **Self-knowledge**: Primal discovers its own hardware!
    /// **No hardcoding**: Agnostic runtime discovery!
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
            .map_err(|e| DisplayError::InputError(format!("Failed to read /dev/input: {}", e)))?
        {
            let entry = entry?;
            let path = entry.path();
            
            // Only event* devices
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("event") {
                    tracing::trace!("  Found: {}", path.display());
                    
                    // TODO: Open device and get info
                    // For now, just add path
                    devices.push(DeviceInfo {
                        path: path.clone(),
                        name: "TODO".into(),
                        device_type: DeviceType::Other,
                        capabilities: vec![],
                    });
                }
            }
        }
        
        tracing::info!("✅ Discovered {} input device(s)", devices.len());
        Ok(devices)
    }
    
    /// Get device information
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            path: self.path.clone(),
            name: self.name.clone(),
            device_type: self.device_type,
            capabilities: vec![], // TODO
        }
    }
}

// TODO: Phase 0 Implementation
//
// 1. Device opening:
//    - Use evdev::Device::open()
//    - Get device name, vendor/product IDs
//    - Detect capabilities (keys, rel/abs axes, etc.)
//
// 2. Type detection:
//    - Keys + no axes = Keyboard
//    - Rel axes + buttons = Mouse
//    - Abs axes + touch = Touchscreen/Touchpad
//    - Use heuristics based on capabilities
//
// 3. Event reading (async):
//    - device.into_event_stream()
//    - Integrate with tokio
//    - Parse InputEvent types
//
// 4. Hotplug support:
//    - Watch /dev/input with inotify or notify crate
//    - Emit device added/removed events
//
// No unsafe needed! evdev crate is pure Rust and safe.
