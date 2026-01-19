//! System and ecosystem discovery functionality

use anyhow::{Context, Result};
use chrono::Utc;
use std::future::Future;
// use std::time::Duration;  // Unused after HTTP removal
use tokio::process::Command;
use tracing::{debug, info};

use super::types::*;

/// Discovery extension trait
pub trait DiscoveryExt {
    /// Discover system hardware and software
    fn discover_system(&mut self) -> impl Future<Output = Result<()>> + Send;

    /// Discover ecosystem services
    fn discover_ecosystem(&mut self) -> impl Future<Output = Result<()>> + Send;
}

impl DiscoveryExt for ZeroConfigDeployment {
    async fn discover_system(&mut self) -> Result<()> {
        info!("🖥️ Discovering system capabilities");

        // Discover CPU information
        self.system_info.cpu = self.discover_cpu().await?;

        // Discover memory information
        self.system_info.memory = self.discover_memory().await?;

        // Discover storage information
        self.system_info.storage = self.discover_storage().await?;

        // Discover network information
        self.system_info.network = self.discover_network().await?;

        // Discover OS information
        self.system_info.os = self.discover_os().await?;

        // Discover container runtime
        self.system_info.container_runtime = self.discover_container_runtime().await?;

        // Discover GPU information
        self.system_info.gpu = self.discover_gpu().await?;

        info!("✅ System discovery completed");
        Ok(())
    }

    async fn discover_ecosystem(&mut self) -> Result<()> {
        info!("🌐 Discovering ecosystem services via capabilities");

        // Use capability-based discovery instead of hardcoded names
        use toadstool_common::infant_discovery::capabilities::capabilities::*;

        // Discover orchestration service (formerly Songbird)
        self.ecosystem_services.songbird = self
            .discover_by_capability(ORCHESTRATION, "orchestration")
            .await?;

        // Discover PKI service (formerly BearDog)
        self.ecosystem_services.beardog = self.discover_by_capability(PKI, "pki").await?;

        // Discover storage service (formerly NestGate)
        self.ecosystem_services.nestgate = self.discover_by_capability(STORAGE, "storage").await?;

        // Discover AI service (formerly Squirrel)
        self.ecosystem_services.squirrel = self.discover_by_capability(AI_PROCESSING, "ai").await?;

        // Discover ToadStool peers
        self.ecosystem_services.toadstool_peers = self.discover_toadstool_peers().await?;

        info!("✅ Ecosystem discovery completed");
        Ok(())
    }
}

impl ZeroConfigDeployment {
    /// Discover CPU information
    async fn discover_cpu(&self) -> Result<CpuInfo> {
        debug!("Discovering CPU information");

        // Use /proc/cpuinfo on Linux
        let output = Command::new("nproc")
            .output()
            .await
            .context("Failed to run nproc")?;

        let cores = String::from_utf8(output.stdout)?
            .trim()
            .parse::<u32>()
            .unwrap_or(1);

        // Get CPU model from /proc/cpuinfo
        let model = self
            .get_cpu_model()
            .await
            .unwrap_or_else(|_| "Unknown".to_string());

        Ok(CpuInfo {
            cores,
            architecture: std::env::consts::ARCH.to_string(),
            model,
            frequency: 2400, // Default assumption
            vendor: "Unknown".to_string(),
        })
    }

    /// Get CPU model information
    async fn get_cpu_model(&self) -> Result<String> {
        let output = Command::new("cat")
            .arg("/proc/cpuinfo")
            .output()
            .await
            .context("Failed to read /proc/cpuinfo")?;

        let content = String::from_utf8(output.stdout)?;

        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some(model) = line.split(':').nth(1) {
                    return Ok(model.trim().to_string());
                }
            }
        }

        Ok("Unknown".to_string())
    }

    /// Discover memory information
    async fn discover_memory(&self) -> Result<MemoryInfo> {
        debug!("Discovering memory information");

        let output = Command::new("cat")
            .arg("/proc/meminfo")
            .output()
            .await
            .context("Failed to read /proc/meminfo")?;

        let content = String::from_utf8(output.stdout)?;
        let mut total_bytes = 0;
        let mut available_bytes = 0;

        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    total_bytes = value.parse::<u64>().unwrap_or(0) * 1024;
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    available_bytes = value.parse::<u64>().unwrap_or(0) * 1024;
                }
            }
        }

        Ok(MemoryInfo {
            total_bytes,
            available_bytes,
            memory_type: "DDR4".to_string(), // Default assumption
        })
    }

    /// Discover storage information
    async fn discover_storage(&self) -> Result<StorageInfo> {
        debug!("Discovering storage information");

        let output = Command::new("df")
            .arg("-B1")
            .arg("/")
            .output()
            .await
            .context("Failed to run df")?;

        let content = String::from_utf8(output.stdout)?;
        let mut total_bytes = 0;
        let mut available_bytes = 0;

        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                total_bytes = parts[1].parse::<u64>().unwrap_or(0);
                available_bytes = parts[3].parse::<u64>().unwrap_or(0);
                break;
            }
        }

        Ok(StorageInfo {
            total_bytes,
            available_bytes,
            storage_type: "SSD".to_string(), // Default assumption
            filesystem: "ext4".to_string(),  // Default assumption
        })
    }

    /// Discover network information
    async fn discover_network(&self) -> Result<NetworkInfo> {
        debug!("Discovering network information");

        let output = Command::new("ip")
            .arg("addr")
            .arg("show")
            .output()
            .await
            .context("Failed to run ip addr")?;

        let content = String::from_utf8(output.stdout)?;
        let mut interfaces = Vec::new();
        let mut local_ips = Vec::new();

        // Parse network interfaces
        for line in content.lines() {
            if line.contains("inet ") && !line.contains("127.0.0.1") {
                if let Some(ip) = line.split_whitespace().nth(1) {
                    if let Some(ip_addr) = ip.split('/').next() {
                        local_ips.push(ip_addr.to_string());
                    }
                }
            }
        }

        // Add a default interface
        if !local_ips.is_empty() {
            interfaces.push(NetworkInterface {
                name: "eth0".to_string(),
                ip: local_ips[0].clone(),
                mac: "00:00:00:00:00:00".to_string(),
                speed: 1000,
            });
        }

        Ok(NetworkInfo {
            interfaces,
            external_ip: None,
            local_ips,
        })
    }

    /// Discover OS information
    async fn discover_os(&self) -> Result<OsInfo> {
        debug!("Discovering OS information");

        let output = Command::new("uname")
            .arg("-a")
            .output()
            .await
            .context("Failed to run uname")?;

        let content = String::from_utf8(output.stdout)?;
        let parts: Vec<&str> = content.split_whitespace().collect();

        Ok(OsInfo {
            name: parts.first().unwrap_or(&"Unknown").to_string(),
            version: parts.get(2).unwrap_or(&"Unknown").to_string(),
            kernel: parts.get(2).unwrap_or(&"Unknown").to_string(),
            arch: parts.get(4).unwrap_or(&"Unknown").to_string(),
        })
    }

    /// Discover container runtime
    async fn discover_container_runtime(&self) -> Result<ContainerRuntimeInfo> {
        debug!("Discovering container runtime");

        let docker = Command::new("docker")
            .arg("--version")
            .output()
            .await
            .is_ok();

        let podman = Command::new("podman")
            .arg("--version")
            .output()
            .await
            .is_ok();

        let containerd = Command::new("containerd")
            .arg("--version")
            .output()
            .await
            .is_ok();

        let version = if docker {
            self.get_docker_version().await.ok()
        } else if podman {
            self.get_podman_version().await.ok()
        } else {
            None
        };

        Ok(ContainerRuntimeInfo {
            docker,
            podman,
            containerd,
            version,
        })
    }

    /// Get Docker version
    async fn get_docker_version(&self) -> Result<String> {
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .await
            .context("Failed to get Docker version")?;

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Get Podman version
    async fn get_podman_version(&self) -> Result<String> {
        let output = Command::new("podman")
            .arg("--version")
            .output()
            .await
            .context("Failed to get Podman version")?;

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    /// Discover GPU information
    async fn discover_gpu(&self) -> Result<GpuInfo> {
        debug!("Discovering GPU information");

        // Try to detect NVIDIA GPU
        let nvidia_output = Command::new("nvidia-smi")
            .arg("--query-gpu=count,name,memory.total")
            .arg("--format=csv,noheader,nounits")
            .output()
            .await;

        if let Ok(output) = nvidia_output {
            if output.status.success() {
                let content = String::from_utf8(output.stdout)?;
                if let Some(line) = content.lines().next() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        return Ok(GpuInfo {
                            count: parts[0].trim().parse().unwrap_or(0),
                            vendor: "NVIDIA".to_string(),
                            model: parts[1].trim().to_string(),
                            memory_bytes: parts[2].trim().parse::<u64>().unwrap_or(0) * 1024 * 1024,
                            cuda: true,
                            opencl: true,
                        });
                    }
                }
            }
        }

        // Default no GPU
        Ok(GpuInfo {
            count: 0,
            vendor: "None".to_string(),
            model: "None".to_string(),
            memory_bytes: 0,
            cuda: false,
            opencl: false,
        })
    }

    /// Discover service by capability (modern approach)
    ///
    /// This replaces hardcoded discover_songbird, discover_beardog, etc.
    /// Services are discovered by what they can do, not by hardcoded names/ports.
    async fn discover_by_capability(
        &self,
        capability: &str,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        debug!("Discovering service with {} capability", capability_name);

        // Use network configuration for discovery endpoints
        use toadstool_config::network_config::NetworkConfig;
        let network_config = NetworkConfig::from_env();

        // Try discovery endpoints from network config
        for discovery_endpoint in &network_config.discovery_endpoints {
            debug!("Trying discovery endpoint: {}", discovery_endpoint);

            // Query for services with this capability
            // In a full implementation, this would use mDNS, DNS-SD, or a registry service
            // For now, we'll try common patterns based on the capability
            if let Some(service) = self
                .try_discover_capability(capability, capability_name, discovery_endpoint)
                .await?
            {
                return Ok(Some(service));
            }
        }

        // Fallback: try localhost with environment-configured ports
        if let Some(service) = self.try_localhost_discovery(capability_name).await? {
            return Ok(Some(service));
        }

        debug!("Service with {} capability not found", capability_name);
        Ok(None)
    }

    /// Try to discover capability via discovery protocols
    ///
    /// Uses modern service discovery mechanisms:
    /// - mDNS (multicast DNS for local network)
    /// - DNS-SD (DNS-based service discovery)
    /// - HTTP Registry (centralized discovery service)
    async fn try_discover_capability(
        &self,
        capability: &str,
        capability_name: &str,
        _discovery_endpoint: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        use super::service_discovery::ServiceDiscovery;

        // Create service discovery coordinator
        let discovery = ServiceDiscovery::new();

        // Try all discovery methods
        discovery
            .discover_by_capability(capability, capability_name)
            .await
    }

    /// Try localhost discovery with environment-aware ports
    async fn try_localhost_discovery(
        &self,
        capability_name: &str,
    ) -> Result<Option<ServiceEndpoint>> {
        use toadstool_config::network_config::NetworkConfig;
        let config = NetworkConfig::from_env();

        // Map capabilities to ports (still using config, not hardcoding!)
        let port = match capability_name {
            "orchestration" => config.service_port + 2000, // e.g., 5000 + 2000 = 7000
            "pki" => config.service_port + 3000,           // e.g., 5000 + 3000 = 8000
            "storage" => config.service_port + 4000,       // e.g., 5000 + 4000 = 9000
            "ai" => config.service_port + 1000,            // e.g., 5000 + 1000 = 6000
            _ => return Ok(None),
        };

        let endpoint = format!("http://localhost:{}", port);
        match self
            .check_service_endpoint(&endpoint, capability_name)
            .await
        {
            Ok(service) => {
                debug!("Found {} service on localhost:{}", capability_name, port);
                Ok(Some(service))
            }
            Err(_) => Ok(None),
        }
    }

    /// Discover ToadStool peers
    async fn discover_toadstool_peers(&self) -> Result<Vec<ServiceEndpoint>> {
        debug!("Discovering ToadStool peers");

        // Network scan for ToadStool instances
        let mut peers = Vec::new();

        // Try common ToadStool ports
        let ports = vec![5000, 5001, 5002];

        for port in ports {
            let endpoint = format!("http://localhost:{port}");
            if let Ok(service) = self.check_service_endpoint(&endpoint, "toadstool").await {
                peers.push(service);
            }
        }

        Ok(peers)
    }

    /// Check if a service endpoint is available
    ///
    /// DEEP DEBT SOLUTION: HTTP removed - use Unix sockets for primal discovery!
    /// External HTTP should go through Songbird (Concentrated Gap architecture)
    async fn check_service_endpoint(
        &self,
        endpoint: &str,
        service_name: &str,
    ) -> Result<ServiceEndpoint> {
        // PURE RUST: External HTTP removed - stub for backward compatibility
        tracing::warn!(
            "HTTP service check deprecated for {} at {} - use Unix socket discovery instead",
            service_name,
            endpoint
        );

        // Return mock success for now - real impl should use Unix socket RPC
        // TODO: Migrate to capability-based discovery via Unix sockets
        Ok(ServiceEndpoint {
            name: service_name.to_string(),
            endpoint: endpoint.to_string(),
            version: "1.0.0".to_string(),
            status: "stub".to_string(), // Indicate this is a stub
            auth_required: false,
            discovered_at: Utc::now(),
        })
    }
}
