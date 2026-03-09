// SPDX-License-Identifier: AGPL-3.0-only
//! `ServiceEndpoint` URL parsing extension

use std::collections::HashMap;

use crate::primal_identity::ServiceEndpoint;

use super::types::{DiscoveryError, DiscoveryResult};

impl ServiceEndpoint {
    /// Create endpoint from URL string
    ///
    /// # Errors
    ///
    /// Returns [`DiscoveryError`] if the URL format is invalid or missing required parts.
    pub fn from_url_string(url: &str) -> DiscoveryResult<Self> {
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() != 2 {
            return Err(DiscoveryError::InvalidResponse {
                reason: format!("Invalid URL format: {url}"),
            });
        }

        let protocol = parts[0];
        let rest = parts[1];
        let host_port: Vec<&str> = rest.split(':').collect();
        let address = host_port
            .first()
            .ok_or_else(|| DiscoveryError::InvalidResponse {
                reason: format!("Missing host in URL: {url}"),
            })?;
        let port = host_port.get(1).and_then(|p| p.parse().ok()).unwrap_or(80);

        Ok(Self {
            protocol: protocol.to_string(),
            address: (*address).to_string(),
            port,
            path: None,
            metadata: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_url_string_http_with_port() {
        let endpoint = ServiceEndpoint::from_url_string("http://localhost:8080").unwrap();
        assert_eq!(endpoint.protocol, "http");
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.port, 8080);
        assert!(endpoint.path.is_none());
        assert!(endpoint.metadata.is_empty());
    }

    #[test]
    fn test_from_url_string_https_with_port() {
        let endpoint = ServiceEndpoint::from_url_string("https://example.com:443").unwrap();
        assert_eq!(endpoint.protocol, "https");
        assert_eq!(endpoint.address, "example.com");
        assert_eq!(endpoint.port, 443);
    }

    #[test]
    fn test_from_url_string_default_port() {
        let endpoint = ServiceEndpoint::from_url_string("http://localhost").unwrap();
        assert_eq!(endpoint.protocol, "http");
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn test_from_url_string_ip_address() {
        let endpoint = ServiceEndpoint::from_url_string("http://192.168.1.100:9000").unwrap();
        assert_eq!(endpoint.protocol, "http");
        assert_eq!(endpoint.address, "192.168.1.100");
        assert_eq!(endpoint.port, 9000);
    }

    #[test]
    fn test_from_url_string_custom_protocol() {
        let endpoint = ServiceEndpoint::from_url_string("grpc://service.local:50051").unwrap();
        assert_eq!(endpoint.protocol, "grpc");
        assert_eq!(endpoint.address, "service.local");
        assert_eq!(endpoint.port, 50051);
    }

    #[test]
    fn test_from_url_string_unix_socket_protocol() {
        let endpoint = ServiceEndpoint::from_url_string("unix:///var/run/service.sock").unwrap();
        assert_eq!(endpoint.protocol, "unix");
        assert_eq!(endpoint.address, "/var/run/service.sock");
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn test_from_url_string_invalid_no_protocol() {
        let result = ServiceEndpoint::from_url_string("localhost:8080");
        assert!(result.is_err());
        if let Err(DiscoveryError::InvalidResponse { reason }) = result {
            assert!(reason.contains("Invalid URL format"));
        }
    }

    #[test]
    fn test_from_url_string_invalid_empty() {
        let result = ServiceEndpoint::from_url_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_url_string_invalid_only_protocol() {
        let result = ServiceEndpoint::from_url_string("http://");
        assert!(result.is_ok());
        let endpoint = result.unwrap();
        assert_eq!(endpoint.address, "");
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn test_from_url_string_invalid_port_uses_default() {
        let endpoint = ServiceEndpoint::from_url_string("http://localhost:invalid").unwrap();
        assert_eq!(endpoint.address, "localhost");
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn test_from_url_string_high_port() {
        let endpoint = ServiceEndpoint::from_url_string("http://localhost:65535").unwrap();
        assert_eq!(endpoint.port, 65535);
    }

    #[test]
    fn test_from_url_string_websocket() {
        let endpoint = ServiceEndpoint::from_url_string("ws://localhost:3000").unwrap();
        assert_eq!(endpoint.protocol, "ws");
        assert_eq!(endpoint.port, 3000);
    }

    #[test]
    fn test_from_url_string_wss() {
        let endpoint = ServiceEndpoint::from_url_string("wss://secure.example.com:443").unwrap();
        assert_eq!(endpoint.protocol, "wss");
        assert_eq!(endpoint.address, "secure.example.com");
        assert_eq!(endpoint.port, 443);
    }
}
