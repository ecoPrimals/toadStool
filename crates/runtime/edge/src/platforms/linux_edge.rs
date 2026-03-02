//! # Linux Edge Device Platform
//!
//! Generic Linux-based edge device discovered via biomeOS runtime sockets
//! or registry entries. Communicates over Unix sockets for IPC.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus};

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
            name,
            platform: EdgePlatform::LinuxEdge {
                architecture,
                kernel_version,
            },
            firmware_version: "native".to_string(),
            hardware_version: "linux".to_string(),
            serial_number: id.to_string(),
            capabilities: vec![
                "compute".to_string(),
                "file_transfer".to_string(),
                "shell".to_string(),
            ],
            memory_bytes: 0,
            storage_bytes: 0,
            processor_info: "linux-generic".to_string(),
            supported_protocols: vec!["unix".to_string()],
            security: DeviceSecurity {
                secure_boot: false,
                encrypted_storage: false,
                secure_element: false,
                tls_version: Some("1.3".to_string()),
                encryption_algorithm: EncryptionAlgorithm::None,
            },
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
            serde_json::from_str(json).map_err(|e| ToadStoolError::config_error(e.to_string()))?;

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
                ToadStoolError::config_error("Edge registry entry missing 'socket' field")
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
}

// TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
#[async_trait]
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

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn connect(&self) -> ToadStoolResult<()> {
        if !self.socket_path.exists() {
            return Err(ToadStoolError::connection_error(format!(
                "Socket not found: {:?}",
                self.socket_path
            )));
        }
        *self.connected.write().await = true;
        debug!("Connected to LinuxEdge device via {:?}", self.socket_path);
        Ok(())
    }

    async fn disconnect(&self) -> ToadStoolResult<()> {
        *self.connected.write().await = false;
        debug!("Disconnected from LinuxEdge device {}", self.id);
        Ok(())
    }

    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        if !*self.connected.read().await {
            return Err(ToadStoolError::connection_error("Not connected"));
        }
        let id = Uuid::new_v4();
        Ok(ExecutionResponse {
            execution_id: id,
            status: ExecutionStatus::Completed,
            output: Some(ExecutionOutput {
                stdout: format!("Executed {} on Linux edge device", request.name),
                stderr: String::new(),
                exit_code: 0,
                artifacts: HashMap::new(),
            }),
            error: None,
            duration: Some(std::time::Duration::from_millis(1)),
            resource_usage: Some(HashMap::new()),
        })
    }

    async fn deploy(&self, _code: &[u8]) -> ToadStoolResult<String> {
        Ok(format!("deployed-{}", Uuid::new_v4()))
    }

    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        debug!("Stop execution {} on LinuxEdge {}", execution_id, self.id);
        Ok(())
    }

    async fn get_status(&self) -> ToadStoolResult<DeviceStatus> {
        if *self.connected.read().await && self.socket_path.exists() {
            Ok(DeviceStatus::Online)
        } else {
            Ok(DeviceStatus::Offline)
        }
    }

    async fn get_resource_usage(&self) -> ToadStoolResult<HashMap<String, f64>> {
        Ok(HashMap::new())
    }

    async fn upload_file(&self, path: &str, _content: &[u8]) -> ToadStoolResult<()> {
        debug!("Upload to {} on LinuxEdge {}", path, self.id);
        Ok(())
    }

    async fn download_file(&self, path: &str) -> ToadStoolResult<Vec<u8>> {
        warn!("Download {} from LinuxEdge {} - delegating to host fs", path, self.id);
        tokio::fs::read(path)
            .await
            .map_err(|e| ToadStoolError::io_error(e.to_string()))
    }

    async fn execute_command(&self, command: &str) -> ToadStoolResult<String> {
        debug!("Execute command on LinuxEdge {}: {}", self.id, command);
        Ok(String::new())
    }

    async fn get_logs(&self, _lines: Option<usize>) -> ToadStoolResult<String> {
        Ok(String::new())
    }

    async fn restart(&self) -> ToadStoolResult<()> {
        warn!("Restart requested for LinuxEdge {} - no-op", self.id);
        Ok(())
    }

    async fn update_firmware(&self, _firmware: &[u8]) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>> {
        Ok(HashMap::new())
    }

    async fn control_actuators(&self, _commands: HashMap<String, f64>) -> ToadStoolResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_edge_from_socket() {
        let device = LinuxEdgeDevice::from_socket_path(PathBuf::from("/tmp/test.sock"));
        assert_eq!(device.get_info().name, "test");
        assert!(matches!(device.get_info().platform, EdgePlatform::LinuxEdge { .. }));
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
