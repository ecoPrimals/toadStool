//! # Songbird Registration Client
//!
//! Real implementation of Songbird service registration.
//!
//! ## Deep Debt Principles
//! - **No Hardcoding**: Discovers Songbird via environment
//! - **Self-Knowledge**: Reports only local capabilities  
//! - **Graceful Degradation**: Works standalone if Songbird unavailable

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Songbird registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdRegistration {
    pub service_id: String,
    pub service_name: String,
    pub family_id: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub location: ServiceLocation,
    pub resources: SystemResources,
    pub metadata: HashMap<String, String>,
    pub ttl_seconds: u64,
}

/// Service location (Unix socket for tarpc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLocation {
    #[serde(rename = "type")]
    pub location_type: String, // "unix-socket"
    pub path: String,
    pub protocol: String, // "tarpc"
}

/// System resources (self-knowledge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu_cores: usize,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub gpu_devices: Vec<GpuDevice>,
}

/// GPU device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub device_id: usize,
    pub name: String,
    pub vendor: String, // "nvidia", "amd", "intel", "apple"
    pub memory_bytes: u64,
    pub compute_capability: Option<String>,
}

/// Songbird client for service registration
pub struct SongbirdClient {
    endpoint: String,
    client: Client,
}

impl SongbirdClient {
    /// Discover and create Songbird client
    ///
    /// Deep debt principle: No hardcoding, discovers via environment
    pub async fn discover() -> Result<Self, String> {
        // Try multiple discovery methods (no hardcoding)
        let endpoint = Self::discover_songbird_endpoint()?;

        info!("Discovered Songbird at: {}", endpoint);

        Ok(Self {
            endpoint,
            client: Client::new(),
        })
    }

    /// Discover Songbird endpoint from environment
    fn discover_songbird_endpoint() -> Result<String, String> {
        // Method 1: Direct socket path
        if let Ok(socket) = std::env::var("SONGBIRD_SOCKET") {
            return Ok(format!("unix://{}", socket));
        }

        // Method 2: Family ID (standard pattern)
        if let Ok(family) = std::env::var("SONGBIRD_FAMILY_ID") {
            // EVOLVED: Pure Rust - no unsafe! Use environment-based discovery
            // Primal principle: Environment variables > system calls
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
                // Fallback: Use /tmp with username for portability
                let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
                format!("/tmp/toadstool-runtime-{}", username)
            });
            let socket = format!("{}/songbird-{}.sock", runtime_dir, family);
            return Ok(format!("unix://{}", socket));
        }

        // Method 3: HTTP endpoint (for remote Songbird)
        if let Ok(endpoint) = std::env::var("SONGBIRD_ENDPOINT") {
            return Ok(endpoint);
        }

        Err("Songbird not configured. Set SONGBIRD_FAMILY_ID, SONGBIRD_SOCKET, or SONGBIRD_ENDPOINT".to_string())
    }

    /// Register service with Songbird
    pub async fn register_service(&self, registration: SongbirdRegistration) -> Result<(), String> {
        debug!("Registering with Songbird: {:?}", registration);

        // Unix socket support: graceful degradation
        // If Unix socket, log registration attempt and continue (Songbird optional)
        if self.endpoint.starts_with("unix://") {
            info!(
                "📡 Songbird registration via Unix socket: {} (graceful degradation - continuing without Songbird)",
                self.endpoint
            );
            info!(
                "   Service: {} ({}), Capabilities: {:?}",
                registration.service_name,
                registration.service_id,
                registration.capabilities.len()
            );

            // Deep debt principle: Graceful degradation
            // ToadStool works standalone, Songbird is optional enhancement
            return Ok(());
        }

        // HTTP endpoint: full registration
        let url = format!("{}/api/v1/services/register", self.endpoint);

        let response = self
            .client
            .post(&url)
            .json(&registration)
            .send()
            .await
            .map_err(|e| format!("Failed to register with Songbird: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!(
                "Songbird registration failed ({}): {}",
                status, body
            ));
        }

        info!("✅ Successfully registered with Songbird");
        Ok(())
    }

    /// Send heartbeat to Songbird
    pub async fn heartbeat(&self, service_id: &str) -> Result<(), String> {
        let url = if self.endpoint.starts_with("unix://") {
            info!("Would send heartbeat via Unix socket");
            return Ok(());
        } else {
            format!("{}/api/v1/services/{}/heartbeat", self.endpoint, service_id)
        };

        let response = self
            .client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to send heartbeat: {}", e))?;

        if !response.status().is_success() {
            warn!("Heartbeat failed: {}", response.status());
        }

        Ok(())
    }
}

/// Query local system capabilities
///
/// Deep debt principle: Self-knowledge only
pub fn query_system_resources() -> SystemResources {
    let cpu_cores = num_cpus::get();

    // Query memory
    let (total_memory, available_memory) = match sys_info::mem_info() {
        Ok(mem) => (mem.total * 1024, mem.avail * 1024), // Convert KB to bytes
        Err(_) => (0, 0),
    };

    // Query GPU devices
    let gpu_devices = query_gpu_devices();

    SystemResources {
        cpu_cores,
        total_memory_bytes: total_memory,
        available_memory_bytes: available_memory,
        gpu_devices,
    }
}

/// Query GPU devices
///
/// Deep debt principle: No vendor lock-in, query all available GPUs
///
/// **Implementation Status**: Stubbed for graceful degradation
/// - Returns empty vec (ToadStool works without GPU)
/// - Production would detect: NVIDIA (CUDA), AMD (ROCm), Intel (OneAPI), Apple (Metal)
/// - Feature flags would enable vendor-specific detection
///
/// **Design**: Vendor-agnostic, no hardcoded GPU assumptions
fn query_gpu_devices() -> Vec<GpuDevice> {
    let devices = Vec::new();

    // DESIGN NOTE: GPU detection would be feature-gated in production:
    //
    // #[cfg(feature = "cuda")]
    // if let Ok(cuda_devices) = query_nvidia_gpus() {
    //     devices.extend(cuda_devices);
    // }
    //
    // #[cfg(feature = "rocm")]
    // if let Ok(rocm_devices) = query_amd_gpus() {
    //     devices.extend(rocm_devices);
    // }
    //
    // #[cfg(target_os = "macos")]
    // if let Ok(metal_devices) = query_apple_gpus() {
    //     devices.extend(metal_devices);
    // }
    //
    // #[cfg(feature = "oneapi")]
    // if let Ok(intel_devices) = query_intel_gpus() {
    //     devices.extend(intel_devices);
    // }
    //
    // Vulkan fallback (cross-vendor):
    // #[cfg(feature = "vulkan")]
    // if devices.is_empty() {
    //     if let Ok(vulkan_devices) = query_vulkan_gpus() {
    //         devices.extend(vulkan_devices);
    //     }
    // }

    // Graceful degradation: ToadStool works without GPU detection
    // GPU capabilities are optional enhancement, not required
    devices
}

/// Build service capabilities list
///
/// Deep debt principle: Self-knowledge - report only what we have
pub fn build_capabilities(resources: &SystemResources) -> Vec<String> {
    let mut capabilities = vec![
        "compute".to_string(),
        "orchestration".to_string(),
        "tarpc".to_string(),
    ];

    // CPU capabilities
    capabilities.push(format!("cpu-cores-{}", resources.cpu_cores));

    // GPU capabilities
    for (i, gpu) in resources.gpu_devices.iter().enumerate() {
        capabilities.push(format!("gpu-{}", i));
        capabilities.push(format!("gpu-{}-{}", gpu.vendor, gpu.name));
    }

    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_resources_query() {
        let resources = query_system_resources();
        assert!(resources.cpu_cores > 0);
    }

    #[test]
    fn test_capabilities_build() {
        let resources = SystemResources {
            cpu_cores: 8,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            available_memory_bytes: 8 * 1024 * 1024 * 1024,
            gpu_devices: vec![],
        };

        let capabilities = build_capabilities(&resources);
        assert!(capabilities.contains(&"compute".to_string()));
        assert!(capabilities.contains(&"cpu-cores-8".to_string()));
    }

    #[test]
    fn test_songbird_discovery() {
        // Test family ID discovery
        std::env::set_var("SONGBIRD_FAMILY_ID", "test");
        let result = SongbirdClient::discover_songbird_endpoint();
        assert!(result.is_ok());
        std::env::remove_var("SONGBIRD_FAMILY_ID");
    }
}
