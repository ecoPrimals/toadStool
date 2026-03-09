// SPDX-License-Identifier: AGPL-3.0-only
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

/// Bind to any interface with OS-assigned port (mDNS socket)
const BIND_ANY: &str = "0.0.0.0:0";

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
    pub fn new() -> Self {
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

        // Try HTTP registry (centralized discovery)
        if let Some(service) = self
            .try_registry_discovery(capability, capability_name)
            .await?
        {
            debug!("Found {} via registry", capability_name);
            return Ok(Some(service));
        }

        debug!("Service with {} capability not found", capability_name);
        Ok(None)
    }

    /// Try filesystem-based discovery (biomeOS runtime directory)
    ///
    /// Scans the biomeOS runtime directory for capability socket files.
    #[allow(clippy::unused_async)] // Sync filesystem scan; async for API consistency
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

    /// Try HTTP registry discovery
    ///
    /// Queries a centralized service registry via HTTP API.
    #[allow(clippy::unused_async)] // Deprecated; returns None; async for API consistency
    async fn try_registry_discovery(
        &self,
        capability: &str,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        // DEEP DEBT: HTTP registry discovery removed - use Unix socket capability discovery!
        tracing::debug!(
            "HTTP registry discovery deprecated for {} ({}) - use Unix socket discovery",
            capability_name,
            capability
        );

        // Return None to allow fallback to other discovery methods
        Ok(None)
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

impl Default for ServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_discovery_creation() {
        let discovery = ServiceDiscovery::new();
        let default_discovery = ServiceDiscovery::default();
        // Both should work; discover returns None for unknown capability
        let result = discovery.discover_by_capability("test", "unknown").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        let _ = default_discovery;
    }

    #[tokio::test]
    async fn test_mdns_query_packet_format() {
        let discovery = ServiceDiscovery::new();
        let query = discovery.build_mdns_query("test-service");

        assert!(query.len() >= 12);
        assert_eq!(query[0], 0x00);
        assert_eq!(query[1], 0x00);
        assert_eq!(query[4], 0x00);
        assert_eq!(query[5], 0x01);
    }

    #[tokio::test]
    async fn test_discovery_handles_timeout() {
        let discovery = ServiceDiscovery::new();
        let result = discovery
            .discover_by_capability("nonexistent", "test")
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_parse_mdns_response_too_short() {
        let discovery = ServiceDiscovery::new();
        let result = discovery.parse_mdns_response(&[0u8; 8], "test");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_parse_mdns_response_query_not_response() {
        let discovery = ServiceDiscovery::new();
        // QR bit = 0 (query)
        let mut data = [0u8; 20];
        data[2] = 0x00; // flags: standard query
        data[4] = 0x00;
        data[5] = 0x01; // 1 question
        let result = discovery.parse_mdns_response(&data, "test");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_parse_mdns_response_with_a_record() {
        let discovery = ServiceDiscovery::new();
        // Minimal DNS response: header + question + 1 A record
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00]); // id
        data.extend_from_slice(&[0x80, 0x00]); // flags (QR=1)
        data.extend_from_slice(&[0x00, 0x01]); // 1 question
        data.extend_from_slice(&[0x00, 0x01]); // 1 answer
        data.extend_from_slice(&[0x00, 0x00]); // 0 auth
        data.extend_from_slice(&[0x00, 0x00]); // 0 additional
                                               // Question: _test._tcp.local (simplified)
        data.push(5);
        data.extend_from_slice(b"_test");
        data.push(4);
        data.extend_from_slice(b"_tcp");
        data.push(5);
        data.extend_from_slice(b"local");
        data.push(0);
        data.extend_from_slice(&[0x00, 0x0C]); // PTR
        data.extend_from_slice(&[0x00, 0x01]); // IN
                                               // Answer: instance._test._tcp.local, A record, 127.0.0.1
        data.push(8);
        data.extend_from_slice(b"instance");
        data.push(5);
        data.extend_from_slice(b"_test");
        data.push(4);
        data.extend_from_slice(b"_tcp");
        data.push(5);
        data.extend_from_slice(b"local");
        data.push(0);
        data.extend_from_slice(&[0x00, 0x01]); // A
        data.extend_from_slice(&[0x00, 0x01]); // IN
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]); // TTL 60
        data.extend_from_slice(&[0x00, 0x04]); // rdlength 4
        data.extend_from_slice(&[127, 0, 0, 1]); // 127.0.0.1

        let result = discovery.parse_mdns_response(&data, "toadstool");
        assert!(result.is_ok());
        let endpoint = result.unwrap();
        assert!(endpoint.is_some());
        let ep = endpoint.unwrap();
        assert_eq!(ep.name, "toadstool");
        assert!(ep.endpoint.contains("127.0.0.1"));
        assert!(ep.endpoint.contains("8084"));
    }

    #[tokio::test]
    async fn test_parse_mdns_response_no_a_record() {
        let discovery = ServiceDiscovery::new();
        // Response with PTR only (no A record)
        let mut data = Vec::new();
        data.extend_from_slice(&[0x00, 0x00, 0x80, 0x00]);
        data.extend_from_slice(&[0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
        data.push(5);
        data.extend_from_slice(b"_test");
        data.push(4);
        data.extend_from_slice(b"_tcp");
        data.push(5);
        data.extend_from_slice(b"local");
        data.push(0);
        data.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]);
        // Answer: PTR record (type 12), rdata is a name
        data.push(8);
        data.extend_from_slice(b"instance");
        data.push(5);
        data.extend_from_slice(b"_test");
        data.push(4);
        data.extend_from_slice(b"_tcp");
        data.push(5);
        data.extend_from_slice(b"local");
        data.push(0);
        data.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]);
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x3C]);
        data.extend_from_slice(&[0x00, 0x0F]); // rdlength 15
        data.push(8);
        data.extend_from_slice(b"myhost");
        data.push(5);
        data.extend_from_slice(b"local");
        data.push(0);

        let result = discovery.parse_mdns_response(&data, "test");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
