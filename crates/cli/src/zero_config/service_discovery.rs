// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service discovery implementations
//!
//! Provides multiple discovery mechanisms for finding ecosystem services:
//! - Filesystem (biomeOS runtime directory - capability socket files)
//! - mDNS/Multicast DNS (local network)
//! - DNS-SD/Service Discovery (DNS-based)
//! - Localhost fallback (development)

use crate::Result;
use std::net::IpAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use tracing::debug;

use super::types::ServiceEndpoint;
use toadstool_common::constants::network::BIND_ANY;

/// mDNS multicast group address (RFC 6762)
const MDNS_MULTICAST_ADDR: &str = "224.0.0.251";

/// mDNS port (RFC 6762)
const MDNS_PORT: u16 = 5353;

/// Service discovery coordinator
pub struct ServiceDiscovery {
    /// Timeout for discovery attempts
    discovery_timeout: Duration,
}

impl ServiceDiscovery {
    /// Create new service discovery coordinator
    pub const fn new() -> Self {
        Self {
            discovery_timeout: Duration::from_secs(2),
        }
    }

    /// Discover service by capability using all available methods
    pub async fn discover_by_capability(
        &self,
        capability: &str,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        debug!("Discovering service with {} capability", capability_name);

        // Try filesystem first (biomeOS runtime directory - primal socket files)
        if let Some(service) = self.try_filesystem_discovery(capability_name).await? {
            debug!("Found {} via filesystem (biomeOS sockets)", capability_name);
            return Ok(Some(service));
        }

        // Try mDNS (local network discovery)
        if let Some(service) = self.try_mdns_discovery(capability_name).await? {
            debug!("Found {} via mDNS", capability_name);
            return Ok(Some(service));
        }

        // Try DNS-SD (DNS-based service discovery)
        if let Some(service) = self.try_dns_sd_discovery(capability_name).await? {
            debug!("Found {} via DNS-SD", capability_name);
            return Ok(Some(service));
        }

        // Try biomeOS socket directory scan (wildcard matching for capability variants)
        if let Some(service) = self
            .try_biomeos_directory_scan(capability, capability_name)
            .await?
        {
            debug!("Found {} via biomeOS directory scan", capability_name);
            return Ok(Some(service));
        }

        debug!("Service with {} capability not found", capability_name);
        Ok(None)
    }

    /// Try filesystem-based discovery (biomeOS runtime directory)
    ///
    /// Scans the biomeOS runtime directory for capability socket files.
    #[expect(
        clippy::unused_async,
        reason = "async signature required by trait/interface"
    )] // Sync filesystem scan; async for API consistency
    async fn try_filesystem_discovery(
        &self,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        debug!("Attempting filesystem discovery for {}", capability_name);

        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability(capability_name);

        if socket_path.exists() {
            let endpoint = format!("unix://{}", socket_path.display());
            debug!("Found capability socket: {}", endpoint);

            return Ok(Some(ServiceEndpoint {
                name: capability_name.to_string(),
                endpoint,
                version: "1.0.0".to_string(),
                status: "discovered".to_string(),
                auth_required: false,
                discovered_at: std::time::SystemTime::now(),
            }));
        }

        Ok(None)
    }

    /// Try mDNS/Multicast DNS discovery
    ///
    /// mDNS allows services to announce themselves on local networks
    /// without requiring a central DNS server.
    async fn try_mdns_discovery(&self, capability_name: &str) -> Result<Option<ServiceEndpoint>> {
        debug!("Attempting mDNS discovery for {}", capability_name);

        let multicast_addr: IpAddr = MDNS_MULTICAST_ADDR.parse()?;
        let mdns_port = MDNS_PORT;

        // Create UDP socket for mDNS
        let socket = match UdpSocket::bind(BIND_ANY).await {
            Ok(s) => s,
            Err(e) => {
                debug!("Failed to create mDNS socket: {}", e);
                return Ok(None);
            }
        };

        // Build mDNS query packet
        let query = self.build_mdns_query(capability_name);

        // Send query to multicast address
        if let Err(e) = socket.send_to(&query, (multicast_addr, mdns_port)).await {
            debug!("Failed to send mDNS query: {}", e);
            return Ok(None);
        }

        // Wait for responses (with timeout)
        let mut buf = vec![0u8; 1500];
        match timeout(self.discovery_timeout, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, addr))) => {
                debug!("Received mDNS response from {}", addr);
                self.parse_mdns_response(&buf[..len], capability_name)
            }
            Ok(Err(e)) => {
                debug!("Error receiving mDNS response: {}", e);
                Ok(None)
            }
            Err(_) => {
                debug!("mDNS discovery timeout for {}", capability_name);
                Ok(None)
            }
        }
    }

    /// Try DNS-SD (DNS Service Discovery)
    ///
    /// DNS-SD uses standard DNS records (SRV, TXT, PTR) to advertise services.
    async fn try_dns_sd_discovery(&self, capability_name: &str) -> Result<Option<ServiceEndpoint>> {
        debug!("Attempting DNS-SD discovery for {}", capability_name);

        // DNS-SD service name format: _service._proto.domain
        let service_name = format!("_{capability_name}._tcp.local");

        // Query DNS for SRV records
        // Try to resolve the service name
        let addrs_result = tokio::net::lookup_host(service_name.as_str()).await;

        match addrs_result {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    debug!("Found {} at {} via DNS-SD", capability_name, addr);
                    return Ok(Some(ServiceEndpoint {
                        name: capability_name.to_string(),
                        endpoint: format!("http://{addr}"),
                        version: "unknown".to_string(),
                        status: "discovered".to_string(),
                        auth_required: false,
                        discovered_at: std::time::SystemTime::now(),
                    }));
                }
                Ok(None)
            }
            Err(e) => {
                debug!("DNS-SD lookup failed for {}: {}", service_name, e);
                Ok(None)
            }
        }
    }

    /// Try biomeOS socket directory scan.
    ///
    /// Scans `$XDG_RUNTIME_DIR/biomeos/` for sockets matching the capability name
    /// prefix. Handles variant naming (e.g. `coralreef-core-default.sock` for
    /// capability `coralreef`).
    async fn try_biomeos_directory_scan(
        &self,
        _capability: &str,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        let name = capability_name.to_string();
        tokio::task::spawn_blocking(move || scan_biomeos_dir(&name))
            .await
            .map_err(|e| crate::CliError::Other(format!("spawn_blocking failed: {e}")))?
    }

    /// Build mDNS query packet
    ///
    /// Constructs a DNS query packet for mDNS service discovery.
    fn build_mdns_query(&self, service_name: &str) -> Vec<u8> {
        let mut packet = Vec::new();

        // Transaction ID (2 bytes) - use 0 for queries
        packet.extend_from_slice(&[0x00, 0x00]);

        // Flags (2 bytes) - standard query
        packet.extend_from_slice(&[0x00, 0x00]);

        // Questions (2 bytes) - 1 question
        packet.extend_from_slice(&[0x00, 0x01]);

        // Answer RRs (2 bytes) - 0 answers
        packet.extend_from_slice(&[0x00, 0x00]);

        // Authority RRs (2 bytes) - 0
        packet.extend_from_slice(&[0x00, 0x00]);

        // Additional RRs (2 bytes) - 0
        packet.extend_from_slice(&[0x00, 0x00]);

        // Question: _service._tcp.local
        let qname = format!("_{service_name}._tcp.local");
        for label in qname.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }
        packet.push(0x00); // End of name

        // Type: PTR (12)
        packet.extend_from_slice(&[0x00, 0x0C]);

        // Class: IN (1)
        packet.extend_from_slice(&[0x00, 0x01]);

        packet
    }

    /// Parse mDNS response packet
    ///
    /// Parses common mDNS response format: DNS header + question + answer/additional records.
    /// Extracts A record (IPv4) or AAAA record (IPv6) to build ServiceEndpoint.
    pub(crate) fn parse_mdns_response(
        &self,
        data: &[u8],
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        if data.len() < 12 {
            return Ok(None);
        }

        // Check if it's a response (QR bit set)
        if data[2] & 0x80 == 0 {
            return Ok(None);
        }

        let num_questions = u16::from_be_bytes([data[4], data[5]]) as usize;
        let num_answers = u16::from_be_bytes([data[6], data[7]]) as usize;
        let num_auth = u16::from_be_bytes([data[8], data[9]]) as usize;
        let num_additional = u16::from_be_bytes([data[10], data[11]]) as usize;

        let mut pos = 12;

        // Skip question section
        for _ in 0..num_questions {
            pos = skip_dns_name(data, pos)?;
            if pos + 4 > data.len() {
                return Ok(None);
            }
            pos += 4; // type + class
        }

        // Parse answer, authority, additional records
        let total_records = num_answers + num_auth + num_additional;
        for _ in 0..total_records {
            let (new_pos, ip_opt) = parse_rr_for_ip(data, pos)?;
            pos = new_pos;
            if let Some(ip) = ip_opt {
                debug!("Parsed mDNS A/AAAA record for {}: {}", capability_name, ip);
                return Ok(Some(ServiceEndpoint {
                    name: capability_name.to_string(),
                    endpoint: format!("http://{}:{}", ip, toadstool_config::ports::daemon_port()),
                    version: "1.0.0".to_string(),
                    status: "discovered".to_string(),
                    auth_required: false,
                    discovered_at: std::time::SystemTime::now(),
                }));
            }
        }

        debug!(
            "Received mDNS response for {} (no A/AAAA record)",
            capability_name
        );
        Ok(None)
    }
}

/// Skip a DNS name (labels or compression pointer), return position after name
fn skip_dns_name(data: &[u8], start: usize) -> Result<usize, std::io::Error> {
    let mut pos = start;
    loop {
        if pos >= data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated DNS name",
            ));
        }
        let label_len = data[pos] as usize;
        pos += 1;
        if label_len == 0 {
            break;
        }
        if label_len & 0xC0 == 0xC0 {
            // Compression pointer
            if pos + 1 >= data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "truncated compression pointer",
                ));
            }
            pos += 2;
            break;
        }
        pos += label_len;
    }
    Ok(pos)
}

/// Parse one RR, return (new_pos, Some(ip) if A/AAAA record else None)
fn parse_rr_for_ip(data: &[u8], start: usize) -> Result<(usize, Option<IpAddr>), std::io::Error> {
    let pos = skip_dns_name(data, start)?;
    if pos + 10 > data.len() {
        return Ok((data.len(), None));
    }
    let rr_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
    let rdlength = u16::from_be_bytes([data[pos + 8], data[pos + 9]]) as usize;
    let rdata_start = pos + 10;
    let rdata_end = rdata_start + rdlength;

    if rdata_end > data.len() {
        return Ok((data.len(), None));
    }

    let ip_opt = match rr_type {
        1 if rdlength == 4 => {
            // A record
            let octets: [u8; 4] = data[rdata_start..rdata_end].try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "A record length")
            })?;
            Some(IpAddr::V4(std::net::Ipv4Addr::from(octets)))
        }
        28 if rdlength == 16 => {
            // AAAA record
            let octets: [u8; 16] = data[rdata_start..rdata_end].try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "AAAA record length")
            })?;
            Some(IpAddr::V6(std::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    };

    Ok((rdata_end, ip_opt))
}

/// Blocking directory scan for biomeOS capability sockets.
///
/// Designed to run via `spawn_blocking` so directory iteration
/// never stalls the async runtime.
fn scan_biomeos_dir(capability_name: &str) -> Result<Option<ServiceEndpoint>> {
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    if !biomeos_dir.exists() {
        return Ok(None);
    }

    let prefix = capability_name.to_lowercase();
    let entries = match std::fs::read_dir(&biomeos_dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_match = path.extension().and_then(|e| e.to_str()) == Some("sock")
            && path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(&prefix));
        if is_match && path.exists() {
            let endpoint = format!("unix://{}", path.display());
            tracing::debug!(
                "biomeOS directory scan found {} socket: {}",
                capability_name,
                endpoint
            );
            return Ok(Some(ServiceEndpoint {
                name: capability_name.to_string(),
                endpoint,
                version: "1.0.0".to_string(),
                status: "discovered".to_string(),
                auth_required: false,
                discovered_at: std::time::SystemTime::now(),
            }));
        }
    }

    Ok(None)
}

impl Default for ServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "service_discovery_tests.rs"]
mod tests;
