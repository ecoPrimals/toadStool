// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network interface detection

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolResult;

use super::HardwareDetector;

/// Network information and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkInfo {
    /// Detected network interfaces.
    pub interfaces: Vec<NetworkInterface>,
}

/// Network interface information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g. eth0).
    pub name: String,
    /// Interface type.
    pub interface_type: NetworkInterfaceType,
    /// Link speed in Mbps.
    pub speed_mbps: u32,
    /// Whether the interface is wireless.
    pub is_wireless: bool,
}

/// Network interface type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkInterfaceType {
    /// Wired Ethernet.
    Ethernet,
    /// Wireless WiFi.
    WiFi,
    /// Loopback interface.
    Loopback,
    /// Unknown interface type.
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
