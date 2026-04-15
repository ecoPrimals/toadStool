// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP port scanning over configured subnets.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use toadstool::error::ToadStoolResult;

use crate::platforms::*;

use super::DiscoveryMethod;

/// Network Discovery Method
pub struct NetworkDiscovery {
    pub(super) scan_range: Vec<IpAddr>,
    pub(super) ports: Vec<u16>,
    pub(super) timeout: Duration,
}

#[async_trait::async_trait]
impl DiscoveryMethod for NetworkDiscovery {
    fn get_name(&self) -> &str {
        "Network Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();

        // Scan network ranges
        for ip in &self.scan_range {
            let scan_devices = self.scan_network_range(*ip).await?;
            devices.extend(scan_devices);
        }

        Ok(devices)
    }

    async fn is_available(&self) -> bool {
        // Check if network interface is available
        true
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Raspberry Pi".to_string(),
            "Linux Edge".to_string(),
            "ESP32".to_string(),
            "Network Device".to_string(),
        ]
    }
}

impl NetworkDiscovery {
    async fn scan_network_range(&self, base_ip: IpAddr) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();

        // For now, implement a simple ping-based scan
        // In a real implementation, this would use more sophisticated network scanning

        match base_ip {
            IpAddr::V4(ipv4) => {
                let base_octets = ipv4.octets();

                // Scan /24 network
                for host in 1..255 {
                    let target_ip = Ipv4Addr::new(
                        base_octets[0],
                        base_octets[1],
                        base_octets[2],
                        host,
                    );

                    if let Some(device) = self.probe_network_device(IpAddr::V4(target_ip)).await {
                        devices.push(device);
                    }
                }
            }
            IpAddr::V6(_) => {
                // IPv6 scanning not implemented yet
                debug!("IPv6 scanning not yet implemented");
            }
        }

        Ok(devices)
    }

    async fn probe_network_device(&self, ip: IpAddr) -> Option<Arc<dyn EdgeDevice>> {
        // Try to connect to common ports
        for &port in &self.ports {
            let socket_addr = SocketAddr::new(ip, port);

            // Try to connect
            if let Ok(_stream) = tokio::time::timeout(
                self.timeout,
                tokio::net::TcpStream::connect(socket_addr),
            )
            .await
            {
                // Device is reachable, try to identify it
                if let Some(device) = self.identify_network_device(ip, port).await {
                    return Some(device);
                }
            }
        }

        None
    }

    async fn identify_network_device(&self, ip: IpAddr, port: u16) -> Option<Arc<dyn EdgeDevice>> {
        // Try to identify device type based on open ports and responses
        match port {
            22 => {
                // SSH port - likely Linux-based edge device
                debug!("Found SSH service on {}:{}", ip, port);
                // Could be Raspberry Pi or other Linux edge device
                // Implementation needed for RaspberryPiDevice
                None
            }
            80 | 8080 => {
                // HTTP port - could be ESP32 or other web-enabled device
                debug!("Found HTTP service on {}:{}", ip, port);
                None
            }
            _ => None,
        }
    }
}
