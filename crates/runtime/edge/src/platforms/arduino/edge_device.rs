// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`EdgeDevice`] trait implementation for Arduino.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus};
use toadstool::{RuntimeMetrics, RuntimeType};

use super::super::*;
use super::device::{ArduinoDevice, ArduinoExecution};

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

    fn is_connected(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            #[cfg(feature = "serial-transport")]
            {
                dev.serial_port.lock().await.is_some()
            }
            #[cfg(not(feature = "serial-transport"))]
            {
                let _ = dev;
                false
            }
        })
    }

    fn connect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move { dev.open_serial_connection().await })
    }

    fn disconnect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move { dev.close_serial_connection().await })
    }

    fn execute(
        &self,
        request: &ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        let dev = self.clone_handles();
        let request = request.clone();
        Box::pin(async move {
            info!("Executing code on Arduino device {}", dev.id);

            let code = std::str::from_utf8(&request.input_data.data).map_err(|e| {
                ToadStoolError::execution(format!("Invalid UTF-8 in Arduino workload: {}", e))
            })?;

            let execution_id = Uuid::new_v4();
            let started_at = std::time::Instant::now();

            {
                let mut executions = dev.active_executions.write().unwrap_or_else(|e| e.into_inner());
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

            let compiled_code = dev.compile_code(code).await?;
            dev.upload_code(&compiled_code).await?;

            const SERIAL_READ_TIMEOUT_SECS: u64 = 2;
            let output = match dev
                .read_serial_output(Duration::from_secs(SERIAL_READ_TIMEOUT_SECS))
                .await
            {
                Ok(serial_out) if !serial_out.is_empty() => serial_out,
                Ok(_) => "Deployed — no serial output within timeout".to_string(),
                Err(_) => "Deployed — serial monitor unavailable".to_string(),
            };

            {
                let mut executions = dev.active_executions.write().unwrap_or_else(|e| e.into_inner());
                if let Some(execution) = executions.get_mut(&execution_id) {
                    execution.status = ExecutionStatus::Success;
                }
            }

            Ok(ExecutionResponse {
                execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput {
                    stdout: Some(output),
                    stderr: Some(String::new()),
                    exit_code: Some(0),
                    ..ExecutionOutput::default()
                },
                metrics: RuntimeMetrics::default(),
                duration: started_at.elapsed(),
                runtime_used: RuntimeType::Native,
                warnings: Vec::new(),
            })
        })
    }

    fn deploy(
        &self,
        code: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        let dev = self.clone_handles();
        let code = code.to_vec();
        Box::pin(async move {
            let code_str = std::str::from_utf8(&code).map_err(|e| {
                ToadStoolError::execution(format!("Invalid UTF-8 in Arduino code: {}", e))
            })?;

            let compiled_code = dev.compile_code(code_str).await?;
            dev.upload_code(&compiled_code).await?;

            Ok(format!("Deployed to Arduino {}", dev.id))
        })
    }

    fn stop_execution(
        &self,
        execution_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            let _response = dev.send_command("RESET").await?;

            {
                let mut executions = dev.active_executions.write().unwrap_or_else(|e| e.into_inner());
                if let Some(execution) = executions.get_mut(&execution_id) {
                    execution.status = ExecutionStatus::Cancelled;
                }
            }

            Ok(())
        })
    }

    fn get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<DeviceStatus>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            #[cfg(feature = "serial-transport")]
            {
                if dev.serial_port.lock().await.is_some() {
                    Ok(DeviceStatus::Online)
                } else {
                    Ok(DeviceStatus::Offline)
                }
            }
            #[cfg(not(feature = "serial-transport"))]
            {
                let _ = dev;
                Ok(DeviceStatus::Offline)
            }
        })
    }

    fn get_resource_usage(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<HashMap<String, f64>>> + Send + '_>> {
        Box::pin(async move {
            let mut usage = HashMap::new();

            usage.insert("cpu_percent".to_string(), 50.0);
            usage.insert("memory_bytes".to_string(), 512.0);
            usage.insert("flash_usage_percent".to_string(), 25.0);

            Ok(usage)
        })
    }

    fn upload_file(
        &self,
        _path: &str,
        _content: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            Err(ToadStoolError::not_supported(
                "File upload not supported on Arduino".to_string(),
            ))
        })
    }

    fn download_file(
        &self,
        _path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>> {
        Box::pin(async move {
            Err(ToadStoolError::not_supported(
                "File download not supported on Arduino".to_string(),
            ))
        })
    }

    fn execute_command(
        &self,
        command: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        let dev = self.clone_handles();
        let command = command.to_string();
        Box::pin(async move { dev.send_command(&command).await })
    }

    fn get_logs(
        &self,
        _lines: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move { dev.send_command("LOGS").await })
    }

    fn restart(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            dev.send_command("RESTART").await?;
            Ok(())
        })
    }

    fn update_firmware(
        &self,
        firmware: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        let firmware = firmware.to_vec();
        Box::pin(async move {
            let firmware_str = std::str::from_utf8(&firmware).map_err(|e| {
                ToadStoolError::execution(format!("Invalid UTF-8 in Arduino firmware: {}", e))
            })?;

            let compiled_code = dev.compile_code(firmware_str).await?;
            dev.upload_code(&compiled_code).await?;

            Ok(())
        })
    }

    fn get_sensors(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<HashMap<String, f64>>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            let response = dev.send_command("SENSORS").await?;

            let sensors: HashMap<String, f64> =
                serde_json::from_str(&response).unwrap_or_else(|_| HashMap::new());

            Ok(sensors)
        })
    }

    fn control_actuators(
        &self,
        commands: HashMap<String, f64>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            let command_json = serde_json::to_string(&commands).map_err(|e| {
                ToadStoolError::execution(format!("Failed to serialize actuator commands: {}", e))
            })?;

            let _response = dev
                .send_command(&format!("ACTUATORS {}", command_json))
                .await?;
            Ok(())
        })
    }
}
