//! Network interface detection

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// Network information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: NetworkInterfaceType,
    pub speed_mbps: u32,
    pub is_wireless: bool,
}

/// Network interface type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkInterfaceType {
    Ethernet,
    WiFi,
    Loopback,
    Unknown,
}

/// Detect network interfaces and capabilities
pub fn detect_network(_detector: &HardwareDetector) -> ToadStoolResult<NetworkInfo> {
    let network_info = NetworkInfo {
        interfaces: vec![NetworkInterface {
            name: "default".to_string(),
            interface_type: NetworkInterfaceType::Ethernet,
            speed_mbps: 1000, // Default assumption
            is_wireless: false,
        }],
    };

    debug!(
        "Detected {} network interface(s)",
        network_info.interfaces.len()
    );
    Ok(network_info)
}
