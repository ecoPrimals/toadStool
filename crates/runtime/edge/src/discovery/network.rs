// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP port scanning over configured subnets.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

use toadstool::error::ToadStoolResult;
use toadstool_common::constants::platform_paths::procfs;

use crate::platforms::*;

use super::DiscoveryMethod;

/// Well-known port numbers used during edge device identification.
mod well_known_ports {
    pub const SSH: u16 = 22;
    pub const HTTP: u16 = 80;
    pub const HTTP_ALT: u16 = 8080;
}

/// Network Discovery Method
pub struct NetworkDiscovery {
    pub(super) scan_range: Vec<IpAddr>,
    pub(super) ports: Vec<u16>,
    pub(super) timeout: Duration,
}

impl DiscoveryMethod for NetworkDiscovery {
    fn get_name(&self) -> &str {
        "Network Discovery"
    }

    fn discover(&self) -> super::DiscoveryFuture<'_> {
        Box::pin(async move {
            let mut devices = Vec::new();

            // Scan network ranges
            for ip in &self.scan_range {
                let scan_devices = self.scan_network_range(*ip).await?;
                devices.extend(scan_devices);
            }

            Ok(devices)
        })
    }

    fn is_available(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
        Box::pin(async { true })
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
    async fn scan_network_range(
        &self,
        base_ip: IpAddr,
    ) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();

        // For now, implement a simple ping-based scan
        // In a real implementation, this would use more sophisticated network scanning

        match base_ip {
            IpAddr::V4(ipv4) => {
                let base_octets = ipv4.octets();

                // Scan /24 network
                for host in 1..255 {
                    let target_ip =
                        Ipv4Addr::new(base_octets[0], base_octets[1], base_octets[2], host);

                    if let Some(device) = self.probe_network_device(IpAddr::V4(target_ip)).await {
                        devices.push(device);
                    }
                }
            }
            IpAddr::V6(_) => {
                if !cfg!(target_os = "linux") {
                    debug!("IPv6 discovery skipped (not Linux)");
                } else {
                    devices.extend(self.scan_ipv6_link_local_sysfs().await);
                }
            }
        }

        Ok(devices)
    }

    /// Enumerates IPv6 link-local addresses from [`procfs::NET_IF_INET6`] and probes configured ports
    /// using [`SocketAddrV6`] interface scope (required for link-local TCP on Linux).
    async fn scan_ipv6_link_local_sysfs(&self) -> Vec<Arc<dyn EdgeDevice>> {
        let mut found = Vec::new();
        let path = Path::new(procfs::NET_IF_INET6);
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "cannot read IPv6 interface table");
                return found;
            }
        };

        let mut seen_iface: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in contents.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let addr_hex = parts[0];
            if !addr_hex.starts_with("fe80") {
                continue;
            }
            let ifname = match parts.last() {
                Some(n) => (*n).to_string(),
                None => continue,
            };
            if ifname == "lo" {
                continue;
            }
            if !seen_iface.insert(ifname.clone()) {
                continue;
            }

            let Some(scope_id) = read_net_ifindex(&ifname) else {
                warn!(interface = %ifname, "skipping IPv6 probe: no ifindex in sysfs");
                continue;
            };

            let Some(ip) = ipv6_from_proc_hex32(addr_hex) else {
                warn!(addr_hex, "skipping IPv6 line: parse error");
                continue;
            };

            debug!(
                interface = %ifname,
                %ip,
                scope_id,
                "IPv6 link-local candidate from /proc/net/if_inet6"
            );

            for &port in &self.ports {
                let sa = SocketAddr::V6(SocketAddrV6::new(ip, port, 0, scope_id));
                let connect_ok = matches!(
                    tokio::time::timeout(self.timeout, tokio::net::TcpStream::connect(sa)).await,
                    Ok(Ok(_))
                );
                if connect_ok
                    && let Some(device) = self.identify_network_device(IpAddr::V6(ip), port).await
                {
                    found.push(device);
                    break;
                }
            }
        }

        found
    }

    async fn probe_network_device(&self, ip: IpAddr) -> Option<Arc<dyn EdgeDevice>> {
        // Try to connect to common ports
        for &port in &self.ports {
            let socket_addr = SocketAddr::new(ip, port);

            // Try to connect
            if let Ok(_stream) =
                tokio::time::timeout(self.timeout, tokio::net::TcpStream::connect(socket_addr))
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
        match port {
            well_known_ports::SSH => {
                debug!("Found SSH service on {}:{}", ip, port);
                None
            }
            well_known_ports::HTTP | well_known_ports::HTTP_ALT => {
                debug!("Found HTTP service on {}:{}", ip, port);
                None
            }
            _ => None,
        }
    }
}

fn read_net_ifindex(ifname: &str) -> Option<u32> {
    let path = format!("/sys/class/net/{ifname}/ifindex");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn ipv6_from_proc_hex32(s: &str) -> Option<Ipv6Addr> {
    if s.len() != 32 {
        return None;
    }
    let mut segs = [0u16; 8];
    for i in 0..8 {
        segs[i] = u16::from_str_radix(&s[i * 4..(i + 1) * 4], 16).ok()?;
    }
    Some(Ipv6Addr::new(
        segs[0], segs[1], segs[2], segs[3], segs[4], segs[5], segs[6], segs[7],
    ))
}
