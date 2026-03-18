// SPDX-License-Identifier: AGPL-3.0-or-later
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
#[allow(clippy::unused_async)] // Sync probe; async for API consistency with SubstrateDetector
pub async fn detect() -> ToadStoolResult<Vec<PlatformType>> {
    let mut platforms = Vec::new();
    let arch = std::env::consts::ARCH.to_string();

    if let Ok(model) = fs::read_to_string("/proc/device-tree/model") {
        let device_type = if model.contains("Raspberry Pi") {
            Some("Raspberry Pi")
        } else if model.contains("BeagleBone") {
            Some("BeagleBone")
        } else {
            None
        };
        if let Some(dt) = device_type {
            platforms.push(PlatformType::EdgeDevice {
                device_type: dt.to_string(),
                architecture: arch,
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
