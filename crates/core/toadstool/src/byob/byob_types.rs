//! BYOB (Bring Your Own Biome) type definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// BYOB deployment request from Songbird
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentRequest {
    /// Unique deployment ID
    pub deployment_id: Uuid,
    /// Team identifier
    pub team_id: String,
    /// Deployment name
    pub deployment_name: String,
    /// Services to deploy
    pub services: HashMap<String, ServiceSpec>,
    /// Resource quotas for the team
    pub resource_quotas: TeamResourceQuotas,
    /// Security configuration
    pub security_config: TeamSecurityConfig,
    /// Network configuration
    pub network_config: TeamNetworkConfig,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// Service specification within a team deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Container image or executable
    pub image: Option<String>,
    /// Command to run
    pub command: Option<Vec<String>>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Resource requirements
    pub resources: ServiceResourceRequirements,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Dependencies on other services
    pub dependencies: Vec<String>,
    /// Health check configuration
    pub health_check: Option<HealthCheck>,
    /// Replica count
    pub replicas: u32,
}

/// Resource requirements for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResourceRequirements {
    /// CPU cores required
    pub cpu_cores: Option<f64>,
    /// Memory in bytes
    pub memory_bytes: Option<u64>,
    /// Storage in bytes
    pub storage_bytes: Option<u64>,
    /// GPU count
    pub gpu_count: Option<u32>,
}

/// Team resource quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResourceQuotas {
    /// Maximum CPU cores for the team
    pub max_cpu_cores: f64,
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum storage in bytes
    pub max_storage_bytes: u64,
    /// Maximum GPU count
    pub max_gpu_count: u32,
    /// Maximum concurrent services
    pub max_concurrent_services: u32,
}

/// Team security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSecurityConfig {
    /// Isolation level
    pub isolation_level: String,
    /// Network policies
    pub network_policies: Vec<String>,
    /// Volume access policies
    pub volume_policies: Vec<String>,
    /// Resource access policies
    pub resource_policies: Vec<String>,
}

/// Team network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamNetworkConfig {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// DNS configuration
    pub dns_config: Option<DnsConfig>,
    /// Load balancer configuration
    pub load_balancer: Option<LoadBalancerConfig>,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional)
    pub host_port: Option<u16>,
    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount type
    pub mount_type: String,
    /// Read-only flag
    pub read_only: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Health check command
    pub command: Vec<String>,
    /// Check interval in seconds
    pub interval: u64,
    /// Timeout in seconds
    pub timeout: u64,
    /// Number of retries
    pub retries: u32,
    /// Start period in seconds
    pub start_period: u64,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancer type
    pub lb_type: String,
    /// Configuration options
    pub options: HashMap<String, String>,
}

/// BYOB deployment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentResponse {
    /// Deployment ID
    pub deployment_id: Uuid,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Service statuses
    pub service_statuses: HashMap<String, ServiceStatus>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Network information
    pub network_info: NetworkInfo,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is starting
    Starting,
    /// Deployment is running
    Running,
    /// Deployment is stopping
    Stopping,
    /// Deployment is stopped
    Stopped,
    /// Deployment failed
    Failed { error: String },
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Service name
    pub name: String,
    /// Service state
    pub state: String,
    /// Running replicas
    pub running_replicas: u32,
    /// Desired replicas
    pub desired_replicas: u32,
    /// Health status
    pub health: String,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Storage usage in bytes
    pub storage_usage: u64,
    /// GPU usage
    pub gpu_usage: u32,
    /// Network usage
    pub network_usage: NetworkUsage,
}

/// Network usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// Gateway IP
    pub gateway_ip: String,
    /// Service endpoints
    pub service_endpoints: HashMap<String, ServiceEndpoint>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub name: String,
    /// Internal IP
    pub internal_ip: String,
    /// External IP (if exposed)
    pub external_ip: Option<String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
}
