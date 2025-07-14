//! # Resource Manager
//!
//! Manages system resources, allocation, and monitoring.

use crate::manifest::{BiomeManifest, ServiceConfig, ResourceConfig, ServiceResourceConfig};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Resource management errors
#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Insufficient resources: {resource}")]
    InsufficientResources { resource: String },
    
    #[error("Invalid resource specification: {spec}")]
    InvalidResourceSpec { spec: String },
    
    #[error("Resource allocation failed: {reason}")]
    AllocationFailed { reason: String },
    
    #[error("Resource monitoring failed: {reason}")]
    MonitoringFailed { reason: String },
    
    #[error("System information unavailable: {reason}")]
    SystemInfoUnavailable { reason: String },
    
    #[error("Resource limit exceeded: {resource} - {limit}")]
    ResourceLimitExceeded { resource: String, limit: String },
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_info: String,
    pub kernel_version: String,
    pub architecture: String,
    pub uptime: String,
    pub cpu_cores: u32,
    pub cpu_usage: f64,
    pub memory_total: String,
    pub memory_used: String,
    pub memory_usage_percent: f64,
    pub disk_total: String,
    pub disk_used: String,
    pub disk_usage_percent: f64,
    pub wasm_runtime: String,
    pub container_runtime: String,
    pub python_runtime: String,
    pub federation_info: Option<FederationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationInfo {
    pub status: String,
    pub peer_count: usize,
}

/// Resource allocation for a service
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub allocation_id: uuid::Uuid,
    pub service_name: String,
    pub cpu_allocation: CpuAllocation,
    pub memory_allocation: MemoryAllocation,
    pub disk_allocation: DiskAllocation,
    pub network_allocation: NetworkAllocation,
    pub allocated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CpuAllocation {
    pub cores: f64,
    pub limit: Option<f64>,
    pub request: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct MemoryAllocation {
    pub bytes: u64,
    pub limit: Option<u64>,
    pub request: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DiskAllocation {
    pub bytes: u64,
    pub limit: Option<u64>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkAllocation {
    pub bandwidth_limit: Option<u64>,
    pub ports: Vec<u16>,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub disk_usage: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Resource monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub interval: std::time::Duration,
    pub retention_period: std::time::Duration,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub network_usage: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: std::time::Duration::from_secs(30),
            retention_period: std::time::Duration::from_secs(3600 * 24), // 24 hours
            alert_thresholds: AlertThresholds {
                cpu_usage: 80.0,
                memory_usage: 85.0,
                disk_usage: 90.0,
                network_usage: 75.0,
            },
        }
    }
}

/// Main resource manager
pub struct ResourceManager {
    system_info: Arc<RwLock<SystemInfo>>,
    allocations: Arc<RwLock<HashMap<uuid::Uuid, ResourceAllocation>>>,
    usage_history: Arc<RwLock<Vec<ResourceUsage>>>,
    monitoring_config: MonitoringConfig,
    
    // System limits
    total_cpu_cores: f64,
    total_memory: u64,
    total_disk: u64,
    
    // Current usage
    allocated_cpu: Arc<RwLock<f64>>,
    allocated_memory: Arc<RwLock<u64>>,
    allocated_disk: Arc<RwLock<u64>>,
}

impl ResourceManager {
    pub async fn new() -> Result<Self, ResourceError> {
        info!("Initializing resource manager");
        
        // Get system information
        let system_info = Self::gather_system_info().await?;
        
        // Extract system limits
        let total_cpu_cores = system_info.cpu_cores as f64;
        let total_memory = Self::parse_memory_size(&system_info.memory_total)?;
        let total_disk = Self::parse_disk_size(&system_info.disk_total)?;
        
        let manager = Self {
            system_info: Arc::new(RwLock::new(system_info)),
            allocations: Arc::new(RwLock::new(HashMap::new())),
            usage_history: Arc::new(RwLock::new(Vec::new())),
            monitoring_config: MonitoringConfig::default(),
            total_cpu_cores,
            total_memory,
            total_disk,
            allocated_cpu: Arc::new(RwLock::new(0.0)),
            allocated_memory: Arc::new(RwLock::new(0)),
            allocated_disk: Arc::new(RwLock::new(0)),
        };
        
        // Start monitoring
        manager.start_monitoring().await?;
        
        Ok(manager)
    }

    /// Validate biome resource requirements
    pub async fn validate_biome_resources(&self, manifest: &BiomeManifest) -> Result<(), ResourceError> {
        debug!("Validating biome resources: {}", manifest.metadata.name);
        
        let mut total_cpu_needed = 0.0;
        let mut total_memory_needed = 0u64;
        let mut total_disk_needed = 0u64;
        
        // Calculate total resources needed for all services
        for service in &manifest.services {
            if let Some(resources) = &service.resources {
                if let Some(cpu_limit) = &resources.cpu_limit {
                    total_cpu_needed += Self::parse_cpu_spec(cpu_limit)?;
                }
                
                if let Some(memory_limit) = &resources.memory_limit {
                    total_memory_needed += Self::parse_memory_size(memory_limit)?;
                }
                
                if let Some(disk_limit) = &resources.disk_limit {
                    total_disk_needed += Self::parse_disk_size(disk_limit)?;
                }
            }
        }
        
        // Check global biome limits
        if let Some(resources) = &manifest.resources {
            if let Some(cpu_limit) = &resources.cpu_limit {
                let biome_cpu_limit = Self::parse_cpu_spec(cpu_limit)?;
                if total_cpu_needed > biome_cpu_limit {
                    return Err(ResourceError::ResourceLimitExceeded {
                        resource: "CPU".to_string(),
                        limit: format!("Biome limit: {}, Services need: {}", biome_cpu_limit, total_cpu_needed),
                    });
                }
                total_cpu_needed = biome_cpu_limit;
            }
            
            if let Some(memory_limit) = &resources.memory_limit {
                let biome_memory_limit = Self::parse_memory_size(memory_limit)?;
                if total_memory_needed > biome_memory_limit {
                    return Err(ResourceError::ResourceLimitExceeded {
                        resource: "Memory".to_string(),
                        limit: format!("Biome limit: {}, Services need: {}", biome_memory_limit, total_memory_needed),
                    });
                }
                total_memory_needed = biome_memory_limit;
            }
        }
        
        // Check system availability
        let allocated_cpu = *self.allocated_cpu.read().await;
        let allocated_memory = *self.allocated_memory.read().await;
        let allocated_disk = *self.allocated_disk.read().await;
        
        if allocated_cpu + total_cpu_needed > self.total_cpu_cores {
            return Err(ResourceError::InsufficientResources {
                resource: format!("CPU: need {}, available {}", total_cpu_needed, self.total_cpu_cores - allocated_cpu),
            });
        }
        
        if allocated_memory + total_memory_needed > self.total_memory {
            return Err(ResourceError::InsufficientResources {
                resource: format!("Memory: need {}, available {}", total_memory_needed, self.total_memory - allocated_memory),
            });
        }
        
        if allocated_disk + total_disk_needed > self.total_disk {
            return Err(ResourceError::InsufficientResources {
                resource: format!("Disk: need {}, available {}", total_disk_needed, self.total_disk - allocated_disk),
            });
        }
        
        Ok(())
    }

    /// Allocate resources for a service
    pub async fn allocate_service_resources(&self, service: &ServiceConfig) -> Result<ResourceAllocation, ResourceError> {
        info!("Allocating resources for service: {}", service.name);
        
        let allocation_id = uuid::Uuid::new_v4();
        
        // Parse resource requirements
        let cpu_allocation = self.allocate_cpu_resources(service).await?;
        let memory_allocation = self.allocate_memory_resources(service).await?;
        let disk_allocation = self.allocate_disk_resources(service).await?;
        let network_allocation = self.allocate_network_resources(service).await?;
        
        let allocation = ResourceAllocation {
            allocation_id,
            service_name: service.name.clone(),
            cpu_allocation,
            memory_allocation,
            disk_allocation,
            network_allocation,
            allocated_at: chrono::Utc::now(),
        };
        
        // Store allocation
        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(allocation_id, allocation.clone());
        }
        
        // Update allocated resources
        {
            let mut allocated_cpu = self.allocated_cpu.write().await;
            *allocated_cpu += allocation.cpu_allocation.cores;
        }
        
        {
            let mut allocated_memory = self.allocated_memory.write().await;
            *allocated_memory += allocation.memory_allocation.bytes;
        }
        
        {
            let mut allocated_disk = self.allocated_disk.write().await;
            *allocated_disk += allocation.disk_allocation.bytes;
        }
        
        info!("Resources allocated for service: {} ({})", service.name, allocation_id);
        
        Ok(allocation)
    }

    /// Deallocate resources for a service
    pub async fn deallocate_service_resources(&self, allocation: &ResourceAllocation) -> Result<(), ResourceError> {
        info!("Deallocating resources for service: {} ({})", allocation.service_name, allocation.allocation_id);
        
        // Remove allocation
        {
            let mut allocations = self.allocations.write().await;
            allocations.remove(&allocation.allocation_id);
        }
        
        // Update allocated resources
        {
            let mut allocated_cpu = self.allocated_cpu.write().await;
            *allocated_cpu -= allocation.cpu_allocation.cores;
        }
        
        {
            let mut allocated_memory = self.allocated_memory.write().await;
            *allocated_memory -= allocation.memory_allocation.bytes;
        }
        
        {
            let mut allocated_disk = self.allocated_disk.write().await;
            *allocated_disk -= allocation.disk_allocation.bytes;
        }
        
        Ok(())
    }

    /// Get system information
    pub async fn get_system_info(&self, detailed: bool) -> Result<SystemInfo, ResourceError> {
        if detailed {
            // Refresh system information
            let fresh_info = Self::gather_system_info().await?;
            
            let mut system_info = self.system_info.write().await;
            *system_info = fresh_info.clone();
            
            Ok(fresh_info)
        } else {
            let system_info = self.system_info.read().await;
            Ok(system_info.clone())
        }
    }

    /// Get resource usage statistics
    pub async fn get_resource_usage(&self) -> Result<ResourceUsage, ResourceError> {
        let usage = ResourceUsage {
            cpu_usage: self.get_current_cpu_usage().await?,
            memory_usage: self.get_current_memory_usage().await?,
            disk_usage: self.get_current_disk_usage().await?,
            network_rx: self.get_current_network_rx().await?,
            network_tx: self.get_current_network_tx().await?,
            timestamp: chrono::Utc::now(),
        };
        
        Ok(usage)
    }

    /// Get resource allocation summary
    pub async fn get_allocation_summary(&self) -> Result<ResourceAllocationSummary, ResourceError> {
        let allocations = self.allocations.read().await;
        let allocated_cpu = *self.allocated_cpu.read().await;
        let allocated_memory = *self.allocated_memory.read().await;
        let allocated_disk = *self.allocated_disk.read().await;
        
        Ok(ResourceAllocationSummary {
            total_allocations: allocations.len(),
            allocated_cpu,
            allocated_memory,
            allocated_disk,
            available_cpu: self.total_cpu_cores - allocated_cpu,
            available_memory: self.total_memory - allocated_memory,
            available_disk: self.total_disk - allocated_disk,
            cpu_utilization: (allocated_cpu / self.total_cpu_cores) * 100.0,
            memory_utilization: (allocated_memory as f64 / self.total_memory as f64) * 100.0,
            disk_utilization: (allocated_disk as f64 / self.total_disk as f64) * 100.0,
        })
    }

    // Private helper methods

    async fn gather_system_info() -> Result<SystemInfo, ResourceError> {
        use sysinfo::{System, SystemExt, CpuExt, DiskExt};
        
        let mut system = System::new_all();
        system.refresh_all();
        
        let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
        
        let memory_total = system.total_memory();
        let memory_used = system.used_memory();
        let memory_usage_percent = (memory_used as f64 / memory_total as f64) * 100.0;
        
        let disk_total: u64 = system.disks().iter().map(|disk| disk.total_space()).sum();
        let disk_used: u64 = system.disks().iter().map(|disk| disk.total_space() - disk.available_space()).sum();
        let disk_usage_percent = (disk_used as f64 / disk_total as f64) * 100.0;
        
        Ok(SystemInfo {
            os_info: format!("{} {}", system.name().unwrap_or("Unknown"), system.os_version().unwrap_or("Unknown")),
            kernel_version: system.kernel_version().unwrap_or("Unknown".to_string()),
            architecture: std::env::consts::ARCH.to_string(),
            uptime: format!("{} seconds", system.uptime()),
            cpu_cores: system.cpus().len() as u32,
            cpu_usage: cpu_usage as f64,
            memory_total: Self::format_bytes(memory_total),
            memory_used: Self::format_bytes(memory_used),
            memory_usage_percent,
            disk_total: Self::format_bytes(disk_total),
            disk_used: Self::format_bytes(disk_used),
            disk_usage_percent,
            wasm_runtime: "wasmtime 18.0".to_string(),
            container_runtime: "native".to_string(),
            python_runtime: "python 3.x".to_string(),
            federation_info: None,
        })
    }

    async fn allocate_cpu_resources(&self, service: &ServiceConfig) -> Result<CpuAllocation, ResourceError> {
        let (cores, limit, request) = if let Some(resources) = &service.resources {
            let cores = if let Some(cpu_limit) = &resources.cpu_limit {
                Self::parse_cpu_spec(cpu_limit)?
            } else {
                0.1 // Default CPU allocation
            };
            
            let limit = resources.cpu_limit.as_ref().map(|s| Self::parse_cpu_spec(s)).transpose()?;
            let request = resources.cpu_request.as_ref().map(|s| Self::parse_cpu_spec(s)).transpose()?;
            
            (cores, limit, request)
        } else {
            (0.1, None, None) // Default values
        };
        
        Ok(CpuAllocation { cores, limit, request })
    }

    async fn allocate_memory_resources(&self, service: &ServiceConfig) -> Result<MemoryAllocation, ResourceError> {
        let (bytes, limit, request) = if let Some(resources) = &service.resources {
            let bytes = if let Some(memory_limit) = &resources.memory_limit {
                Self::parse_memory_size(memory_limit)?
            } else {
                128 * 1024 * 1024 // Default 128MB
            };
            
            let limit = resources.memory_limit.as_ref().map(|s| Self::parse_memory_size(s)).transpose()?;
            let request = resources.memory_request.as_ref().map(|s| Self::parse_memory_size(s)).transpose()?;
            
            (bytes, limit, request)
        } else {
            (128 * 1024 * 1024, None, None) // Default values
        };
        
        Ok(MemoryAllocation { bytes, limit, request })
    }

    async fn allocate_disk_resources(&self, service: &ServiceConfig) -> Result<DiskAllocation, ResourceError> {
        let (bytes, limit, paths) = if let Some(resources) = &service.resources {
            let bytes = if let Some(disk_limit) = &resources.disk_limit {
                Self::parse_disk_size(disk_limit)?
            } else {
                1024 * 1024 * 1024 // Default 1GB
            };
            
            let limit = resources.disk_limit.as_ref().map(|s| Self::parse_disk_size(s)).transpose()?;
            let paths = service.volumes.iter().map(|v| v.mount_path.clone()).collect();
            
            (bytes, limit, paths)
        } else {
            (1024 * 1024 * 1024, None, Vec::new()) // Default values
        };
        
        Ok(DiskAllocation { bytes, limit, paths })
    }

    async fn allocate_network_resources(&self, service: &ServiceConfig) -> Result<NetworkAllocation, ResourceError> {
        let ports = service.ports.iter().map(|p| p.container_port).collect();
        
        // Calculate bandwidth limit based on service configuration
        let bandwidth_limit = if let Some(resources) = &service.resources {
            // Parse bandwidth specification (e.g., "100M", "1G", "500K")
            resources.network_limit.as_ref().map(|limit| {
                self.parse_bandwidth_limit(limit)
            }).transpose()?
        } else {
            // Default bandwidth limit: 100 Mbps
            Some(100 * 1024 * 1024 / 8) // 100 Mbps in bytes per second
        };
        
        Ok(NetworkAllocation {
            bandwidth_limit,
            ports,
        })
    }

    async fn start_monitoring(&self) -> Result<(), ResourceError> {
        if !self.monitoring_config.enabled {
            return Ok();
        }
        
        info!("Starting resource monitoring");
        
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(manager.monitoring_config.interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.collect_usage_metrics().await {
                    error!("Failed to collect usage metrics: {}", e);
                }
            }
        });
        
        Ok(())
    }

    async fn collect_usage_metrics(&self) -> Result<(), ResourceError> {
        let usage = self.get_resource_usage().await?;
        
        // Store usage in history
        {
            let mut history = self.usage_history.write().await;
            history.push(usage.clone());
            
            // Limit history size
            let max_entries = (self.monitoring_config.retention_period.as_secs() / self.monitoring_config.interval.as_secs()) as usize;
            if history.len() > max_entries {
                history.drain(0..history.len() - max_entries);
            }
        }
        
        // Check alert thresholds
        self.check_alert_thresholds(&usage).await?;
        
        Ok(())
    }

    async fn check_alert_thresholds(&self, usage: &ResourceUsage) -> Result<(), ResourceError> {
        let thresholds = &self.monitoring_config.alert_thresholds;
        
        if usage.cpu_usage > thresholds.cpu_usage {
            warn!("CPU usage alert: {:.1}% (threshold: {:.1}%)", usage.cpu_usage, thresholds.cpu_usage);
        }
        
        let memory_usage_percent = (usage.memory_usage as f64 / self.total_memory as f64) * 100.0;
        if memory_usage_percent > thresholds.memory_usage {
            warn!("Memory usage alert: {:.1}% (threshold: {:.1}%)", memory_usage_percent, thresholds.memory_usage);
        }
        
        let disk_usage_percent = (usage.disk_usage as f64 / self.total_disk as f64) * 100.0;
        if disk_usage_percent > thresholds.disk_usage {
            warn!("Disk usage alert: {:.1}% (threshold: {:.1}%)", disk_usage_percent, thresholds.disk_usage);
        }
        
        // Check network usage (convert bytes to MB for percentage calculation)
        let network_total_mb = (usage.network_rx + usage.network_tx) as f64 / (1024.0 * 1024.0);
        let network_usage_percent = (network_total_mb / 1000.0) * 100.0; // Assume 1GB/s baseline
        if network_usage_percent > thresholds.network_usage {
            warn!("Network usage alert: {:.1} MB total (threshold: {:.1}%)", network_total_mb, thresholds.network_usage);
        }
        
        Ok(())
    }

    async fn get_current_cpu_usage(&self) -> Result<f64, ResourceError> {
        use sysinfo::{System, SystemExt, CpuExt};
        
        let mut system = System::new();
        system.refresh_cpu();
        
        let cpu_usage = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / system.cpus().len() as f32;
        
        Ok(cpu_usage as f64)
    }

    async fn get_current_memory_usage(&self) -> Result<u64, ResourceError> {
        use sysinfo::{System, SystemExt};
        
        let mut system = System::new();
        system.refresh_memory();
        
        Ok(system.used_memory())
    }

    async fn get_current_disk_usage(&self) -> Result<u64, ResourceError> {
        use sysinfo::{System, SystemExt, DiskExt};
        
        let mut system = System::new();
        system.refresh_disks();
        
        let disk_used: u64 = system.disks().iter().map(|disk| disk.total_space() - disk.available_space()).sum();
        
        Ok(disk_used)
    }

    async fn get_current_network_rx(&self) -> Result<u64, ResourceError> {
        use sysinfo::{System, SystemExt, NetworkExt};
        
        let mut system = System::new();
        system.refresh_networks();
        
        let total_rx: u64 = system.networks().iter()
            .map(|(_name, network)| network.received())
            .sum();
        
        Ok(total_rx)
    }

    async fn get_current_network_tx(&self) -> Result<u64, ResourceError> {
        use sysinfo::{System, SystemExt, NetworkExt};
        
        let mut system = System::new();
        system.refresh_networks();
        
        let total_tx: u64 = system.networks().iter()
            .map(|(_name, network)| network.transmitted())
            .sum();
        
        Ok(total_tx)
    }

    fn parse_cpu_spec(spec: &str) -> Result<f64, ResourceError> {
        if spec.ends_with('m') {
            let millis: f64 = spec[..spec.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: spec.to_string() })?;
            Ok(millis / 1000.0)
        } else {
            spec.parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: spec.to_string() })
        }
    }

    fn parse_memory_size(size: &str) -> Result<u64, ResourceError> {
        let size = size.to_uppercase();
        
        if size.ends_with("GI") {
            let gib: f64 = size[..size.len()-2].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((gib * 1024.0 * 1024.0 * 1024.0) as u64)
        } else if size.ends_with("MI") {
            let mib: f64 = size[..size.len()-2].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((mib * 1024.0 * 1024.0) as u64)
        } else if size.ends_with("KI") {
            let kib: f64 = size[..size.len()-2].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((kib * 1024.0) as u64)
        } else if size.ends_with("G") {
            let gb: f64 = size[..size.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((gb * 1000.0 * 1000.0 * 1000.0) as u64)
        } else if size.ends_with("M") {
            let mb: f64 = size[..size.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((mb * 1000.0 * 1000.0) as u64)
        } else if size.ends_with("K") {
            let kb: f64 = size[..size.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })?;
            Ok((kb * 1000.0) as u64)
        } else {
            size.parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: size.to_string() })
        }
    }

    fn parse_disk_size(size: &str) -> Result<u64, ResourceError> {
        Self::parse_memory_size(size) // Same parsing logic
    }

    fn parse_bandwidth_limit(&self, limit: &str) -> Result<u64, ResourceError> {
        let limit = limit.to_uppercase();
        
        // Parse bandwidth in bits per second, convert to bytes per second
        let bits_per_second = if limit.ends_with("GBPS") {
            let gbps: f64 = limit[..limit.len()-4].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (gbps * 1_000_000_000.0) as u64
        } else if limit.ends_with("MBPS") {
            let mbps: f64 = limit[..limit.len()-4].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (mbps * 1_000_000.0) as u64
        } else if limit.ends_with("KBPS") {
            let kbps: f64 = limit[..limit.len()-4].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (kbps * 1_000.0) as u64
        } else if limit.ends_with("G") {
            let gbps: f64 = limit[..limit.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (gbps * 1_000_000_000.0) as u64
        } else if limit.ends_with("M") {
            let mbps: f64 = limit[..limit.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (mbps * 1_000_000.0) as u64
        } else if limit.ends_with("K") {
            let kbps: f64 = limit[..limit.len()-1].parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?;
            (kbps * 1_000.0) as u64
        } else {
            // Default to bits per second
            limit.parse()
                .map_err(|_| ResourceError::InvalidResourceSpec { spec: limit.to_string() })?
        };
        
        // Convert bits per second to bytes per second
        Ok(bits_per_second / 8)
    }

    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Clone implementation for Arc sharing
impl Clone for ResourceManager {
    fn clone(&self) -> Self {
        Self {
            system_info: Arc::clone(&self.system_info),
            allocations: Arc::clone(&self.allocations),
            usage_history: Arc::clone(&self.usage_history),
            monitoring_config: self.monitoring_config.clone(),
            total_cpu_cores: self.total_cpu_cores,
            total_memory: self.total_memory,
            total_disk: self.total_disk,
            allocated_cpu: Arc::clone(&self.allocated_cpu),
            allocated_memory: Arc::clone(&self.allocated_memory),
            allocated_disk: Arc::clone(&self.allocated_disk),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationSummary {
    pub total_allocations: usize,
    pub allocated_cpu: f64,
    pub allocated_memory: u64,
    pub allocated_disk: u64,
    pub available_cpu: f64,
    pub available_memory: u64,
    pub available_disk: u64,
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub disk_utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_manager_creation() {
        let manager = ResourceManager::new().await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_cpu_spec_parsing() {
        assert_eq!(ResourceManager::parse_cpu_spec("100m").unwrap(), 0.1);
        assert_eq!(ResourceManager::parse_cpu_spec("1").unwrap(), 1.0);
        assert_eq!(ResourceManager::parse_cpu_spec("2.5").unwrap(), 2.5);
    }

    #[test]
    fn test_memory_size_parsing() {
        assert_eq!(ResourceManager::parse_memory_size("1Gi").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(ResourceManager::parse_memory_size("512Mi").unwrap(), 512 * 1024 * 1024);
        assert_eq!(ResourceManager::parse_memory_size("1G").unwrap(), 1000 * 1000 * 1000);
        assert_eq!(ResourceManager::parse_memory_size("100M").unwrap(), 100 * 1000 * 1000);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(ResourceManager::format_bytes(1024), "1.0 KB");
        assert_eq!(ResourceManager::format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(ResourceManager::format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[tokio::test]
    async fn test_bandwidth_parsing() {
        let manager = ResourceManager::new().await.unwrap();
        
        // Test standard bandwidth formats
        assert_eq!(manager.parse_bandwidth_limit("100M").unwrap(), 100 * 1_000_000 / 8);
        assert_eq!(manager.parse_bandwidth_limit("1G").unwrap(), 1_000_000_000 / 8);
        assert_eq!(manager.parse_bandwidth_limit("500K").unwrap(), 500 * 1_000 / 8);
        
        // Test explicit format
        assert_eq!(manager.parse_bandwidth_limit("100Mbps").unwrap(), 100 * 1_000_000 / 8);
        assert_eq!(manager.parse_bandwidth_limit("1Gbps").unwrap(), 1_000_000_000 / 8);
        assert_eq!(manager.parse_bandwidth_limit("500Kbps").unwrap(), 500 * 1_000 / 8);
        
        // Test raw bits per second
        assert_eq!(manager.parse_bandwidth_limit("8000000").unwrap(), 1_000_000); // 8 Mbps = 1 MB/s
    }
} 