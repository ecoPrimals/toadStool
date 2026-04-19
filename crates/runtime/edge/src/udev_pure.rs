// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure Rust udev device discovery
//!
//! Reads device information directly from /sys/class without requiring libudev.
//! This provides ecoBin compliance by avoiding C dependencies.
//!
//! Similar to how GPU discovery reads from /sys/class/drm, this module reads
//! from various /sys/class subdirectories to discover devices.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Udev device information
#[derive(Debug, Clone)]
pub struct UdevDevice {
    /// Device name (e.g., "ttyUSB0")
    pub name: String,
    /// Device subsystem (e.g., "tty", "usb", "input")
    pub subsystem: String,
    /// Device class path in /sys/class
    pub sys_path: PathBuf,
    /// Device properties from /sys/class
    pub properties: HashMap<String, String>,
    /// Vendor ID if available
    pub vendor_id: Option<u16>,
    /// Product ID if available
    pub product_id: Option<u16>,
}

/// Pure Rust udev parser
pub struct UdevParser;

impl UdevParser {
    /// Discover devices from a specific /sys/class subsystem
    ///
    /// # Arguments
    /// * `subsystem` - The subsystem name (e.g., "tty", "usb", "input")
    ///
    /// # Returns
    /// Vector of discovered devices
    pub fn discover_subsystem(subsystem: &str) -> ToadStoolResult<Vec<UdevDevice>> {
        let sys_class_path = Path::new("/sys/class").join(subsystem);

        if !sys_class_path.exists() {
            debug!("Subsystem {} does not exist", subsystem);
            return Ok(Vec::new());
        }

        let mut devices = Vec::new();

        let entries = fs::read_dir(&sys_class_path).map_err(|e| {
            ToadStoolError::runtime(format!(
                "Failed to read /sys/class/{}: {}",
                subsystem, e
            ))
        })?;

        for entry in entries.flatten() {
            let device_path = entry.path();
            let device_name = device_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if let Ok(device) = Self::parse_device(&device_path, subsystem, &device_name) {
                devices.push(device);
            }
        }

        Ok(devices)
    }

    /// Parse a single device from its /sys/class path
    fn parse_device(
        device_path: &Path,
        subsystem: &str,
        name: &str,
    ) -> ToadStoolResult<UdevDevice> {
        let mut properties = HashMap::new();
        let mut vendor_id = None;
        let mut product_id = None;

        // Read device properties from various files in the device directory
        if let Ok(uevent) = fs::read_to_string(device_path.join("uevent")) {
            for line in uevent.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    properties.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Try to read vendor and product IDs from device path
        // For USB devices, these are often in /sys/class/tty/ttyUSB0/device/../idVendor
        if let Ok(device_link) = fs::read_link(device_path.join("device")) {
            let device_real_path = device_path.join(&device_link);

            // Try to find vendor and product IDs
            if let Ok(vendor_str) = fs::read_to_string(device_real_path.join("idVendor")) {
                if let Ok(vid) = u16::from_str_radix(vendor_str.trim(), 16) {
                    vendor_id = Some(vid);
                }
            }

            if let Ok(product_str) = fs::read_to_string(device_real_path.join("idProduct")) {
                if let Ok(pid) = u16::from_str_radix(product_str.trim(), 16) {
                    product_id = Some(pid);
                }
            }
        }

        Ok(UdevDevice {
            name: name.to_string(),
            subsystem: subsystem.to_string(),
            sys_path: device_path.to_path_buf(),
            properties,
            vendor_id,
            product_id,
        })
    }

    /// Discover all USB serial devices (ttyUSB*, ttyACM*)
    pub fn discover_usb_serial() -> ToadStoolResult<Vec<UdevDevice>> {
        let mut devices = Vec::new();

        // Check common USB serial device classes
        for subsystem in &["tty"] {
            let mut subsystem_devices = Self::discover_subsystem(subsystem)?;

            // Filter for USB serial devices
            subsystem_devices.retain(|d| {
                d.name.starts_with("ttyUSB")
                    || d.name.starts_with("ttyACM")
                    || d.name.starts_with("ttyS")
            });

            devices.extend(subsystem_devices);
        }

        Ok(devices)
    }

    /// Discover all input devices
    pub fn discover_input_devices() -> ToadStoolResult<Vec<UdevDevice>> {
        Self::discover_subsystem("input")
    }

    /// Discover all USB devices
    pub fn discover_usb_devices() -> ToadStoolResult<Vec<UdevDevice>> {
        let mut devices = Vec::new();

        // USB devices can be in multiple subsystems
        for subsystem in &["usb", "tty", "input"] {
            let mut subsystem_devices = Self::discover_subsystem(subsystem)?;

            // Filter for devices with USB vendor/product IDs
            subsystem_devices.retain(|d| d.vendor_id.is_some() || d.product_id.is_some());

            devices.extend(subsystem_devices);
        }

        Ok(devices)
    }

    /// Get device property
    pub fn get_property<'a>(device: &'a UdevDevice, key: &str) -> Option<&'a String> {
        device.properties.get(key)
    }

    /// Check if device matches vendor/product IDs
    pub fn matches_vid_pid(device: &UdevDevice, vid: Option<u16>, pid: Option<u16>) -> bool {
        match (vid, pid) {
            (Some(v), Some(p)) => device.vendor_id == Some(v) && device.product_id == Some(p),
            (Some(v), None) => device.vendor_id == Some(v),
            (None, Some(p)) => device.product_id == Some(p),
            (None, None) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udev_parser_creation() {
        // Test that parser can be created
        let _parser = UdevParser;
    }

    #[test]
    fn test_discover_subsystem_nonexistent() {
        // Test discovering a non-existent subsystem
        let result = UdevParser::discover_subsystem("nonexistent_subsystem_xyz");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_matches_vid_pid() {
        let device = UdevDevice {
            name: "test".to_string(),
            subsystem: "test".to_string(),
            sys_path: PathBuf::from("/sys/class/test/test"),
            properties: HashMap::new(),
            vendor_id: Some(0x1234),
            product_id: Some(0x5678),
        };

        assert!(UdevParser::matches_vid_pid(
            &device,
            Some(0x1234),
            Some(0x5678)
        ));
        assert!(!UdevParser::matches_vid_pid(
            &device,
            Some(0x1234),
            Some(0x9999)
        ));
        assert!(UdevParser::matches_vid_pid(&device, Some(0x1234), None));
        assert!(UdevParser::matches_vid_pid(&device, None, Some(0x5678)));
    }
}
