//! ServiceEndpoint URL parsing extension

use std::collections::HashMap;

use crate::primal_identity::ServiceEndpoint;

use super::types::{DiscoveryError, DiscoveryResult};

impl ServiceEndpoint {
    /// Create endpoint from URL string
    pub fn from_url_string(url: &str) -> DiscoveryResult<Self> {
        let parts: Vec<&str> = url.split("://").collect();
        if parts.len() != 2 {
            return Err(DiscoveryError::InvalidResponse {
                reason: format!("Invalid URL format: {}", url),
            });
        }

        let protocol = parts[0];
        let rest = parts[1];
        let host_port: Vec<&str> = rest.split(':').collect();
        let address = host_port
            .first()
            .ok_or_else(|| DiscoveryError::InvalidResponse {
                reason: format!("Missing host in URL: {}", url),
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
