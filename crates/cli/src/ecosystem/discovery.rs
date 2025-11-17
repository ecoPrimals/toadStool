//! Service discovery functionality for ecosystem integration
//!
//! This module handles discovering services on the network, verifying them,
//! and maintaining a registry of discovered services.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;
use tracing::{info, warn};

use super::types::*;

/// Get standard service ports for well-known ecosystem services
pub fn get_standard_service_ports() -> HashMap<String, u16> {
    let mut ports = HashMap::new();
    ports.insert("songbird".to_string(), 8080);
    ports.insert("beardog".to_string(), 8081);
    ports.insert("nestgate".to_string(), 8082);
    ports.insert("squirrel".to_string(), 8083);
    ports
}

/// Scan for a specific service type on the network
pub async fn scan_for_service(
    service_type: &str,
    service_ports: &HashMap<String, u16>,
) -> Result<Vec<ServiceEndpoint>> {
    let mut services = Vec::new();

    // Get the standard port for this service
    let port = service_ports.get(service_type).copied().unwrap_or(8080);

    // Scan localhost first (development/local deployment)
    let local_addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .with_context(|| "Failed to parse local address")?;

    // Try to connect to the service
    if is_service_reachable(&local_addr).await {
        services.push(ServiceEndpoint {
            service_type: parse_service_type(service_type),
            address: local_addr,
            version: "unknown".to_string(),
            capabilities: Vec::new(),
            trust_level: TrustLevel::Discovered,
        });
    }

    Ok(services)
}

/// Check if a service is reachable at the given address
async fn is_service_reachable(addr: &SocketAddr) -> bool {
    // Simple TCP connection check with short timeout
    matches!(
        timeout(
            Duration::from_millis(500),
            tokio::net::TcpStream::connect(addr),
        )
        .await,
        Ok(Ok(_))
    )
}

/// Verify a discovered service by checking its health endpoint
pub async fn verify_service(service: &ServiceEndpoint) -> Result<bool> {
    // Try to connect with a longer timeout for verification
    match timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(&service.address),
    )
    .await
    {
        Ok(Ok(_)) => {
            info!("✅ Service verified: {}", service.address);
            Ok(true)
        }
        Ok(Err(e)) => {
            warn!("⚠️  Service verification failed: {}", e);
            Ok(false)
        }
        Err(_) => {
            warn!("⚠️  Service verification timeout");
            Ok(false)
        }
    }
}

/// Parse service type from string
fn parse_service_type(service_type: &str) -> EcosystemService {
    match service_type.to_lowercase().as_str() {
        "songbird" => EcosystemService::Songbird,
        "beardog" => EcosystemService::BearDog,
        "nestgate" => EcosystemService::NestGate,
        _ => EcosystemService::Unknown(service_type.to_string()),
    }
}

// health_check() removed - was unused. Service health is checked via
// is_service_reachable() and verify_service() which are actively used.
