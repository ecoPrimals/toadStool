// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`EdgeDevice`] trait implementation for Arduino.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus,
};

use super::device::{ArduinoDevice, ArduinoExecution};
use super::super::*;

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EdgeDevice for ArduinoDevice {
    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_info(&self) -> EdgeDeviceInfo {
        self.info.clone()
    }

    fn get_platform(&self) -> &EdgePlatform {
        &self.info.platform
    }

    fn get_capabilities(&self) -> Vec<String> {
        self.info.capabilities.clone()
    }

    async fn is_connected(&self) -> bool {
        self.serial_port.read().await.is_some()
    }

    async fn connect(&self) -> ToadStoolResult<()> {
        self.open_serial_connection().await
    }

    async fn disconnect(&self) -> ToadStoolResult<()> {
        self.close_serial_connection().await
    }

    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing code on Arduino device {}", self.id);

        // Extract code from request
        let code = std::str::from_utf8(&request.code).map_err(|e| {
            ToadStoolError::execution_error(format!("Invalid UTF-8 in Arduino code: {}", e))
        })?;

        let execution_id = Uuid::new_v4();
        let started_at = std::time::Instant::now();

        // Store execution info
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(
                execution_id,
                ArduinoExecution {
                    id: execution_id,
                    status: ExecutionStatus::Running,
                    started_at,
                    code_hash: format!("{:x}", Sha256::digest(code.as_bytes())),
                },
            );
        }

        // Compile and upload code
        let compiled_code = self.compile_code(code).await?;
        self.upload_code(&compiled_code).await?;

        // Read serial output after upload completes.
        // Arduino boards run continuously; we collect whatever the board
        // has written back to serial since the upload finished.
        let output = match self.read_serial_output(Duration::from_secs(2)).await {
            Ok(serial_out) if !serial_out.is_empty() => serial_out,
            Ok(_) => "Deployed — no serial output within timeout".to_string(),
            Err(_) => "Deployed — serial monitor unavailable".to_string(),
        };

        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(&execution_id) {
                execution.status = ExecutionStatus::Success;
            }
        }

        Ok(ExecutionResponse {
            id: execution_id,
            status: ExecutionStatus::Success,
            output: Some(ExecutionOutput {
                stdout: output,
                stderr: String::new(),
                exit_code: Some(0),
            }),
            execution_time_ms: started_at.elapsed().as_millis() as u64,
            resource_usage: Some(HashMap::new()),
        })
    }

    async fn deploy(&self, code: &[u8]) -> ToadStoolResult<String> {
        let code_str = std::str::from_utf8(code).map_err(|e| {
            ToadStoolError::execution_error(format!("Invalid UTF-8 in Arduino code: {}", e))
        })?;

        let compiled_code = self.compile_code(code_str).await?;
        self.upload_code(&compiled_code).await?;

        Ok(format!("Deployed to Arduino {}", self.id))
    }

    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        // Send reset command to Arduino
        let _response = self.send_command("RESET").await?;

        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(&execution_id) {
                execution.status = ExecutionStatus::Cancelled;
            }
        }

        Ok(())
    }

    async fn get_status(&self) -> ToadStoolResult<DeviceStatus> {
        if self.is_connected().await {
            Ok(DeviceStatus::Online)
        } else {
            Ok(DeviceStatus::Offline)
        }
    }

    async fn get_resource_usage(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let mut usage = HashMap::new();

        // Arduino resource usage is typically minimal
        usage.insert("cpu_percent".to_string(), 50.0); // Estimated
        usage.insert("memory_bytes".to_string(), 512.0); // Estimated
        usage.insert("flash_usage_percent".to_string(), 25.0); // Estimated

        Ok(usage)
    }

    async fn upload_file(&self, _path: &str, _content: &[u8]) -> ToadStoolResult<()> {
        Err(ToadStoolError::not_supported(
            "File upload not supported on Arduino".to_string(),
        ))
    }

    async fn download_file(&self, _path: &str) -> ToadStoolResult<Vec<u8>> {
        Err(ToadStoolError::not_supported(
            "File download not supported on Arduino".to_string(),
        ))
    }

    async fn execute_command(&self, command: &str) -> ToadStoolResult<String> {
        self.send_command(command).await
    }

    async fn get_logs(&self, _lines: Option<usize>) -> ToadStoolResult<String> {
        // Read from serial monitor
        self.send_command("LOGS").await
    }

    async fn restart(&self) -> ToadStoolResult<()> {
        self.send_command("RESTART").await?;
        Ok(())
    }

    async fn update_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        // For Arduino, firmware update is essentially code upload
        let firmware_str = std::str::from_utf8(firmware).map_err(|e| {
            ToadStoolError::execution_error(format!("Invalid UTF-8 in Arduino firmware: {}", e))
        })?;

        let compiled_code = self.compile_code(firmware_str).await?;
        self.upload_code(&compiled_code).await?;

        Ok(())
    }

    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let response = self.send_command("SENSORS").await?;

        // Parse sensor data (assuming JSON format)
        let sensors: HashMap<String, f64> =
            serde_json::from_str(&response).unwrap_or_else(|_| HashMap::new());

        Ok(sensors)
    }

    async fn control_actuators(&self, commands: HashMap<String, f64>) -> ToadStoolResult<()> {
        let command_json = serde_json::to_string(&commands).map_err(|e| {
            ToadStoolError::execution_error(format!("Failed to serialize actuator commands: {}", e))
        })?;

        let _response = self
            .send_command(&format!("ACTUATORS {}", command_json))
            .await?;
        Ok(())
    }
}
