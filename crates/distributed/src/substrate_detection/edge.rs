//! Edge computing and IoT platform detection.

use std::fs;

use super::probe;
use super::types::PlatformType;
use toadstool::ToadStoolResult;

const MCU_TOOLS: &[(&str, &str)] = &[
    ("arduino-cli", "Arduino"),
    ("pio", "PlatformIO"),
    ("esptool", "ESP32/ESP8266"),
    ("openocd", "ARM Development"),
];

/// Detect edge computing platforms.
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();
    let arch = std::env::consts::ARCH.to_string();

    if let Ok(model) = fs::read_to_string("/proc/device-tree/model") {
        if model.contains("Raspberry Pi") {
            platforms.push(PlatformType::EdgeDevice {
                device_type: "Raspberry Pi".to_string(),
                architecture: arch.clone(),
            });
        } else if model.contains("BeagleBone") {
            platforms.push(PlatformType::EdgeDevice {
                device_type: "BeagleBone".to_string(),
                architecture: arch.clone(),
            });
        }
    }

    for (tool, platform) in MCU_TOOLS {
        if probe::command_exists(tool) {
            platforms.push(PlatformType::MCUDevelopment {
                platform: (*platform).to_string(),
                tool: (*tool).to_string(),
            });
        }
    }

    Ok(platforms)
}
