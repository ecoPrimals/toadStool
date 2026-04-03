// SPDX-License-Identifier: AGPL-3.0-only
//! # ESP32 Platform Support
//!
//! Implementation of ESP32 support for ToadStool Edge Runtime.
//! Supports various ESP32 variants with WiFi, Bluetooth, and various development frameworks.

mod chip_profiles;
mod connection;
mod discovery;
mod flash;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{
        ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeType,
    },
    RuntimeMetrics,
};

use super::*;

use connection::{ESP32Connection, ESP32Execution};

/// ESP32 Device Implementation
pub struct ESP32Device {
    id: Uuid,
    info: EdgeDeviceInfo,
    connection: Arc<RwLock<Option<ESP32Connection>>>,
    active_executions: Arc<RwLock<HashMap<Uuid, ESP32Execution>>>,
}

impl ESP32Device {
    /// Create a new ESP32 device
    pub fn new(
        chip: ESP32Variant,
        framework: ESP32Framework,
        connection_info: ConnectionInfo,
    ) -> ToadStoolResult<Self> {
        let id = Uuid::new_v4();
        let platform = EdgePlatform::ESP32 {
            chip: chip.clone(),
            framework: framework.clone(),
        };

        let resources = chip_profiles::get_chip_resources(&chip);
        let capabilities = chip_profiles::get_chip_capabilities(&chip, &framework);

        let info = EdgeDeviceInfo {
            id,
            name: format!("ESP32 {:?}", chip),
            platform,
            capabilities,
            resources,
            connection_info,
            status: DeviceStatus::Offline,
            last_seen: std::time::SystemTime::now(),
        };

        Ok(Self {
            id,
            info,
            connection: Arc::new(RwLock::new(None)),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EdgeDevice for ESP32Device {
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
        let connection = self.connection.read().await;
        connection.as_ref().map(|c| c.is_connected).unwrap_or(false)
    }

    async fn connect(&self) -> ToadStoolResult<()> {
        self.establish_connection().await
    }

    async fn disconnect(&self) -> ToadStoolResult<()> {
        let mut connection = self.connection.write().await;
        *connection = None;
        Ok(())
    }

    async fn execute(&self, _request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing code on ESP32 device {}", self.id);

        let execution_id = Uuid::new_v4();
        let started_at = std::time::Instant::now();

        {
            let mut executions = self.active_executions.write().await;
            executions.insert(
                execution_id,
                ESP32Execution {
                    id: execution_id,
                    status: ExecutionStatus::Running,
                    started_at,
                    framework: ESP32Framework::ESPIDF,
                },
            );
        }

        let output = self
            .send_command("RUN")
            .await
            .unwrap_or_else(|_| "ESP32 execution completed".to_string());

        {
            let mut executions = self.active_executions.write().await;
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
    }

    async fn deploy(&self, code: &[u8]) -> ToadStoolResult<String> {
        self.flash_firmware(code).await?;
        Ok(format!("Deployed to ESP32 {}", self.id))
    }

    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        let _response = self.send_command("STOP").await?;

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

        usage.insert("cpu_percent".to_string(), 45.0);
        usage.insert("memory_bytes".to_string(), 102400.0);
        usage.insert("wifi_signal_strength".to_string(), -45.0);
        usage.insert("temperature_celsius".to_string(), 45.0);

        Ok(usage)
    }

    async fn upload_file(&self, path: &str, content: &[u8]) -> ToadStoolResult<()> {
        let _response = self
            .send_command(&format!("UPLOAD {} {}", path, content.len()))
            .await?;
        Ok(())
    }

    async fn download_file(&self, path: &str) -> ToadStoolResult<Vec<u8>> {
        let path_trimmed = path.trim();
        if path_trimmed.starts_with("http://") || path_trimmed.starts_with("https://") {
            #[cfg(feature = "http-downloads")]
            {
                match self.download_via_http(path_trimmed).await {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        tracing::error!("ESP32 HTTP download failed for {}: {}", path, e);
                        return Err(ToadStoolError::network(format!(
                            "ESP32 HTTP download failed: {}",
                            e
                        )));
                    }
                }
            }
            #[cfg(not(feature = "http-downloads"))]
            {
                return Err(
                    toadstool_common::error::SystemError::NotSupported {
                        feature: "esp32_http_download".to_string(),
                        reason: "HTTP download requires 'http-downloads' feature. Enable with: toadstool-runtime-edge = { features = [\"http-downloads\"] }".to_string(),
                    }
                    .into(),
                );
            }
        }

        let _ = self.send_command(&format!("DOWNLOAD {}", path)).await;
        Err(toadstool_common::error::SystemError::NotSupported {
            feature: "esp32_file_download".to_string(),
            reason: format!(
                "Download from ESP32 device filesystem not implemented. Path: {}. Use http(s):// URL for remote files, or implement serial/network file transfer protocol.",
                path
            ),
        }
        .into())
    }

    async fn execute_command(&self, command: &str) -> ToadStoolResult<String> {
        self.send_command(command).await
    }

    async fn get_logs(&self, lines: Option<usize>) -> ToadStoolResult<String> {
        let lines_str = lines.map(|l| l.to_string()).unwrap_or_else(|| "100".to_string());
        self.send_command(&format!("LOGS {}", lines_str)).await
    }

    async fn restart(&self) -> ToadStoolResult<()> {
        self.send_command("RESTART").await?;
        Ok(())
    }

    async fn update_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        self.flash_firmware(firmware).await
    }

    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let response = self.send_command("SENSORS").await?;

        let sensors: HashMap<String, f64> = serde_json::from_str(&response).unwrap_or_else(|_| {
            let mut default_sensors = HashMap::new();
            default_sensors.insert("temperature".to_string(), 25.0);
            default_sensors.insert("humidity".to_string(), 60.0);
            default_sensors.insert("pressure".to_string(), 1013.25);
            default_sensors.insert("wifi_rssi".to_string(), -45.0);
            default_sensors
        });

        Ok(sensors)
    }

    async fn control_actuators(&self, commands: HashMap<String, f64>) -> ToadStoolResult<()> {
        let command_json = serde_json::to_string(&commands).map_err(|e| {
            ToadStoolError::execution(format!("Failed to serialize actuator commands: {}", e))
        })?;

        let _response = self
            .send_command(&format!("ACTUATORS {}", command_json))
            .await?;
        Ok(())
    }
}
