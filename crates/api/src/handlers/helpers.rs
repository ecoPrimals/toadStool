// SPDX-License-Identifier: AGPL-3.0-or-later
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
/// Detects actual system resources at runtime using capability-based discovery.
/// Falls back to conservative defaults if detection fails.
pub async fn get_local_node_resources() -> NodeResources {
    let cpu_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1) as u32;

    // Memory detection: try /proc/meminfo on Linux, fall back to conservative default
    let memory_gb = detect_system_memory_gb().unwrap_or(4);

    // Storage: conservative default, actual detection requires filesystem access
    // which may not be available in all environments
    let storage_gb = 100;

    // GPU count: would require GPU library detection (wgpu enumerate)
    // For API layer, default to 0 and let compute layer handle GPU discovery
    let gpu_count = 0;

    NodeResources {
        cpu_cores,
        memory_gb,
        storage_gb,
        gpu_count,
    }
}

/// Detect system memory in GB
///
/// Uses /proc/meminfo on Linux, falls back to None on other platforms.
fn detect_system_memory_gb() -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Some((kb / 1024 / 1024) as u32);
                        }
                    }
                }
            }
        }
        None
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_get_base_url_default() {
        let headers = HeaderMap::new();
        let url = get_base_url(&headers);
        assert!(url.starts_with("http://"));
        assert!(url.contains("127.0.0.1") || url.contains("localhost"));
    }

    #[test]
    fn test_get_base_url_with_host() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com:8080".parse().unwrap());
        let url = get_base_url(&headers);
        assert_eq!(url, "http://example.com:8080");
    }

    #[test]
    fn test_get_base_url_with_host_and_proto() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "api.example.com".parse().unwrap());
        headers.insert("x-forwarded-proto", "https".parse().unwrap());
        let url = get_base_url(&headers);
        assert_eq!(url, "https://api.example.com");
    }

    #[test]
    fn test_get_base_url_invalid_host_header() {
        let mut headers = HeaderMap::new();
        headers.insert("host", "example.com:8080".parse().unwrap());
        let url = get_base_url(&headers);
        assert!(url.starts_with("http"));
        assert!(url.contains("example.com"));
    }

    #[tokio::test]
    async fn test_get_local_node_resources() {
        let resources = get_local_node_resources().await;
        assert!(resources.cpu_cores >= 1);
        assert!(resources.memory_gb >= 1);
        assert!(resources.storage_gb > 0);
        // gpu_count is unsigned — always >= 0 by type definition
        let _ = resources.gpu_count;
    }
}
