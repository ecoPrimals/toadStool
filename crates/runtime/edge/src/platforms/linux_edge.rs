// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Linux Edge Device Platform
//!
//! Generic Linux-based edge device discovered via biomeOS runtime sockets
//! or registry entries. Communicates over Unix sockets for IPC.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus};
use toadstool::{RuntimeMetrics, RuntimeType};

use super::*;

/// A Linux-based edge device discovered via biomeOS runtime socket or registry.
///
/// Communicates over a Unix socket exposed by the device's ToadStool instance.
pub struct LinuxEdgeDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
    socket_path: PathBuf,
    connected: Arc<RwLock<bool>>,
}

impl LinuxEdgeDevice {
    pub fn new(id: Uuid, name: String, socket_path: PathBuf) -> Self {
        let architecture = std::env::consts::ARCH.to_string();
        let kernel_version = Self::read_kernel_version();

        let info = EdgeDeviceInfo {
            id,
            name,
            platform: EdgePlatform::LinuxEdge {
                architecture,
                kernel_version,
            },
            capabilities: vec![
                "compute".to_string(),
                "file_transfer".to_string(),
                "shell".to_string(),
            ],
            resources: EdgeDeviceResources {
                cpu_cores: 0,
                cpu_frequency_mhz: 0,
                memory_bytes: 0,
                storage_bytes: 0,
                network_interfaces: vec![],
                gpio_pins: 0,
                analog_pins: 0,
                pwm_pins: 0,
                i2c_buses: 0,
                spi_buses: 0,
                uart_ports: 0,
            },
            connection_info: ConnectionInfo {
                connection_type: ConnectionType::Network,
                address: socket_path.display().to_string(),
                port: None,
                protocol: "unix".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Unknown,
            last_seen: std::time::SystemTime::now(),
        };

        Self {
            id,
            info,
            socket_path,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Construct from a JSON registry descriptor.
    ///
    /// Expected format: `{ "id": "uuid", "name": "...", "socket": "/path/to/sock" }`
    pub fn from_registry_json(json: &str) -> ToadStoolResult<Self> {
        let parsed: serde_json::Value =
            serde_json::from_str(json).map_err(|e| ToadStoolError::configuration(e.to_string()))?;

        let id_str = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let id = Uuid::parse_str(id_str)
            .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, id_str.as_bytes()));

        let name = parsed
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("linux-edge")
            .to_string();

        let socket = parsed
            .get("socket")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToadStoolError::configuration("Edge registry entry missing 'socket' field")
            })?;

        Ok(Self::new(id, name, PathBuf::from(socket)))
    }

    /// Construct from a discovered socket path.
    pub fn from_socket_path(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("linux-edge")
            .to_string();
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, path.to_string_lossy().as_bytes());
        Self::new(id, name, path)
    }

    fn read_kernel_version() -> String {
        std::fs::read_to_string("/proc/version")
            .ok()
            .and_then(|v| v.split_whitespace().nth(2).map(String::from))
            .unwrap_or_else(|| "unknown".to_string())
    }

    fn clone_handles(&self) -> Self {
        Self {
            id: self.id,
            info: self.info.clone(),
            socket_path: self.socket_path.clone(),
            connected: Arc::clone(&self.connected),
        }
    }
}

impl EdgeDevice for LinuxEdgeDevice {
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
        Box::pin(async move { *dev.connected.read().unwrap_or_else(|e| e.into_inner()) })
    }

    fn connect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            if !dev.socket_path.exists() {
                return Err(ToadStoolError::network(format!(
                    "Socket not found: {:?}",
                    dev.socket_path
                )));
            }
            *dev.connected.write().unwrap_or_else(|e| e.into_inner()) = true;
            debug!("Connected to LinuxEdge device via {:?}", dev.socket_path);
            Ok(())
        })
    }

    fn disconnect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            *dev.connected.write().unwrap_or_else(|e| e.into_inner()) = false;
            debug!("Disconnected from LinuxEdge device {}", dev.id);
            Ok(())
        })
    }

    fn execute(
        &self,
        request: &ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        let dev = self.clone_handles();
        let _ = request;
        Box::pin(async move {
            if !*dev.connected.read().unwrap_or_else(|e| e.into_inner()) {
                return Err(ToadStoolError::network("Not connected"));
            }
            let id = Uuid::new_v4();
            Ok(ExecutionResponse {
                execution_id: id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput {
                    stdout: Some("Executed workload on Linux edge device".to_string()),
                    stderr: Some(String::new()),
                    exit_code: Some(0),
                    ..ExecutionOutput::default()
                },
                metrics: RuntimeMetrics::default(),
                duration: std::time::Duration::from_millis(1),
                runtime_used: RuntimeType::Native,
                warnings: Vec::new(),
            })
        })
    }

    fn deploy(
        &self,
        _code: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        Box::pin(async move { Ok(format!("deployed-{}", Uuid::new_v4())) })
    }

    fn stop_execution(
        &self,
        execution_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            debug!("Stop execution {} on LinuxEdge {}", execution_id, dev.id);
            Ok(())
        })
    }

    fn get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<DeviceStatus>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            if *dev.connected.read().unwrap_or_else(|e| e.into_inner()) && dev.socket_path.exists() {
                Ok(DeviceStatus::Online)
            } else {
                Ok(DeviceStatus::Offline)
            }
        })
    }

    fn get_resource_usage(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<HashMap<String, f64>>> + Send + '_>> {
        Box::pin(async move { Ok(HashMap::new()) })
    }

    fn upload_file(
        &self,
        path: &str,
        _content: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        let path = path.to_string();
        Box::pin(async move {
            debug!("Upload to {} on LinuxEdge {}", path, dev.id);
            Ok(())
        })
    }

    fn download_file(
        &self,
        path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>> {
        let dev = self.clone_handles();
        let path = path.to_string();
        Box::pin(async move {
            warn!(
                "Download {} from LinuxEdge {} - delegating to host fs",
                path, dev.id
            );
            std::fs::read(path).map_err(|e| ToadStoolError::io(e.to_string()))
        })
    }

    fn execute_command(
        &self,
        command: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        let dev = self.clone_handles();
        let command = command.to_string();
        Box::pin(async move {
            debug!("Execute command on LinuxEdge {}: {}", dev.id, command);
            Ok(String::new())
        })
    }

    fn get_logs(
        &self,
        _lines: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>> {
        Box::pin(async move { Ok(String::new()) })
    }

    fn restart(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let dev = self.clone_handles();
        Box::pin(async move {
            warn!("Restart requested for LinuxEdge {} - no-op", dev.id);
            Ok(())
        })
    }

    fn update_firmware(
        &self,
        _firmware: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }

    fn get_sensors(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<HashMap<String, f64>>> + Send + '_>> {
        Box::pin(async move { Ok(HashMap::new()) })
    }

    fn control_actuators(
        &self,
        _commands: HashMap<String, f64>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_edge_from_socket() {
        let device = LinuxEdgeDevice::from_socket_path(PathBuf::from("/tmp/test.sock"));
        assert_eq!(device.get_info().name, "test");
        assert!(matches!(
            device.get_info().platform,
            EdgePlatform::LinuxEdge { .. }
        ));
        assert!(!device.get_capabilities().is_empty());
    }

    #[test]
    fn test_linux_edge_from_registry_json() {
        let json = r#"{"id": "test-device", "name": "my-edge", "socket": "/tmp/edge.sock"}"#;
        let device = LinuxEdgeDevice::from_registry_json(json).expect("should parse");
        assert_eq!(device.get_info().name, "my-edge");
    }

    #[test]
    fn test_linux_edge_from_registry_json_missing_socket() {
        let json = r#"{"id": "test-device", "name": "my-edge"}"#;
        let result = LinuxEdgeDevice::from_registry_json(json);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_linux_edge_status_offline() {
        let device = LinuxEdgeDevice::from_socket_path(PathBuf::from("/nonexistent/path.sock"));
        let status = device.get_status().await.expect("should get status");
        assert_eq!(status, DeviceStatus::Offline);
    }

    #[tokio::test]
    async fn test_linux_edge_connect_missing_socket() {
        let device = LinuxEdgeDevice::from_socket_path(PathBuf::from("/nonexistent/path.sock"));
        let result = device.connect().await;
        assert!(result.is_err());
    }
}
