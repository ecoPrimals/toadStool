// SPDX-License-Identifier: AGPL-3.0-or-later
//! USB discovery via Linux sysfs (`/sys/bus/usb/devices`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::warn;
use uuid::Uuid;

use toadstool::error::ToadStoolResult;
use toadstool::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeType,
};
use toadstool_common::constants::platform_paths::sysfs;

use crate::platforms::*;

use super::DiscoveryMethod;

/// USB Device Discovery Method
pub struct USBDiscovery {
    pub(super) vendor_filters: Vec<u16>,
    pub(super) product_filters: Vec<u16>,
}

#[async_trait::async_trait]
impl DiscoveryMethod for USBDiscovery {
    fn get_name(&self) -> &str {
        "USB Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        if !cfg!(target_os = "linux") {
            return Ok(Vec::new());
        }

        let base = Path::new(sysfs::BUS_USB_DEVICES);
        let entries = match std::fs::read_dir(base) {
            Ok(e) => e,
            Err(e) => {
                warn!(path = %base.display(), error = %e, "USB sysfs not readable; skipping USB discovery");
                return Ok(Vec::new());
            }
        };

        let mut out: Vec<Arc<dyn EdgeDevice>> = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let id_vendor = path.join("idVendor");
            let id_product = path.join("idProduct");
            if !id_vendor.is_file() || !id_product.is_file() {
                // Interface subdirectories (e.g. `1-2:1.0`) do not expose idVendor at this level.
                continue;
            }

            let vid = match parse_hex16_sysfs(&id_vendor) {
                Some(v) => v,
                None => {
                    warn!(path = %id_vendor.display(), "skipping USB device: invalid idVendor");
                    continue;
                }
            };
            let pid = match parse_hex16_sysfs(&id_product) {
                Some(p) => p,
                None => {
                    warn!(path = %id_product.display(), "skipping USB device: invalid idProduct");
                    continue;
                }
            };

            if !self.passes_filters(vid, pid) {
                continue;
            }

            let manufacturer = read_sysfs_optional_line(&path.join("manufacturer"));
            let product = read_sysfs_optional_line(&path.join("product"));

            out.push(Arc::new(UsbSysfsEdgeDevice::new(
                path,
                vid,
                pid,
                manufacturer,
                product,
            )));
        }

        Ok(out)
    }

    async fn is_available(&self) -> bool {
        cfg!(target_os = "linux") && Path::new(sysfs::BUS_USB_DEVICES).is_dir()
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec!["USB Device".to_string()]
    }
}

impl USBDiscovery {
    fn passes_filters(&self, vid: u16, pid: u16) -> bool {
        let vf = &self.vendor_filters;
        let pf = &self.product_filters;
        if !vf.is_empty() && !vf.contains(&vid) {
            return false;
        }
        if !pf.is_empty() && !pf.contains(&pid) {
            return false;
        }
        true
    }
}

fn read_sysfs_optional_line(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_hex16_sysfs(path: &Path) -> Option<u16> {
    let s = read_sysfs_optional_line(path)?;
    u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// USB device discovered from sysfs (descriptor-level; no direct USB I/O).
struct UsbSysfsEdgeDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl UsbSysfsEdgeDevice {
    fn new(
        sysfs_path: PathBuf,
        vid: u16,
        pid: u16,
        manufacturer: Option<String>,
        product: Option<String>,
    ) -> Self {
        let id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("usb:{}:{}:{}", sysfs_path.display(), vid, pid).as_bytes(),
        );
        let mfg = manufacturer.unwrap_or_else(|| "unknown".to_string());
        let prod = product.unwrap_or_else(|| "unknown".to_string());
        let name = format!("USB {vid:04x}:{pid:04x} {mfg} {prod}");

        let info = EdgeDeviceInfo {
            id,
            name,
            platform: EdgePlatform::Microcontroller {
                architecture: MicrocontrollerArch::ARM,
                vendor: mfg,
                model: format!("{prod} (usb {vid:04x}:{pid:04x})"),
            },
            capabilities: vec!["usb".to_string(), "sysfs".to_string()],
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
                connection_type: ConnectionType::USB,
                address: sysfs_path.display().to_string(),
                port: None,
                protocol: "usb-sysfs".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Unknown,
            last_seen: std::time::SystemTime::now(),
        };

        Self { id, info }
    }
}

#[async_trait::async_trait]
impl EdgeDevice for UsbSysfsEdgeDevice {
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
        Path::new(&self.info.connection_info.address).exists()
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
                stdout: Some("usb-sysfs discovery device (no workload execution)".to_string()),
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
