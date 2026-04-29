// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compile and upload flows (Arduino CLI).

use sha2::{Digest, Sha256};
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::device::ArduinoDevice;

impl ArduinoDevice {
    /// Compile Arduino code
    pub(super) async fn compile_code(&self, code: &str) -> ToadStoolResult<Vec<u8>> {
        let code_hash = format!("{:x}", Sha256::digest(code.as_bytes()));

        // Check cache first
        {
            let cache = self.compilation_cache.read().await;
            if let Some(compiled) = cache.get(&code_hash) {
                debug!("Using cached compilation for Arduino code");
                return Ok(compiled.clone());
            }
        }

        info!("Compiling Arduino code");

        // Write code to temporary file
        let temp_dir = std::env::temp_dir();
        let sketch_path = temp_dir.join(format!("arduino_sketch_{}.ino", code_hash));

        std::fs::write(&sketch_path, code).map_err(|e| {
            ToadStoolError::execution(format!("Failed to write sketch file: {}", e))
        })?;

        // Compile using Arduino CLI
        let sketch_path_str = sketch_path.to_str().ok_or_else(|| {
            ToadStoolError::execution(format!("Invalid sketch path: {:?}", sketch_path))
        })?;

        let output = std::process::Command::new("arduino-cli")
            .args([
                "compile",
                "--fqbn",
                "arduino:avr:uno", // Default to Uno
                sketch_path_str,
            ])
            .output()
            .map_err(|e| ToadStoolError::execution(format!("Failed to run Arduino CLI: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution(format!(
                "Arduino compilation failed: {}",
                error_msg
            )));
        }

        // Read compiled binary
        let hex_path = sketch_path.with_extension("hex");
        let compiled_code = std::fs::read(&hex_path).map_err(|e| {
            ToadStoolError::execution(format!("Failed to read compiled binary: {}", e))
        })?;

        // Cache compiled code
        {
            let mut cache = self.compilation_cache.write().await;
            cache.insert(code_hash, compiled_code.clone());
        }

        // Clean up temporary files
        let _ = std::fs::remove_file(&sketch_path);
        let _ = std::fs::remove_file(&hex_path);

        info!("Arduino code compiled successfully");
        Ok(compiled_code)
    }

    /// Upload compiled code to Arduino
    pub(super) async fn upload_code(&self, compiled_code: &[u8]) -> ToadStoolResult<()> {
        info!("Uploading code to Arduino");

        // Write binary to temporary file
        let temp_dir = std::env::temp_dir();
        let hex_path = temp_dir.join(format!("arduino_upload_{}.hex", Uuid::new_v4()));

        std::fs::write(&hex_path, compiled_code)
            .map_err(|e| ToadStoolError::execution(format!("Failed to write hex file: {}", e)))?;

        // Upload using Arduino CLI
        let hex_path_str = hex_path.to_str().ok_or_else(|| {
            ToadStoolError::execution(format!("Invalid hex file path: {:?}", hex_path))
        })?;

        let output = std::process::Command::new("arduino-cli")
            .args([
                "upload",
                "--fqbn",
                "arduino:avr:uno",
                "--port",
                &self.info.connection_info.address,
                "--input-file",
                hex_path_str,
            ])
            .output()
            .map_err(|e| {
                ToadStoolError::execution(format!("Failed to run Arduino CLI upload: {}", e))
            })?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution(format!(
                "Arduino upload failed: {}",
                error_msg
            )));
        }

        // Clean up temporary file
        let _ = std::fs::remove_file(&hex_path);

        info!("Code uploaded to Arduino successfully");
        Ok(())
    }
}
