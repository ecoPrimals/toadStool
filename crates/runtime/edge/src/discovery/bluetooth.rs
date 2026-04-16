// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bluetooth discovery: adapter presence via sysfs, remote devices via `/sys/bus/bluetooth/devices`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::{debug, warn};
use uuid::Uuid;

use toadstool::error::ToadStoolResult;
use toadstool::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeType,
};
use toadstool_common::constants::platform_paths::sysfs;

use crate::platforms::*;

use super::DiscoveryMethod;

/// Bluetooth Discovery Method
pub struct BluetoothDiscovery {
    pub(super) scan_duration: std::time::Duration,
    pub(super) device_types: Vec<String>,
}

#[async_trait::async_trait]
impl DiscoveryMethod for BluetoothDiscovery {
    fn get_name(&self) -> &str {
        "Bluetooth Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        if !cfg!(target_os = "linux") {
            return Ok(Vec::new());
        }

        if !self.is_available().await {
            debug!("No Bluetooth adapters found via sysfs");
            return Ok(Vec::new());
        }

        let bus = Path::new(sysfs::BUS_BLUETOOTH_DEVICES);
        let entries = match std::fs::read_dir(bus) {
            Ok(e) => e,
            Err(e) => {
                warn!(
                    path = %bus.display(),
                    error = %e,
                    "Bluetooth bus sysfs not readable; skipping remote device enumeration"
                );
                return Ok(Vec::new());
            }
        };

        let mut out: Vec<Arc<dyn EdgeDevice>> = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            // Adapters are `hciN`; remote devices are typically `hciN:AA_BB_CC_DD_EE_FF` (or `hciN:AA:BB:...` on some kernels).
            if !name_str.contains(':') {
                continue;
            }

            let path = entry.path();
            let address = match read_sysfs_trimmed(&path.join("address")) {
                Some(a) => a,
                None => {
                    warn!(path = %path.display(), "bluetooth device entry missing address");
                    continue;
                }
            };

            let friendly = read_sysfs_trimmed(&path.join("name"))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| address.clone());

            out.push(Arc::new(BluetoothSysfsEdgeDevice::new(
                path, address, friendly,
            )));
        }

        let _ = self.scan_duration;
        let _ = &self.device_types;

        Ok(out)
    }

    async fn is_available(&self) -> bool {
        if !cfg!(target_os = "linux") {
            return false;
        }
        let bt_class = Path::new(sysfs::CLASS_BLUETOOTH);
        if !bt_class.exists() {
            return false;
        }
        match std::fs::read_dir(bt_class) {
            Ok(entries) => entries.filter_map(|e| e.ok()).next().is_some(),
            Err(_) => false,
        }
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Bluetooth Device".to_string(),
            "ESP32 Bluetooth".to_string(),
        ]
    }
}

fn read_sysfs_trimmed(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Remote Bluetooth device visible in kernel sysfs (paired/connected path varies by stack).
struct BluetoothSysfsEdgeDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
    sysfs_path: PathBuf,
}

impl BluetoothSysfsEdgeDevice {
    fn new(sysfs_path: PathBuf, address: String, friendly_name: String) -> Self {
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("bt:{}:{address}", sysfs_path.display()).as_bytes(),
        );
        let name = format!("Bluetooth {friendly_name} ({address})");

        let info = EdgeDeviceInfo {
            id,
            name,
            platform: EdgePlatform::Microcontroller {
                architecture: MicrocontrollerArch::RISCV,
                vendor: "bluetooth".to_string(),
                model: friendly_name.clone(),
            },
            capabilities: vec!["bluetooth".to_string(), "sysfs".to_string()],
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
                connection_type: ConnectionType::Bluetooth,
                address,
                port: None,
                protocol: "bluetooth-sysfs".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Unknown,
            last_seen: std::time::SystemTime::now(),
        };

        Self {
            id,
            info,
            sysfs_path,
        }
    }
}

#[async_trait::async_trait]
impl EdgeDevice for BluetoothSysfsEdgeDevice {
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
        self.sysfs_path.exists()
    }

    async fn connect(&self) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn disconnect(&self) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        Ok(ExecutionResponse {
            execution_id: request.execution_id,
            status: ExecutionStatus::Success,
            output: ExecutionOutput {
                stdout: Some("bluetooth-sysfs discovery device (no workload execution)".to_string()),
                ..ExecutionOutput::default()
            },
            metrics: toadstool::RuntimeMetrics::default(),
            duration: std::time::Duration::ZERO,
            runtime_used: RuntimeType::Native,
            warnings: Vec::new(),
        })
    }

    async fn deploy(&self, _code: &[u8]) -> ToadStoolResult<String> {
        Ok(String::new())
    }

    async fn stop_execution(&self, _execution_id: Uuid) -> ToadStoolResult<()> {
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
        Ok(HashMap::new())
    }

    async fn upload_file(&self, _path: &str, _content: &[u8]) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn download_file(&self, _path: &str) -> ToadStoolResult<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn execute_command(&self, _command: &str) -> ToadStoolResult<String> {
        Ok(String::new())
    }

    async fn get_logs(&self, _lines: Option<usize>) -> ToadStoolResult<String> {
        Ok(String::new())
    }

    async fn restart(&self) -> ToadStoolResult<()> {
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
