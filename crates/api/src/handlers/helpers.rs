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
