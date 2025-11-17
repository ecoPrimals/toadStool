//! Helper functions for API handlers

use crate::types::NodeResources;
use axum::http::HeaderMap;
use toadstool_config::defaults;

/// Extract base URL from request headers
///
/// Attempts to construct the base URL from the Host header and X-Forwarded-Proto.
/// Falls back to configured default if headers are not present.
pub fn get_base_url(headers: &HeaderMap) -> String {
    let default_host = format!(
        "{}:{}",
        defaults::network::LOCALHOST,
        defaults::network::API_PORT
    );
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&default_host);

    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("http");

    format!("{proto}://{host}")
}

/// Get local node resources
///
/// Returns mock resource data for the local node.
/// In production, this would query actual system resources.
pub async fn get_local_node_resources() -> NodeResources {
    NodeResources {
        cpu_cores: 8,
        memory_gb: 16,
        storage_gb: 500,
        gpu_count: 0,
    }
}
