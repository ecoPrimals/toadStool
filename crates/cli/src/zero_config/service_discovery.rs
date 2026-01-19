//! Service discovery implementations
//!
//! Provides multiple discovery mechanisms for finding ecosystem services:
//! - mDNS/Multicast DNS (local network)
//! - DNS-SD/Service Discovery (DNS-based)
//! - HTTP Registry (centralized registry)
//! - Localhost fallback (development)

use anyhow::Result;
use std::net::IpAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;
use tracing::debug;

use super::types::ServiceEndpoint;

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

        // Try mDNS first (local network discovery)
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

    /// Try mDNS/Multicast DNS discovery
    ///
    /// mDNS allows services to announce themselves on local networks
    /// without requiring a central DNS server.
    async fn try_mdns_discovery(&self, capability_name: &str) -> Result<Option<ServiceEndpoint>> {
        debug!("Attempting mDNS discovery for {}", capability_name);

        // mDNS uses multicast address 224.0.0.251:5353
        let multicast_addr: IpAddr = "224.0.0.251".parse()?;
        let mdns_port = 5353;

        // Create UDP socket for mDNS
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
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
        let service_name = format!("_{}._tcp.local", capability_name);

        // Query DNS for SRV records
        // Try to resolve the service name
        let addrs_result = tokio::net::lookup_host(service_name.as_str()).await;

        match addrs_result {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    debug!("Found {} at {} via DNS-SD", capability_name, addr);
                    return Ok(Some(ServiceEndpoint {
                        name: capability_name.to_string(),
                        endpoint: format!("http://{}", addr),
                        version: "unknown".to_string(),
                        status: "discovered".to_string(),
                        auth_required: false,
                        discovered_at: chrono::Utc::now(),
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
        let qname = format!("_{}._tcp.local", service_name);
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
    fn parse_mdns_response(
        &self,
        data: &[u8],
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        // Simplified mDNS response parsing
        // In production, use a proper DNS library

        if data.len() < 12 {
            return Ok(None);
        }

        // Check if it's a response (QR bit set)
        if data[2] & 0x80 == 0 {
            return Ok(None);
        }

        // For now, return a placeholder
        // Full implementation would parse DNS records
        debug!("Received mDNS response for {}", capability_name);
        Ok(None)
    }
}

impl Default for ServiceDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry service response format
#[derive(serde::Deserialize)]
// HTTP registry removed - struct kept for reference but unused
#[allow(dead_code)]
struct RegistryService {
    endpoint: String,
    version: String,
    requires_auth: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_discovery_creation() {
        let discovery = ServiceDiscovery::new();
        assert_eq!(discovery.discovery_timeout, Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_mdns_query_packet_format() {
        let discovery = ServiceDiscovery::new();
        let query = discovery.build_mdns_query("test-service");

        // Should have at least the DNS header (12 bytes)
        assert!(query.len() >= 12);

        // Check DNS header format
        assert_eq!(query[0], 0x00); // Transaction ID high byte
        assert_eq!(query[1], 0x00); // Transaction ID low byte
        assert_eq!(query[4], 0x00); // Questions high byte
        assert_eq!(query[5], 0x01); // Questions low byte (1 question)
    }

    #[tokio::test]
    async fn test_discovery_handles_timeout() {
        let discovery = ServiceDiscovery::new();

        // Try to discover non-existent service (should timeout gracefully)
        let result = discovery
            .discover_by_capability("nonexistent", "test")
            .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
