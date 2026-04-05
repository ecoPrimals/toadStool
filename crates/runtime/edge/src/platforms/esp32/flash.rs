// SPDX-License-Identifier: AGPL-3.0-or-later
//! Firmware flashing and optional HTTP download for ESP32.

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tracing::info;

use super::ESP32Device;

impl ESP32Device {
    /// Flash firmware to ESP32
    pub(crate) async fn flash_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        info!("Flashing firmware to ESP32");

        let temp_dir = std::env::temp_dir();
        let firmware_path = temp_dir.join(format!("esp32_firmware_{}.bin", self.id));

        std::fs::write(&firmware_path, firmware).map_err(|e| {
            ToadStoolError::execution(format!("Failed to write firmware file: {}", e))
        })?;

        let firmware_path_str = firmware_path.to_str().ok_or_else(|| {
            ToadStoolError::execution(format!("Invalid firmware path: {:?}", firmware_path))
        })?;

        let output = std::process::Command::new("esptool.py")
            .args(&[
                "--chip",
                "esp32",
                "--port",
                &self.info.connection_info.address,
                "--baud",
                "460800",
                "write_flash",
                "-z",
                "0x1000",
                firmware_path_str,
            ])
            .output()
            .map_err(|e| ToadStoolError::execution(format!("Failed to run esptool: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution(format!(
                "ESP32 flash failed: {}",
                error_msg
            )));
        }

        let _ = std::fs::remove_file(&firmware_path);

        info!("ESP32 firmware flashed successfully");
        Ok(())
    }

    /// Download file via HTTP(S) when path is a URL.
    #[cfg(feature = "http-downloads")]
    pub(crate) async fn download_via_http(&self, url: &str) -> ToadStoolResult<Vec<u8>> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ToadStoolError::network(format!("Failed to create HTTP client: {}", e)))?;
        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToadStoolError::network(format!("HTTP request failed: {}", e)))?
            .error_for_status()
            .map_err(|e| ToadStoolError::network(format!("HTTP error: {}", e)))?
            .bytes()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to read response: {}", e)))?;
        Ok(bytes.to_vec())
    }
}
