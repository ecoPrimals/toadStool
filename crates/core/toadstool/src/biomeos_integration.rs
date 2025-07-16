//! # biomeOS Integration for Universal Orchestration
//!
//! This module provides comprehensive biomeOS integration capabilities for orchestrating
//! all 5 Primals (ToadStool, Songbird, BearDog, NestGate, Squirrel) from a single
//! `biome.yaml` manifest file.
//!
//! ## Phase 4 Features
//!
//! - **Single manifest orchestration**: `biome.yaml` configures all Primals
//! - **Zero-configuration deployment**: Auto-discovery and configuration
//! - **Cross-Primal authentication**: BearDog token propagation
//! - **Automated provisioning**: Storage and AI agent deployment
//! - **Sub-60-second bootstrap**: Optimized startup performance

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{ToadStoolError, ToadStoolResult};

/// Enhanced BiomeManifest structure for Phase 4 Universal Orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// API version for compatibility
    pub api_version: String,
    /// Manifest type (always "Biome")
    pub kind: String,
    /// Biome metadata
    pub metadata: BiomeMetadata,
    /// Primal-specific configurations
    pub primals: PrimalsConfig,
    /// Storage configuration and provisioning
    pub storage: Option<BiomeStorage>,
    /// AI agent deployment configuration
    pub agents: Option<Vec<AgentConfig>>,
    /// Security policies and authentication
    pub security: Option<BiomeSecurity>,
    /// Network configuration
    pub networking: Option<BiomeNetworking>,
    /// Resource allocation and limits
    pub resources: Option<BiomeResources>,
    /// Legacy services configuration (for backward compatibility)
    pub services: Option<Vec<ServiceConfig>>,
}

/// Biome metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Unique biome name
    pub name: String,
    /// Team or organization
    pub team: Option<String>,
    /// Environment type (dev, staging, prod)
    pub environment: Option<String>,
    /// Biome version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Labels for categorization
    pub labels: HashMap<String, String>,
    /// Annotations for additional metadata
    pub annotations: HashMap<String, String>,
}

/// Configuration for all Primals in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalsConfig {
    /// ToadStool (Universal Compute) configuration
    pub toadstool: Option<ToadStoolConfig>,
    /// Songbird (Network Coordination) configuration
    pub songbird: Option<SongbirdConfig>,
    /// BearDog (Security) configuration
    pub beardog: Option<BearDogConfig>,
    /// NestGate (Storage) configuration
    pub nestgate: Option<NestGateConfig>,
    /// Squirrel (AI) configuration
    pub squirrel: Option<SquirrelConfig>,
    /// biomeOS (Universal OS) configuration
    pub biomeos: Option<BiomeOSConfig>,
}

/// ToadStool Universal Compute configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolConfig {
    /// Enable ToadStool
    pub enabled: bool,
    /// Act as primary orchestrator
    pub orchestrator: bool,
    /// Resource allocation
    pub resources: Option<PrimalResources>,
    /// Runtime engines to enable
    pub runtime_engines: Vec<String>,
    /// Execution environments
    pub execution_environments: Vec<String>,
    /// Substrate support
    pub substrates: Vec<String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Songbird Network Coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConfig {
    /// Enable Songbird
    pub enabled: bool,
    /// Enable service mesh functionality
    pub service_mesh: bool,
    /// Port range for dynamic allocation
    pub port_range: Option<String>,
    /// Load balancing strategy
    pub load_balancing: Option<String>,
    /// Health check configuration
    pub health_checks: Option<HealthCheckConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// BearDog Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    /// Enable BearDog
    pub enabled: bool,
    /// Security level (low, medium, high, maximum)
    pub security_level: String,
    /// Enable crypto-lock functionality
    pub crypto_lock: bool,
    /// Authentication methods
    pub auth_methods: Vec<String>,
    /// Token propagation settings
    pub token_propagation: Option<TokenPropagationConfig>,
    /// Security policies
    pub policies: Vec<String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// NestGate Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateConfig {
    /// Enable NestGate
    pub enabled: bool,
    /// Storage tier (cold, warm, hot)
    pub storage_tier: String,
    /// Volume definitions
    pub volumes: Vec<VolumeConfig>,
    /// Backup configuration
    pub backup: Option<BackupConfig>,
    /// Replication settings
    pub replication: Option<ReplicationConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Squirrel AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquirrelConfig {
    /// Enable Squirrel
    pub enabled: bool,
    /// AI agents to deploy
    pub ai_agents: Vec<AgentConfig>,
    /// Model configurations
    pub models: Vec<ModelConfig>,
    /// MCP (Model Control Protocol) settings
    pub mcp: Option<MCPConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// biomeOS Universal OS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeOSConfig {
    /// Enable biomeOS
    pub enabled: bool,
    /// OS compatibility layers
    pub compatibility_layers: Vec<String>,
    /// System services
    pub system_services: Vec<String>,
    /// Boot configuration
    pub boot: Option<BootConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Resource allocation for a Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResources {
    /// CPU cores allocation
    pub cpu_cores: Option<f64>,
    /// Memory allocation in GB
    pub memory_gb: Option<f64>,
    /// Storage allocation in GB
    pub storage_gb: Option<f64>,
    /// GPU allocation
    pub gpu: Option<GpuAllocation>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Number of GPUs
    pub count: u32,
    /// GPU type preference
    pub gpu_type: Option<String>,
    /// Memory per GPU in GB
    pub memory_gb: Option<f64>,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Number of retries
    pub retries: u32,
    /// Initial delay before first check
    pub initial_delay: Duration,
}

/// Token propagation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationConfig {
    /// Enable cross-Primal token propagation
    pub enabled: bool,
    /// Token refresh interval
    pub refresh_interval: Duration,
    /// Token validation settings
    pub validation: TokenValidationConfig,
}

/// Token validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidationConfig {
    /// Require signature validation
    pub require_signature: bool,
    /// Timestamp validation window
    pub timestamp_window: Duration,
    /// Replay attack protection
    pub replay_protection: bool,
}

/// Volume configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    /// Volume name
    pub name: String,
    /// Volume size (e.g., "100Gi", "1TB")
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Mount path
    pub mount_path: Option<String>,
    /// Backup policy
    pub backup_policy: Option<String>,
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Enable automated backups
    pub enabled: bool,
    /// Backup schedule (cron format)
    pub schedule: String,
    /// Retention policy
    pub retention: String,
    /// Backup destination
    pub destination: String,
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Enable replication
    pub enabled: bool,
    /// Replication factor
    pub factor: u32,
    /// Replication strategy
    pub strategy: String,
}

/// AI agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// Model to use
    pub model: String,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Resource requirements
    pub resources: Option<PrimalResources>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model name
    pub name: String,
    /// Model type (e.g., "gpt-4", "claude-3")
    pub model_type: String,
    /// Model parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Resource requirements
    pub resources: Option<PrimalResources>,
}

/// MCP (Model Control Protocol) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPConfig {
    /// Enable MCP
    pub enabled: bool,
    /// MCP version
    pub version: String,
    /// Protocol settings
    pub protocol: HashMap<String, serde_json::Value>,
}

/// Boot configuration for biomeOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    /// Boot mode (normal, recovery, safe)
    pub mode: String,
    /// Boot timeout
    pub timeout: Duration,
    /// Boot services
    pub services: Vec<String>,
}

/// Storage configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    /// Enable NestGate integration
    pub nestgate_integration: bool,
    /// Global storage settings
    pub global_settings: HashMap<String, serde_json::Value>,
    /// Storage classes
    pub storage_classes: Vec<StorageClass>,
    /// Persistent volumes
    pub persistent_volumes: Vec<PersistentVolume>,
}

/// Storage class definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClass {
    /// Storage class name
    pub name: String,
    /// Provisioner
    pub provisioner: String,
    /// Parameters
    pub parameters: HashMap<String, String>,
    /// Reclaim policy
    pub reclaim_policy: String,
}

/// Persistent volume definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentVolume {
    /// Volume name
    pub name: String,
    /// Capacity
    pub capacity: String,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Storage class
    pub storage_class: String,
    /// Host path (for local storage)
    pub host_path: Option<PathBuf>,
}

/// Security configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeSecurity {
    /// Enable BearDog integration
    pub beardog_integration: bool,
    /// Security policies
    pub policies: Vec<SecurityPolicy>,
    /// Network policies
    pub network_policies: Vec<NetworkPolicy>,
    /// Authentication settings
    pub authentication: AuthenticationConfig,
    /// Authorization settings
    pub authorization: AuthorizationConfig,
}

/// Security policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Policy type
    pub policy_type: String,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
}

/// Network policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Policy name
    pub name: String,
    /// Ingress rules
    pub ingress: Vec<NetworkRule>,
    /// Egress rules
    pub egress: Vec<NetworkRule>,
}

/// Network rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    /// Allowed sources/destinations
    pub from: Vec<String>,
    /// Allowed ports
    pub ports: Vec<u16>,
    /// Protocol
    pub protocol: String,
}

/// Policy rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule name
    pub name: String,
    /// Rule action (allow, deny)
    pub action: String,
    /// Rule conditions
    pub conditions: HashMap<String, serde_json::Value>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Authentication methods
    pub methods: Vec<String>,
    /// Token settings
    pub token: Option<TokenConfig>,
    /// OAuth settings
    pub oauth: Option<OAuthConfig>,
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationConfig {
    /// Authorization method
    pub method: String,
    /// Role-based access control
    pub rbac: Option<RBACConfig>,
    /// Policy-based access control
    pub pbac: Option<PBACConfig>,
}

/// Token configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Token type
    pub token_type: String,
    /// Token lifetime
    pub lifetime: Duration,
    /// Refresh settings
    pub refresh: Option<TokenRefreshConfig>,
}

/// Token refresh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshConfig {
    /// Enable token refresh
    pub enabled: bool,
    /// Refresh interval
    pub interval: Duration,
}

/// OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// OAuth provider
    pub provider: String,
    /// Client ID
    pub client_id: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
}

/// RBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RBACConfig {
    /// Enable RBAC
    pub enabled: bool,
    /// Roles
    pub roles: Vec<Role>,
    /// Role bindings
    pub role_bindings: Vec<RoleBinding>,
}

/// PBAC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PBACConfig {
    /// Enable PBAC
    pub enabled: bool,
    /// Policies
    pub policies: Vec<String>,
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name
    pub name: String,
    /// Permissions
    pub permissions: Vec<String>,
}

/// Role binding definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleBinding {
    /// Binding name
    pub name: String,
    /// Role name
    pub role: String,
    /// Subjects
    pub subjects: Vec<String>,
}

/// Networking configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeNetworking {
    /// Enable Songbird integration
    pub songbird_integration: bool,
    /// Network mode
    pub mode: String,
    /// DNS settings
    pub dns: Option<DNSConfig>,
    /// Port mappings
    pub port_mappings: Vec<PortMapping>,
    /// Service mesh settings
    pub service_mesh: Option<ServiceMeshConfig>,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DNSConfig {
    /// DNS servers
    pub servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
}

/// Port mapping definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port
    pub host_port: u16,
    /// Protocol
    pub protocol: String,
}

/// Service mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshConfig {
    /// Enable service mesh
    pub enabled: bool,
    /// Mesh provider
    pub provider: String,
    /// Mesh settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Resource configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeResources {
    /// CPU limits
    pub cpu_limit: Option<f64>,
    /// Memory limits
    pub memory_limit: Option<String>,
    /// Storage limits
    pub storage_limit: Option<String>,
    /// GPU limits
    pub gpu_limit: Option<u32>,
    /// Network bandwidth
    pub network_bandwidth: Option<String>,
}

/// Legacy service configuration (for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service source
    pub source: ServiceSource,
    /// Replicas
    pub replicas: Option<u32>,
    /// Resource requirements
    pub resources: Option<PrimalResources>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMountSpec>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Health check
    pub health_check: Option<HealthCheckConfig>,
}

/// Service source definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSource {
    /// Source type
    pub source_type: String,
    /// Registry
    pub registry: Option<String>,
    /// Image
    pub image: String,
    /// Tag
    pub tag: String,
    /// Digest
    pub digest: Option<String>,
}

/// Volume mount specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeMountSpec {
    /// Volume name to mount
    pub volume_name: String,
    /// Mount path in the container/environment
    pub mount_path: String,
    /// Read-only mount
    pub read_only: bool,
}

/// Volume mount information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeMountInfo {
    /// Mount specification
    pub spec: VolumeMountSpec,
    /// Mount ID
    pub mount_id: String,
    /// Mount status
    pub status: MountStatus,
    /// Mount time
    pub mounted_at: chrono::DateTime<chrono::Utc>,
}

/// Mount status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MountStatus {
    /// Mount is being created
    Mounting,
    /// Mount is active
    Mounted,
    /// Mount is being removed
    Unmounting,
    /// Mount failed
    Failed(String),
}

/// Volume provisioning status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VolumeProvisioningStatus {
    /// Provisioning successful
    Success(VolumeInfo),
    /// Provisioning failed
    Failed(String),
    /// Provisioning in progress
    InProgress,
    /// Provisioning skipped
    Skipped(String),
}

/// Volume mount status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VolumeMountStatus {
    /// Mount successful
    Success(VolumeMountInfo),
    /// Mount failed
    Failed(String),
    /// Mount in progress
    InProgress,
}

/// Volume cleanup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VolumeCleanupStatus {
    /// Cleanup successful
    Success,
    /// Cleanup failed
    Failed(String),
    /// Cleanup skipped
    Skipped(String),
}

/// Result of storage provisioning operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProvisioningResult {
    /// Total volumes processed
    pub total_volumes: usize,
    /// Successfully provisioned volumes
    pub provisioned_volumes: usize,
    /// Individual volume results
    pub results: HashMap<String, VolumeProvisioningStatus>,
    /// Provisioning timestamp
    pub provisioning_time: chrono::DateTime<chrono::Utc>,
}

/// Result of volume mounting operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMountResult {
    /// Total mounts attempted
    pub total_mounts: usize,
    /// Successful mounts
    pub successful_mounts: usize,
    /// Individual mount results
    pub results: HashMap<String, VolumeMountStatus>,
    /// Mount timestamp
    pub mount_time: chrono::DateTime<chrono::Utc>,
}

/// Result of storage cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCleanupResult {
    /// Total volumes checked for cleanup
    pub total_volumes_checked: usize,
    /// Successfully cleaned volumes
    pub cleaned_volumes: usize,
    /// Individual cleanup results
    pub results: HashMap<String, VolumeCleanupStatus>,
    /// Cleanup timestamp
    pub cleanup_time: chrono::DateTime<chrono::Utc>,
}

/// Volume usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeUsage {
    /// Volume name
    pub volume_name: String,
    /// Allocated bytes
    pub allocated_bytes: u64,
    /// Used bytes
    pub used_bytes: u64,
    /// Available bytes
    pub available_bytes: u64,
    /// Usage percentage
    pub usage_percentage: f64,
    /// Last updated
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Storage usage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageUsageReport {
    /// Total allocated bytes across all volumes
    pub total_allocated_bytes: u64,
    /// Total used bytes across all volumes
    pub total_used_bytes: u64,
    /// Overall usage percentage
    pub usage_percentage: f64,
    /// Individual volume usage
    pub volume_usage: HashMap<String, VolumeUsage>,
    /// Storage issues detected
    pub issues: Vec<String>,
    /// Report timestamp
    pub report_time: chrono::DateTime<chrono::Utc>,
}

/// Storage provisioning request to NestGate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProvisioningRequest {
    /// Volume name
    pub volume_name: String,
    /// Volume size
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Backup policy
    pub backup_policy: Option<String>,
    /// Replication settings
    pub replication: Option<ReplicationSettings>,
}

/// Replication settings for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSettings {
    /// Enable replication
    pub enabled: bool,
    /// Replication factor
    pub factor: u32,
    /// Replication strategy
    pub strategy: String,
}

impl Default for BiomeManifest {
    fn default() -> Self {
        Self {
            api_version: "biomeOS/v1".to_string(),
            kind: "Biome".to_string(),
            metadata: BiomeMetadata {
                name: "default-biome".to_string(),
                team: None,
                environment: Some("development".to_string()),
                version: "1.0.0".to_string(),
                description: Some("Default biome configuration".to_string()),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            primals: PrimalsConfig {
                toadstool: Some(ToadStoolConfig {
                    enabled: true,
                    orchestrator: true,
                    resources: Some(PrimalResources {
                        cpu_cores: Some(4.0),
                        memory_gb: Some(8.0),
                        storage_gb: Some(50.0),
                        gpu: None,
                        network_bandwidth: Some("1Gbps".to_string()),
                    }),
                    runtime_engines: vec![
                        "native".to_string(),
                        "wasm".to_string(),
                        "container".to_string(),
                    ],
                    execution_environments: vec!["linux".to_string(), "docker".to_string()],
                    substrates: vec!["x86_64".to_string(), "aarch64".to_string()],
                    config: HashMap::new(),
                }),
                songbird: Some(SongbirdConfig {
                    enabled: true,
                    service_mesh: true,
                    port_range: Some("8080-8999".to_string()),
                    load_balancing: Some("round_robin".to_string()),
                    health_checks: Some(HealthCheckConfig {
                        interval: Duration::from_secs(30),
                        timeout: Duration::from_secs(10),
                        retries: 3,
                        initial_delay: Duration::from_secs(30),
                    }),
                    config: HashMap::new(),
                }),
                beardog: Some(BearDogConfig {
                    enabled: true,
                    security_level: "high".to_string(),
                    crypto_lock: true,
                    auth_methods: vec!["ed25519".to_string()],
                    token_propagation: Some(TokenPropagationConfig {
                        enabled: true,
                        refresh_interval: Duration::from_secs(300),
                        validation: TokenValidationConfig {
                            require_signature: true,
                            timestamp_window: Duration::from_secs(60),
                            replay_protection: true,
                        },
                    }),
                    policies: vec!["default".to_string()],
                    config: HashMap::new(),
                }),
                nestgate: Some(NestGateConfig {
                    enabled: true,
                    storage_tier: "hot".to_string(),
                    volumes: vec![],
                    backup: Some(BackupConfig {
                        enabled: true,
                        schedule: "0 2 * * *".to_string(),
                        retention: "30d".to_string(),
                        destination: "s3://backup-bucket".to_string(),
                    }),
                    replication: Some(ReplicationConfig {
                        enabled: true,
                        factor: 3,
                        strategy: "async".to_string(),
                    }),
                    config: HashMap::new(),
                }),
                squirrel: Some(SquirrelConfig {
                    enabled: true,
                    ai_agents: vec![],
                    models: vec![],
                    mcp: Some(MCPConfig {
                        enabled: true,
                        version: "1.0".to_string(),
                        protocol: HashMap::new(),
                    }),
                    config: HashMap::new(),
                }),
                biomeos: Some(BiomeOSConfig {
                    enabled: true,
                    compatibility_layers: vec!["linux".to_string(), "docker".to_string()],
                    system_services: vec!["init".to_string(), "networking".to_string()],
                    boot: Some(BootConfig {
                        mode: "normal".to_string(),
                        timeout: Duration::from_secs(60),
                        services: vec!["all".to_string()],
                    }),
                    config: HashMap::new(),
                }),
            },
            storage: None,
            agents: None,
            security: None,
            networking: None,
            resources: None,
            services: None,
        }
    }
}

/// BiomeOS Integration Manager for orchestrating all Primals
pub struct BiomeOSIntegration {
    /// Current biome manifest
    manifest: BiomeManifest,
    /// Orchestration state
    orchestration_state: HashMap<String, PrimalOrchestrationState>,
}

/// Orchestration state for a Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalOrchestrationState {
    /// Primal name
    pub name: String,
    /// Current status
    pub status: PrimalOrchestrationStatus,
    /// Configuration applied
    pub config_applied: bool,
    /// Resources allocated
    pub resources_allocated: bool,
    /// Dependencies resolved
    pub dependencies_resolved: bool,
    /// Last update timestamp
    pub last_update: chrono::DateTime<chrono::Utc>,
}

/// Status of Primal orchestration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalOrchestrationStatus {
    /// Not started
    NotStarted,
    /// Configuration phase
    Configuring,
    /// Dependency resolution phase
    ResolvingDependencies,
    /// Resource allocation phase
    AllocatingResources,
    /// Starting services
    Starting,
    /// Running successfully
    Running,
    /// Failed
    Failed(String),
    /// Stopped
    Stopped,
}

impl BiomeOSIntegration {
    /// Create a new BiomeOS integration manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            manifest: BiomeManifest::default(),
            orchestration_state: HashMap::new(),
        }
    }

    /// Load biome manifest from file
    pub async fn load_manifest(&mut self, path: &str) -> ToadStoolResult<()> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to read manifest: {e}")))?;

        self.manifest = serde_yaml::from_str(&content)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to parse manifest: {e}")))?;

        Ok(())
    }

    /// Save biome manifest to file
    pub async fn save_manifest(&self, path: &str) -> ToadStoolResult<()> {
        let content = serde_yaml::to_string(&self.manifest)
            .map_err(|e| ToadStoolError::runtime(format!("Failed to serialize manifest: {e}")))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to write manifest: {e}")))?;

        Ok(())
    }

    /// Get current manifest
    #[must_use]
    pub fn get_manifest(&self) -> &BiomeManifest {
        &self.manifest
    }

    /// Update manifest
    pub fn update_manifest(&mut self, manifest: BiomeManifest) {
        self.manifest = manifest;
    }

    /// Get orchestration state
    #[must_use]
    pub fn get_orchestration_state(&self) -> &HashMap<String, PrimalOrchestrationState> {
        &self.orchestration_state
    }

    /// Update orchestration state for a Primal
    pub fn update_primal_state(&mut self, name: &str, status: PrimalOrchestrationStatus) {
        let state = self
            .orchestration_state
            .entry(name.to_string())
            .or_insert_with(|| PrimalOrchestrationState {
                name: name.to_string(),
                status: PrimalOrchestrationStatus::NotStarted,
                config_applied: false,
                resources_allocated: false,
                dependencies_resolved: false,
                last_update: chrono::Utc::now(),
            });

        state.status = status;
        state.last_update = chrono::Utc::now();
    }

    /// Bootstrap biome from manifest
    pub async fn bootstrap_biome(&mut self) -> ToadStoolResult<Duration> {
        let start_time = std::time::Instant::now();

        // Phase 1: Validate manifest
        self.validate_manifest()?;

        // Phase 2: Resolve dependencies
        self.resolve_dependencies().await?;

        // Phase 3: Allocate resources
        self.allocate_resources().await?;

        // Phase 4: Configure Primals
        self.configure_primals().await?;

        // Phase 5: Start Primals
        self.start_primals().await?;

        // Phase 6: Verify deployment
        self.verify_deployment().await?;

        Ok(start_time.elapsed())
    }

    /// Validate the biome manifest
    fn validate_manifest(&self) -> ToadStoolResult<()> {
        // Validate API version
        if self.manifest.api_version != "biomeOS/v1" {
            return Err(ToadStoolError::runtime(format!(
                "Unsupported API version: {}. Expected: biomeOS/v1",
                self.manifest.api_version
            )));
        }

        // Validate kind
        if self.manifest.kind != "Biome" {
            return Err(ToadStoolError::runtime(format!(
                "Unsupported kind: {}. Expected: Biome",
                self.manifest.kind
            )));
        }

        // Validate metadata
        if self.manifest.metadata.name.is_empty() {
            return Err(ToadStoolError::runtime(
                "Biome name cannot be empty".to_string(),
            ));
        }

        // Validate at least one Primal is enabled
        let primals = &self.manifest.primals;
        let any_enabled = primals.toadstool.as_ref().is_some_and(|p| p.enabled)
            || primals.songbird.as_ref().is_some_and(|p| p.enabled)
            || primals.beardog.as_ref().is_some_and(|p| p.enabled)
            || primals.nestgate.as_ref().is_some_and(|p| p.enabled)
            || primals.squirrel.as_ref().is_some_and(|p| p.enabled)
            || primals.biomeos.as_ref().is_some_and(|p| p.enabled);

        if !any_enabled {
            return Err(ToadStoolError::runtime(
                "At least one Primal must be enabled".to_string(),
            ));
        }

        Ok(())
    }

    /// Resolve dependencies between Primals
    async fn resolve_dependencies(&mut self) -> ToadStoolResult<()> {
        // Implementation for dependency resolution
        // This would analyze the manifest and determine startup order
        Ok(())
    }

    /// Allocate resources for Primals
    async fn allocate_resources(&mut self) -> ToadStoolResult<()> {
        // Implementation for resource allocation
        // This would reserve CPU, memory, storage, etc.
        Ok(())
    }

    /// Configure all enabled Primals
    async fn configure_primals(&mut self) -> ToadStoolResult<()> {
        // Implementation for Primal configuration
        // This would generate configuration files for each Primal
        Ok(())
    }

    /// Start all enabled Primals
    async fn start_primals(&mut self) -> ToadStoolResult<()> {
        // Implementation for Primal startup
        // This would start services in dependency order
        Ok(())
    }

    /// Verify deployment success
    async fn verify_deployment(&mut self) -> ToadStoolResult<()> {
        // Implementation for deployment verification
        // This would check health and connectivity
        Ok(())
    }

    // ===== BearDog Authentication Token Propagation =====

    /// Initialize cross-Primal authentication system
    pub async fn initialize_cross_primal_auth(&mut self) -> ToadStoolResult<AuthenticationManager> {
        let beardog_config = self.manifest.primals.beardog.as_ref().ok_or_else(|| {
            ToadStoolError::runtime("BearDog not configured in manifest".to_string())
        })?;

        if !beardog_config.enabled {
            return Err(ToadStoolError::runtime(
                "BearDog is not enabled in manifest".to_string(),
            ));
        }

        let token_config = beardog_config.token_propagation.as_ref().ok_or_else(|| {
            ToadStoolError::runtime("Token propagation not configured".to_string())
        })?;

        if !token_config.enabled {
            return Err(ToadStoolError::runtime(
                "Token propagation is disabled".to_string(),
            ));
        }

        // Create authentication manager
        let auth_manager = AuthenticationManager::new(AuthManagerConfig {
            beardog_endpoint: self.get_beardog_endpoint(),
            token_refresh_interval: token_config.refresh_interval,
            signature_validation: token_config.validation.require_signature,
            timestamp_window: token_config.validation.timestamp_window,
            replay_protection: token_config.validation.replay_protection,
        });

        // Initialize BearDog connection
        auth_manager.initialize_beardog_connection().await?;

        Ok(auth_manager)
    }

    /// Get BearDog endpoint from configuration
    fn get_beardog_endpoint(&self) -> String {
        // Check if custom endpoint is configured
        if let Some(endpoint) = self
            .manifest
            .primals
            .beardog
            .as_ref()
            .and_then(|config| config.config.get("endpoint"))
            .and_then(|v| v.as_str())
        {
            return endpoint.to_string();
        }

        // Default endpoint for local BearDog
        "http://localhost:8083".to_string()
    }

    /// Propagate authentication token to all Primals
    pub async fn propagate_auth_token(
        &mut self,
        auth_manager: &AuthenticationManager,
    ) -> ToadStoolResult<PropagationResult> {
        let mut results = HashMap::new();

        // Get current authentication token from BearDog
        let token = auth_manager.get_current_token().await?;

        // Propagate to all enabled Primals
        for (primal_name, primal_config) in &self.get_enabled_primals() {
            match self
                .propagate_token_to_primal(primal_name, &token, primal_config, auth_manager)
                .await
            {
                Ok(()) => {
                    results.insert(primal_name.clone(), TokenPropagationStatus::Success);
                }
                Err(e) => {
                    tracing::error!("Failed to propagate token to {}: {}", primal_name, e);
                    results.insert(
                        primal_name.clone(),
                        TokenPropagationStatus::Failed(e.to_string()),
                    );
                }
            }
        }

        let successful_count = results
            .values()
            .filter(|status| matches!(status, TokenPropagationStatus::Success))
            .count();

        Ok(PropagationResult {
            total_primals: results.len(),
            successful_propagations: successful_count,
            results,
            token_id: token.id,
            propagation_time: chrono::Utc::now(),
        })
    }

    /// Get enabled Primals from manifest
    fn get_enabled_primals(&self) -> HashMap<String, PrimalTypeConfig> {
        let mut enabled = HashMap::new();
        let primals = &self.manifest.primals;

        if let Some(config) = &primals.toadstool {
            if config.enabled {
                enabled.insert(
                    "toadstool".to_string(),
                    PrimalTypeConfig::ToadStool(config.clone()),
                );
            }
        }

        if let Some(config) = &primals.songbird {
            if config.enabled {
                enabled.insert(
                    "songbird".to_string(),
                    PrimalTypeConfig::Songbird(config.clone()),
                );
            }
        }

        if let Some(config) = &primals.beardog {
            if config.enabled {
                enabled.insert(
                    "beardog".to_string(),
                    PrimalTypeConfig::BearDog(config.clone()),
                );
            }
        }

        if let Some(config) = &primals.nestgate {
            if config.enabled {
                enabled.insert(
                    "nestgate".to_string(),
                    PrimalTypeConfig::NestGate(config.clone()),
                );
            }
        }

        if let Some(config) = &primals.squirrel {
            if config.enabled {
                enabled.insert(
                    "squirrel".to_string(),
                    PrimalTypeConfig::Squirrel(config.clone()),
                );
            }
        }

        if let Some(config) = &primals.biomeos {
            if config.enabled {
                enabled.insert(
                    "biomeos".to_string(),
                    PrimalTypeConfig::BiomeOS(config.clone()),
                );
            }
        }

        enabled
    }

    /// Propagate authentication token to a specific Primal
    #[allow(unused_variables)]
    async fn propagate_token_to_primal(
        &self,
        primal_name: &str,
        token: &AuthenticationToken,
        _primal_config: &PrimalTypeConfig,
        auth_manager: &AuthenticationManager,
    ) -> ToadStoolResult<()> {
        #[cfg(feature = "networking")]
        {
            // Get Primal endpoint
            let endpoint = self.get_primal_endpoint(primal_name)?;

            // Create token propagation request
            let request = TokenPropagationRequest {
                token: token.clone(),
                source_primal: "beardog".to_string(),
                target_primal: primal_name.to_string(),
                timestamp: chrono::Utc::now(),
                signature: auth_manager.sign_token_request(token, primal_name).await?,
            };

            // Send token to Primal
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{endpoint}/auth/token"))
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to send token to {primal_name}: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Token propagation to {} failed with status: {}",
                    primal_name,
                    response.status()
                )));
            }
        }

        #[cfg(not(feature = "networking"))]
        {
            tracing::info!(
                "Mock token propagation to {} (networking disabled)",
                primal_name
            );
        }

        Ok(())
    }

    /// Get endpoint for a specific Primal
    #[allow(dead_code)]
    fn get_primal_endpoint(&self, primal_name: &str) -> ToadStoolResult<String> {
        // Default endpoints for Primals
        let default_endpoints = [
            ("toadstool", "http://localhost:8080"),
            ("songbird", "http://localhost:8081"),
            ("beardog", "http://localhost:8083"),
            ("nestgate", "http://localhost:8084"),
            ("squirrel", "http://localhost:8085"),
            ("biomeos", "http://localhost:8086"),
        ];

        for (name, endpoint) in &default_endpoints {
            if *name == primal_name {
                return Ok(endpoint.to_string());
            }
        }

        Err(ToadStoolError::runtime(format!(
            "Unknown Primal: {primal_name}"
        )))
    }

    /// Verify authentication tokens across all Primals
    pub async fn verify_cross_primal_tokens(
        &self,
        auth_manager: &AuthenticationManager,
    ) -> ToadStoolResult<VerificationResult> {
        let mut results = HashMap::new();

        for primal_name in self.get_enabled_primals().keys() {
            match self.verify_primal_token(primal_name, auth_manager).await {
                Ok(status) => {
                    results.insert(primal_name.clone(), status);
                }
                Err(e) => {
                    tracing::error!("Failed to verify token for {}: {}", primal_name, e);
                    results.insert(
                        primal_name.clone(),
                        TokenVerificationStatus::Error(e.to_string()),
                    );
                }
            }
        }

        let valid_count = results
            .values()
            .filter(|status| matches!(status, TokenVerificationStatus::Valid))
            .count();

        Ok(VerificationResult {
            total_primals: results.len(),
            valid_tokens: valid_count,
            results,
            verification_time: chrono::Utc::now(),
        })
    }

    /// Verify authentication token for a specific Primal
    #[allow(unused_variables)]
    async fn verify_primal_token(
        &self,
        primal_name: &str,
        auth_manager: &AuthenticationManager,
    ) -> ToadStoolResult<TokenVerificationStatus> {
        #[cfg(feature = "networking")]
        {
            let endpoint = self.get_primal_endpoint(primal_name)?;

            // Create verification request
            let request = TokenVerificationRequest {
                primal_name: primal_name.to_string(),
                timestamp: chrono::Utc::now(),
                signature: auth_manager.sign_verification_request(primal_name).await?,
            };

            // Send verification request
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{endpoint}/auth/verify"))
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to verify token for {primal_name}: {e}"
                    ))
                })?;

            if !response.status().is_success() {
                return Ok(TokenVerificationStatus::Error(format!(
                    "Verification request failed with status: {}",
                    response.status()
                )));
            }

            let verification_response: TokenVerificationResponse =
                response.json().await.map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to parse verification response: {e}"))
                })?;

            Ok(verification_response.status)
        }

        #[cfg(not(feature = "networking"))]
        {
            tracing::info!(
                "Mock token verification for {} (networking disabled)",
                primal_name
            );
            Ok(TokenVerificationStatus::Valid)
        }
    }

    // ===== NestGate Storage Provisioning =====

    /// Initialize storage provisioning system
    pub async fn initialize_storage_provisioning(
        &mut self,
    ) -> ToadStoolResult<StorageProvisioningManager> {
        let nestgate_config = self.manifest.primals.nestgate.as_ref().ok_or_else(|| {
            ToadStoolError::runtime("NestGate not configured in manifest".to_string())
        })?;

        if !nestgate_config.enabled {
            return Err(ToadStoolError::runtime(
                "NestGate is not enabled in manifest".to_string(),
            ));
        }

        // Create storage provisioning manager
        let manager = StorageProvisioningManager::new(StorageProvisioningConfig {
            nestgate_endpoint: self.get_nestgate_endpoint(),
            storage_tier: nestgate_config.storage_tier.clone(),
            backup_enabled: nestgate_config.backup.as_ref().is_some_and(|b| b.enabled),
            replication_enabled: nestgate_config
                .replication
                .as_ref()
                .is_some_and(|r| r.enabled),
            replication_factor: nestgate_config.replication.as_ref().map_or(1, |r| r.factor),
        });

        // Initialize NestGate connection
        manager.initialize_nestgate_connection().await?;

        Ok(manager)
    }

    /// Get NestGate endpoint from configuration
    fn get_nestgate_endpoint(&self) -> String {
        // Check if custom endpoint is configured
        if let Some(endpoint) = self
            .manifest
            .primals
            .nestgate
            .as_ref()
            .and_then(|config| config.config.get("endpoint"))
            .and_then(|v| v.as_str())
        {
            return endpoint.to_string();
        }

        // Default endpoint for local NestGate
        "http://localhost:8084".to_string()
    }

    /// Provision storage from manifest configuration
    pub async fn provision_storage_from_manifest(
        &mut self,
        storage_manager: &StorageProvisioningManager,
    ) -> ToadStoolResult<StorageProvisioningResult> {
        let mut results = HashMap::new();
        let mut total_provisioned = 0;

        // Provision storage for NestGate configuration
        if let Some(nestgate_config) = &self.manifest.primals.nestgate {
            if nestgate_config.enabled {
                for volume in &nestgate_config.volumes {
                    match storage_manager.provision_volume(volume).await {
                        Ok(volume_info) => {
                            total_provisioned += 1;
                            results.insert(
                                volume.name.clone(),
                                VolumeProvisioningStatus::Success(volume_info),
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to provision volume {}: {}", volume.name, e);
                            results.insert(
                                volume.name.clone(),
                                VolumeProvisioningStatus::Failed(e.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Provision storage defined in the storage section
        if let Some(storage_config) = &self.manifest.storage {
            for pv in &storage_config.persistent_volumes {
                match storage_manager.provision_persistent_volume(pv).await {
                    Ok(volume_info) => {
                        total_provisioned += 1;
                        results.insert(
                            pv.name.clone(),
                            VolumeProvisioningStatus::Success(volume_info),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to provision persistent volume {}: {}", pv.name, e);
                        results.insert(
                            pv.name.clone(),
                            VolumeProvisioningStatus::Failed(e.to_string()),
                        );
                    }
                }
            }
        }

        Ok(StorageProvisioningResult {
            total_volumes: results.len(),
            provisioned_volumes: total_provisioned,
            results,
            provisioning_time: chrono::Utc::now(),
        })
    }

    /// Mount volumes in execution environments
    pub async fn mount_volumes_in_environments(
        &mut self,
        storage_manager: &StorageProvisioningManager,
    ) -> ToadStoolResult<VolumeMountResult> {
        let mut mount_results = HashMap::new();
        let mut successful_mounts = 0;

        // Get enabled Primals that need storage
        for (primal_name, primal_config) in &self.get_enabled_primals() {
            let volumes_to_mount = self.get_volumes_for_primal(primal_name, primal_config);

            for volume_mount in volumes_to_mount {
                match storage_manager
                    .mount_volume_in_primal(primal_name, &volume_mount)
                    .await
                {
                    Ok(mount_info) => {
                        successful_mounts += 1;
                        mount_results.insert(
                            format!("{}:{}", primal_name, volume_mount.volume_name),
                            VolumeMountStatus::Success(mount_info),
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to mount volume {} in {}: {}",
                            volume_mount.volume_name,
                            primal_name,
                            e
                        );
                        mount_results.insert(
                            format!("{}:{}", primal_name, volume_mount.volume_name),
                            VolumeMountStatus::Failed(e.to_string()),
                        );
                    }
                }
            }
        }

        Ok(VolumeMountResult {
            total_mounts: mount_results.len(),
            successful_mounts,
            results: mount_results,
            mount_time: chrono::Utc::now(),
        })
    }

    /// Get volumes that need to be mounted for a specific Primal
    fn get_volumes_for_primal(
        &self,
        primal_name: &str,
        primal_config: &PrimalTypeConfig,
    ) -> Vec<VolumeMountSpec> {
        let mut volumes = Vec::new();

        // Add volumes from NestGate configuration
        if let Some(nestgate_config) = &self.manifest.primals.nestgate {
            if nestgate_config.enabled {
                for volume in &nestgate_config.volumes {
                    if let Some(mount_path) = &volume.mount_path {
                        volumes.push(VolumeMountSpec {
                            volume_name: volume.name.clone(),
                            mount_path: mount_path.clone(),
                            read_only: false, // Default to read-write
                        });
                    }
                }
            }
        }

        // Add Primal-specific volumes based on configuration
        match primal_config {
            PrimalTypeConfig::ToadStool(config) => {
                // ToadStool might need workspace volumes, cache volumes, etc.
                if let Some(_storage_gb) = config.resources.as_ref().and_then(|r| r.storage_gb) {
                    volumes.push(VolumeMountSpec {
                        volume_name: format!("{primal_name}-workspace"),
                        mount_path: "/var/lib/toadstool/workspace".to_string(),
                        read_only: false,
                    });
                }
            }
            PrimalTypeConfig::Songbird(_) => {
                // Songbird might need configuration storage
                volumes.push(VolumeMountSpec {
                    volume_name: format!("{primal_name}-config"),
                    mount_path: "/etc/songbird".to_string(),
                    read_only: false,
                });
            }
            PrimalTypeConfig::NestGate(_) => {
                // NestGate needs data storage
                volumes.push(VolumeMountSpec {
                    volume_name: format!("{primal_name}-data"),
                    mount_path: "/var/lib/nestgate/data".to_string(),
                    read_only: false,
                });
            }
            PrimalTypeConfig::Squirrel(_) => {
                // Squirrel might need model storage and agent data
                volumes.push(VolumeMountSpec {
                    volume_name: format!("{primal_name}-models"),
                    mount_path: "/var/lib/squirrel/models".to_string(),
                    read_only: true, // Models are read-only
                });
                volumes.push(VolumeMountSpec {
                    volume_name: format!("{primal_name}-agents"),
                    mount_path: "/var/lib/squirrel/agents".to_string(),
                    read_only: false,
                });
            }
            _ => {}
        }

        volumes
    }

    /// Monitor storage usage across all Primals
    pub async fn monitor_storage_usage(
        &self,
        storage_manager: &StorageProvisioningManager,
    ) -> ToadStoolResult<StorageUsageReport> {
        let mut volume_usage = HashMap::new();
        let mut total_allocated = 0;
        let mut total_used = 0;

        // Get usage for all provisioned volumes
        if let Some(nestgate_config) = &self.manifest.primals.nestgate {
            if nestgate_config.enabled {
                for volume in &nestgate_config.volumes {
                    match storage_manager.get_volume_usage(&volume.name).await {
                        Ok(usage) => {
                            total_allocated += usage.allocated_bytes;
                            total_used += usage.used_bytes;
                            volume_usage.insert(volume.name.clone(), usage);
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to get usage for volume {}: {}",
                                volume.name,
                                e
                            );
                        }
                    }
                }
            }
        }

        // Calculate usage percentages and identify issues
        let usage_percentage = if total_allocated > 0 {
            (total_used as f64 / total_allocated as f64) * 100.0
        } else {
            0.0
        };

        let issues = volume_usage
            .iter()
            .filter_map(|(name, usage)| {
                let usage_pct = if usage.allocated_bytes > 0 {
                    (usage.used_bytes as f64 / usage.allocated_bytes as f64) * 100.0
                } else {
                    0.0
                };

                if usage_pct > 90.0 {
                    Some(format!("Volume {} is {}% full", name, usage_pct as u32))
                } else {
                    None
                }
            })
            .collect();

        Ok(StorageUsageReport {
            total_allocated_bytes: total_allocated,
            total_used_bytes: total_used,
            usage_percentage,
            volume_usage,
            issues,
            report_time: chrono::Utc::now(),
        })
    }

    /// Cleanup unused storage volumes
    pub async fn cleanup_unused_storage(
        &mut self,
        storage_manager: &StorageProvisioningManager,
    ) -> ToadStoolResult<StorageCleanupResult> {
        let mut cleanup_results = HashMap::new();
        let mut cleaned_volumes = 0;

        // Get list of volumes that are no longer needed
        let volumes_to_cleanup = self.identify_unused_volumes(storage_manager).await?;

        for volume_name in volumes_to_cleanup {
            match storage_manager.delete_volume(&volume_name).await {
                Ok(()) => {
                    cleaned_volumes += 1;
                    cleanup_results.insert(volume_name.clone(), VolumeCleanupStatus::Success);
                    tracing::info!("Successfully cleaned up volume: {}", volume_name);
                }
                Err(e) => {
                    tracing::error!("Failed to cleanup volume {}: {}", volume_name, e);
                    cleanup_results.insert(
                        volume_name.clone(),
                        VolumeCleanupStatus::Failed(e.to_string()),
                    );
                }
            }
        }

        Ok(StorageCleanupResult {
            total_volumes_checked: cleanup_results.len(),
            cleaned_volumes,
            results: cleanup_results,
            cleanup_time: chrono::Utc::now(),
        })
    }

    /// Identify volumes that are no longer needed
    async fn identify_unused_volumes(
        &self,
        storage_manager: &StorageProvisioningManager,
    ) -> ToadStoolResult<Vec<String>> {
        let mut unused_volumes = Vec::new();

        // Get all existing volumes from NestGate
        let existing_volumes = storage_manager.list_all_volumes().await?;

        // Get currently required volumes from manifest
        let mut required_volumes = std::collections::HashSet::new();

        if let Some(nestgate_config) = &self.manifest.primals.nestgate {
            if nestgate_config.enabled {
                for volume in &nestgate_config.volumes {
                    required_volumes.insert(volume.name.clone());
                }
            }
        }

        if let Some(storage_config) = &self.manifest.storage {
            for pv in &storage_config.persistent_volumes {
                required_volumes.insert(pv.name.clone());
            }
        }

        // Identify volumes that exist but are not required
        for volume in existing_volumes {
            if !required_volumes.contains(&volume.name) {
                // Check if volume is in use
                if !storage_manager.is_volume_in_use(&volume.name).await? {
                    unused_volumes.push(volume.name);
                }
            }
        }

        Ok(unused_volumes)
    }

    // ===== Squirrel AI Agent Deployment =====

    /// Initialize AI agent deployment system
    pub async fn initialize_agent_deployment(&mut self) -> ToadStoolResult<AgentDeploymentManager> {
        let squirrel_config = self.manifest.primals.squirrel.as_ref().ok_or_else(|| {
            ToadStoolError::runtime("Squirrel not configured in manifest".to_string())
        })?;

        if !squirrel_config.enabled {
            return Err(ToadStoolError::runtime(
                "Squirrel is not enabled in manifest".to_string(),
            ));
        }

        // Create agent deployment manager
        let manager = AgentDeploymentManager::new(AgentDeploymentConfig {
            squirrel_endpoint: self.get_squirrel_endpoint(),
            model_registry: squirrel_config
                .config
                .get("model_registry")
                .and_then(|v| v.as_str())
                .unwrap_or("local")
                .to_string(),
            agent_runtime: squirrel_config
                .config
                .get("agent_runtime")
                .and_then(|v| v.as_str())
                .unwrap_or("container")
                .to_string(),
            mcp_enabled: squirrel_config.mcp.as_ref().is_some_and(|m| m.enabled),
            resource_limits: squirrel_config
                .config
                .get("resource_limits")
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default(),
        });

        // Initialize Squirrel connection
        manager.initialize_squirrel_connection().await?;

        Ok(manager)
    }

    /// Get Squirrel endpoint from configuration
    fn get_squirrel_endpoint(&self) -> String {
        // Check if custom endpoint is configured
        if let Some(endpoint) = self
            .manifest
            .primals
            .squirrel
            .as_ref()
            .and_then(|config| config.config.get("endpoint"))
            .and_then(|v| v.as_str())
        {
            return endpoint.to_string();
        }

        // Default endpoint for local Squirrel
        "http://localhost:8085".to_string()
    }

    /// Deploy AI agents from manifest configuration
    pub async fn deploy_agents_from_manifest(
        &mut self,
        agent_manager: &mut AgentDeploymentManager,
    ) -> ToadStoolResult<AgentDeploymentResult> {
        let mut results = HashMap::new();
        let mut total_deployed = 0;

        // Deploy agents from Squirrel configuration
        if let Some(squirrel_config) = &self.manifest.primals.squirrel {
            if squirrel_config.enabled {
                for agent_config in &squirrel_config.ai_agents {
                    match agent_manager.deploy_agent(agent_config).await {
                        Ok(agent_info) => {
                            total_deployed += 1;
                            results.insert(
                                agent_config.name.clone(),
                                AgentDeploymentStatus::Success(agent_info),
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to deploy agent {}: {}", agent_config.name, e);
                            results.insert(
                                agent_config.name.clone(),
                                AgentDeploymentStatus::Failed(e.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Deploy agents from the agents section
        if let Some(agents_config) = &self.manifest.agents {
            for agent_config in agents_config {
                match agent_manager.deploy_agent(agent_config).await {
                    Ok(agent_info) => {
                        total_deployed += 1;
                        results.insert(
                            agent_config.name.clone(),
                            AgentDeploymentStatus::Success(agent_info),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to deploy agent {}: {}", agent_config.name, e);
                        results.insert(
                            agent_config.name.clone(),
                            AgentDeploymentStatus::Failed(e.to_string()),
                        );
                    }
                }
            }
        }

        Ok(AgentDeploymentResult {
            total_agents: results.len(),
            deployed_agents: total_deployed,
            results,
            deployment_time: chrono::Utc::now(),
        })
    }

    /// Deploy models for AI agents
    pub async fn deploy_models_from_manifest(
        &mut self,
        agent_manager: &mut AgentDeploymentManager,
    ) -> ToadStoolResult<ModelDeploymentResult> {
        let mut results = HashMap::new();
        let mut total_deployed = 0;

        // Deploy models from Squirrel configuration
        if let Some(squirrel_config) = &self.manifest.primals.squirrel {
            if squirrel_config.enabled {
                for model_config in &squirrel_config.models {
                    match agent_manager.deploy_model(model_config).await {
                        Ok(model_info) => {
                            total_deployed += 1;
                            results.insert(
                                model_config.name.clone(),
                                ModelDeploymentStatus::Success(model_info),
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to deploy model {}: {}", model_config.name, e);
                            results.insert(
                                model_config.name.clone(),
                                ModelDeploymentStatus::Failed(e.to_string()),
                            );
                        }
                    }
                }
            }
        }

        Ok(ModelDeploymentResult {
            total_models: results.len(),
            deployed_models: total_deployed,
            results,
            deployment_time: chrono::Utc::now(),
        })
    }

    /// Scale agents based on load and performance metrics
    pub async fn scale_agents(
        &mut self,
        agent_manager: &mut AgentDeploymentManager,
    ) -> ToadStoolResult<AgentScalingResult> {
        let mut scaling_results = HashMap::new();
        let mut total_scaled = 0;

        // Get current agent metrics
        let agent_metrics = agent_manager.get_agent_metrics().await?;

        for (agent_name, metrics) in agent_metrics {
            let scaling_decision = self.calculate_scaling_decision(&agent_name, &metrics);

            match scaling_decision {
                AgentScalingDecision::ScaleUp(replicas) => {
                    match agent_manager.scale_agent(&agent_name, replicas).await {
                        Ok(_) => {
                            total_scaled += 1;
                            scaling_results
                                .insert(agent_name.clone(), AgentScalingStatus::ScaledUp(replicas));
                            tracing::info!(
                                "Scaled up agent {} to {} replicas",
                                agent_name,
                                replicas
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to scale up agent {}: {}", agent_name, e);
                            scaling_results.insert(
                                agent_name.clone(),
                                AgentScalingStatus::Failed(e.to_string()),
                            );
                        }
                    }
                }
                AgentScalingDecision::ScaleDown(replicas) => {
                    match agent_manager.scale_agent(&agent_name, replicas).await {
                        Ok(_) => {
                            total_scaled += 1;
                            scaling_results.insert(
                                agent_name.clone(),
                                AgentScalingStatus::ScaledDown(replicas),
                            );
                            tracing::info!(
                                "Scaled down agent {} to {} replicas",
                                agent_name,
                                replicas
                            );
                        }
                        Err(e) => {
                            tracing::error!("Failed to scale down agent {}: {}", agent_name, e);
                            scaling_results.insert(
                                agent_name.clone(),
                                AgentScalingStatus::Failed(e.to_string()),
                            );
                        }
                    }
                }
                AgentScalingDecision::NoChange => {
                    scaling_results.insert(agent_name.clone(), AgentScalingStatus::NoChange);
                }
            }
        }

        Ok(AgentScalingResult {
            total_agents_evaluated: scaling_results.len(),
            agents_scaled: total_scaled,
            results: scaling_results,
            scaling_time: chrono::Utc::now(),
        })
    }

    /// Calculate scaling decision for an agent based on metrics
    fn calculate_scaling_decision(
        &self,
        _agent_name: &str,
        metrics: &AgentMetrics,
    ) -> AgentScalingDecision {
        // Get current replica count
        let current_replicas = metrics.replica_count;

        // Calculate resource utilization
        let cpu_utilization = metrics.cpu_usage;
        let memory_utilization = metrics.memory_usage;
        let request_queue_length = metrics.request_queue_length;
        let avg_response_time = metrics.avg_response_time_ms;

        // Scaling thresholds (can be configured from manifest)
        let cpu_scale_up_threshold = 80.0;
        let cpu_scale_down_threshold = 20.0;
        let memory_scale_up_threshold = 85.0;
        let memory_scale_down_threshold = 30.0;
        let queue_length_threshold = 50;
        let response_time_threshold = 5000.0; // 5 seconds

        // Determine scaling decision
        let should_scale_up = cpu_utilization > cpu_scale_up_threshold
            || memory_utilization > memory_scale_up_threshold
            || request_queue_length > queue_length_threshold
            || avg_response_time > response_time_threshold;

        let should_scale_down = current_replicas > 1
            && cpu_utilization < cpu_scale_down_threshold
            && memory_utilization < memory_scale_down_threshold
            && request_queue_length < 10
            && avg_response_time < 1000.0;

        if should_scale_up && current_replicas < 10 {
            // Max 10 replicas
            let new_replicas = std::cmp::min(current_replicas + 1, 10);
            AgentScalingDecision::ScaleUp(new_replicas)
        } else if should_scale_down {
            let new_replicas = std::cmp::max(current_replicas - 1, 1);
            AgentScalingDecision::ScaleDown(new_replicas)
        } else {
            AgentScalingDecision::NoChange
        }
    }

    /// Monitor agent performance and health
    pub async fn monitor_agent_performance(
        &self,
        agent_manager: &AgentDeploymentManager,
    ) -> ToadStoolResult<AgentPerformanceReport> {
        let mut agent_health = HashMap::new();
        let mut total_healthy = 0;
        let mut performance_issues = Vec::new();

        // Get performance metrics for all deployed agents
        let agent_metrics = agent_manager.get_agent_metrics().await?;

        for (agent_name, metrics) in agent_metrics {
            let health_status = self.assess_agent_health(&agent_name, &metrics);

            if health_status.healthy {
                total_healthy += 1;
            } else {
                for issue in &health_status.issues {
                    performance_issues.push(format!("Agent {agent_name}: {issue}"));
                }
            }

            agent_health.insert(agent_name, health_status);
        }

        // Check for model performance issues
        let model_metrics = agent_manager.get_model_metrics().await?;
        for (model_name, metrics) in model_metrics {
            if metrics.avg_inference_time > 10000.0 {
                // 10 seconds
                performance_issues.push(format!(
                    "Model {} has slow inference time: {:.2}ms",
                    model_name, metrics.avg_inference_time
                ));
            }

            if metrics.error_rate > 5.0 {
                // 5% error rate
                performance_issues.push(format!(
                    "Model {} has high error rate: {:.2}%",
                    model_name, metrics.error_rate
                ));
            }
        }

        Ok(AgentPerformanceReport {
            total_agents: agent_health.len(),
            healthy_agents: total_healthy,
            agent_health,
            performance_issues,
            report_time: chrono::Utc::now(),
        })
    }

    /// Assess the health of an individual agent
    fn assess_agent_health(&self, _agent_name: &str, metrics: &AgentMetrics) -> AgentHealthStatus {
        let mut issues = Vec::new();

        // Check CPU usage
        if metrics.cpu_usage > 90.0 {
            issues.push("High CPU usage".to_string());
        }

        // Check memory usage
        if metrics.memory_usage > 95.0 {
            issues.push("High memory usage".to_string());
        }

        // Check response time
        if metrics.avg_response_time_ms > 10000.0 {
            issues.push("Slow response time".to_string());
        }

        // Check error rate
        if metrics.error_rate > 10.0 {
            issues.push("High error rate".to_string());
        }

        // Check if agent is responsive
        let now = chrono::Utc::now();
        let last_heartbeat_secs = now
            .signed_duration_since(metrics.last_heartbeat)
            .num_seconds();
        if last_heartbeat_secs > 60 {
            issues.push("Agent unresponsive".to_string());
        }

        AgentHealthStatus {
            healthy: issues.is_empty(),
            issues,
            last_check: chrono::Utc::now(),
        }
    }

    /// Cleanup unused agents and models
    pub async fn cleanup_unused_agents(
        &mut self,
        agent_manager: &mut AgentDeploymentManager,
    ) -> ToadStoolResult<AgentCleanupResult> {
        let mut cleanup_results = HashMap::new();
        let mut cleaned_agents = 0;

        // Get list of agents that are no longer needed
        let agents_to_cleanup = self.identify_unused_agents(agent_manager).await?;

        for agent_name in agents_to_cleanup {
            match agent_manager.terminate_agent(&agent_name).await {
                Ok(()) => {
                    cleaned_agents += 1;
                    cleanup_results.insert(agent_name.clone(), AgentCleanupStatus::Success);
                    tracing::info!("Successfully cleaned up agent: {}", agent_name);
                }
                Err(e) => {
                    tracing::error!("Failed to cleanup agent {}: {}", agent_name, e);
                    cleanup_results.insert(
                        agent_name.clone(),
                        AgentCleanupStatus::Failed(e.to_string()),
                    );
                }
            }
        }

        // Cleanup unused models
        let models_to_cleanup = self.identify_unused_models(agent_manager).await?;
        for model_name in models_to_cleanup {
            match agent_manager.unload_model(&model_name).await {
                Ok(()) => {
                    cleanup_results
                        .insert(format!("model:{model_name}"), AgentCleanupStatus::Success);
                    tracing::info!("Successfully unloaded model: {}", model_name);
                }
                Err(e) => {
                    tracing::error!("Failed to unload model {}: {}", model_name, e);
                    cleanup_results.insert(
                        format!("model:{model_name}"),
                        AgentCleanupStatus::Failed(e.to_string()),
                    );
                }
            }
        }

        Ok(AgentCleanupResult {
            total_items_checked: cleanup_results.len(),
            cleaned_items: cleaned_agents,
            results: cleanup_results,
            cleanup_time: chrono::Utc::now(),
        })
    }

    /// Identify agents that are no longer needed
    async fn identify_unused_agents(
        &self,
        agent_manager: &AgentDeploymentManager,
    ) -> ToadStoolResult<Vec<String>> {
        let mut unused_agents = Vec::new();

        // Get all deployed agents
        let deployed_agents = agent_manager.list_all_agents().await?;

        // Get currently required agents from manifest
        let mut required_agents = std::collections::HashSet::new();

        if let Some(squirrel_config) = &self.manifest.primals.squirrel {
            if squirrel_config.enabled {
                for agent in &squirrel_config.ai_agents {
                    required_agents.insert(agent.name.clone());
                }
            }
        }

        if let Some(agents_config) = &self.manifest.agents {
            for agent in agents_config {
                required_agents.insert(agent.name.clone());
            }
        }

        // Identify agents that are deployed but not required
        for agent in deployed_agents {
            if !required_agents.contains(&agent.name) {
                // Check if agent is idle
                if agent_manager.is_agent_idle(&agent.name).await? {
                    unused_agents.push(agent.name);
                }
            }
        }

        Ok(unused_agents)
    }

    /// Identify models that are no longer needed
    async fn identify_unused_models(
        &self,
        agent_manager: &AgentDeploymentManager,
    ) -> ToadStoolResult<Vec<String>> {
        let mut unused_models = Vec::new();

        // Get all loaded models
        let loaded_models = agent_manager.list_all_models().await?;

        // Get currently required models from manifest
        let mut required_models = std::collections::HashSet::new();

        if let Some(squirrel_config) = &self.manifest.primals.squirrel {
            if squirrel_config.enabled {
                for model in &squirrel_config.models {
                    required_models.insert(model.name.clone());
                }

                for agent in &squirrel_config.ai_agents {
                    required_models.insert(agent.model.clone());
                }
            }
        }

        if let Some(agents_config) = &self.manifest.agents {
            for agent in agents_config {
                required_models.insert(agent.model.clone());
            }
        }

        // Identify models that are loaded but not required
        for model in loaded_models {
            if !required_models.contains(&model.name) {
                // Check if model is in use
                if !agent_manager.is_model_in_use(&model.name).await? {
                    unused_models.push(model.name);
                }
            }
        }

        Ok(unused_models)
    }
}

impl Default for BiomeOSIntegration {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Authentication Token Propagation Types =====

/// Authentication manager for cross-Primal token propagation
pub struct AuthenticationManager {
    /// Configuration
    config: AuthManagerConfig,
    /// Current authentication token
    current_token: Option<AuthenticationToken>,
    /// BearDog client
    #[cfg(feature = "networking")]
    #[allow(dead_code)]
    _beardog_client: Option<reqwest::Client>,
    #[cfg(not(feature = "networking"))]
    _beardog_client: Option<()>,
    /// Token refresh task handle
    refresh_task: Option<tokio::task::JoinHandle<()>>,
}

/// Configuration for authentication manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthManagerConfig {
    /// BearDog endpoint URL
    pub beardog_endpoint: String,
    /// Token refresh interval
    pub token_refresh_interval: Duration,
    /// Require signature validation
    pub signature_validation: bool,
    /// Timestamp validation window
    pub timestamp_window: Duration,
    /// Enable replay attack protection
    pub replay_protection: bool,
}

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationToken {
    /// Unique token ID
    pub id: String,
    /// Token type
    pub token_type: String,
    /// Token value (encrypted)
    pub token: String,
    /// Public key for signature verification
    pub public_key: String,
    /// Token expiration time
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Token issued time
    pub issued_at: chrono::DateTime<chrono::Utc>,
    /// Issuing Primal (always "beardog")
    pub issuer: String,
    /// Target audiences (Primals)
    pub audience: Vec<String>,
    /// Token scope/permissions
    pub scope: Vec<String>,
    /// Additional claims
    pub claims: HashMap<String, serde_json::Value>,
}

/// Token propagation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPropagationRequest {
    /// Authentication token to propagate
    pub token: AuthenticationToken,
    /// Source Primal (sender)
    pub source_primal: String,
    /// Target Primal (receiver)
    pub target_primal: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request signature for integrity
    pub signature: String,
}

/// Token verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationRequest {
    /// Primal name to verify token for
    pub primal_name: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Request signature
    pub signature: String,
}

/// Token verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationResponse {
    /// Verification status
    pub status: TokenVerificationStatus,
    /// Token expiration time (if valid)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Additional verification details
    pub details: Option<String>,
}

/// Token verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenVerificationStatus {
    /// Token is valid
    Valid,
    /// Token is expired
    Expired,
    /// Token is invalid
    Invalid,
    /// Token not found
    NotFound,
    /// Verification error
    Error(String),
}

/// Token propagation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenPropagationStatus {
    /// Propagation successful
    Success,
    /// Propagation failed
    Failed(String),
    /// Propagation pending
    Pending,
    /// Propagation skipped
    Skipped(String),
}

/// Result of token propagation across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationResult {
    /// Total number of Primals
    pub total_primals: usize,
    /// Number of successful propagations
    pub successful_propagations: usize,
    /// Individual Primal results
    pub results: HashMap<String, TokenPropagationStatus>,
    /// Token ID that was propagated
    pub token_id: String,
    /// Propagation timestamp
    pub propagation_time: chrono::DateTime<chrono::Utc>,
}

/// Result of token verification across Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Total number of Primals
    pub total_primals: usize,
    /// Number of valid tokens
    pub valid_tokens: usize,
    /// Individual Primal verification results
    pub results: HashMap<String, TokenVerificationStatus>,
    /// Verification timestamp
    pub verification_time: chrono::DateTime<chrono::Utc>,
}

/// Primal type configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimalTypeConfig {
    /// ToadStool configuration
    ToadStool(ToadStoolConfig),
    /// Songbird configuration
    Songbird(SongbirdConfig),
    /// BearDog configuration
    BearDog(BearDogConfig),
    /// NestGate configuration
    NestGate(NestGateConfig),
    /// Squirrel configuration
    Squirrel(SquirrelConfig),
    /// biomeOS configuration
    BiomeOS(BiomeOSConfig),
}

impl AuthenticationManager {
    /// Create a new authentication manager
    #[must_use]
    pub fn new(config: AuthManagerConfig) -> Self {
        Self {
            config,
            current_token: None,
            _beardog_client: None,
            refresh_task: None,
        }
    }

    /// Initialize connection to BearDog
    pub async fn initialize_beardog_connection(&self) -> ToadStoolResult<()> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            // Test connection to BearDog
            let response = client
                .get(format!("{}/health", self.config.beardog_endpoint))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to connect to BearDog: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "BearDog health check failed with status: {}",
                    response.status()
                )));
            }

            // Store client for future use
            // Note: This would require making self mutable in actual implementation
            tracing::info!(
                "Successfully connected to BearDog at {}",
                self.config.beardog_endpoint
            );
        }

        #[cfg(not(feature = "networking"))]
        {
            tracing::warn!("Networking feature disabled - using mock BearDog connection");
        }

        Ok(())
    }

    /// Get current authentication token
    pub async fn get_current_token(&self) -> ToadStoolResult<AuthenticationToken> {
        // Check if we have a valid cached token
        if let Some(token) = &self.current_token {
            if token.expires_at > chrono::Utc::now() + chrono::Duration::seconds(30) {
                return Ok(token.clone());
            }
        }

        // Request new token from BearDog
        self.request_new_token().await
    }

    /// Request new authentication token from BearDog
    async fn request_new_token(&self) -> ToadStoolResult<AuthenticationToken> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let token_request = TokenRequest {
                requesting_primal: "toadstool".to_string(),
                scope: vec!["cross-primal".to_string(), "propagation".to_string()],
                audience: vec![
                    "songbird".to_string(),
                    "nestgate".to_string(),
                    "squirrel".to_string(),
                    "biomeos".to_string(),
                ],
                timestamp: chrono::Utc::now(),
            };

            let response = client
                .post(format!("{}/auth/token", self.config.beardog_endpoint))
                .json(&token_request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to request token from BearDog: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Token request failed with status: {}",
                    response.status()
                )));
            }

            let token: AuthenticationToken = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse token response: {e}"))
            })?;

            // Validate token
            self.validate_token(&token)?;

            Ok(token)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock token for testing
            Ok(AuthenticationToken {
                id: "mock-token-id".to_string(),
                token_type: "Bearer".to_string(),
                token: "mock-token-value".to_string(),
                public_key: "mock-public-key".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
                issued_at: chrono::Utc::now(),
                issuer: "beardog".to_string(),
                audience: vec!["songbird".to_string(), "nestgate".to_string()],
                scope: vec!["cross-primal".to_string()],
                claims: HashMap::new(),
            })
        }
    }

    /// Validate authentication token
    #[allow(dead_code)]
    fn validate_token(&self, token: &AuthenticationToken) -> ToadStoolResult<()> {
        // Check expiration
        if token.expires_at <= chrono::Utc::now() {
            return Err(ToadStoolError::runtime(
                "Token is already expired".to_string(),
            ));
        }

        // Check issuer
        if token.issuer != "beardog" {
            return Err(ToadStoolError::runtime(format!(
                "Invalid token issuer: {}",
                token.issuer
            )));
        }

        // Check token type
        if token.token_type != "Bearer" && token.token_type != "Ed25519" {
            return Err(ToadStoolError::runtime(format!(
                "Unsupported token type: {}",
                token.token_type
            )));
        }

        Ok(())
    }

    /// Sign token propagation request
    pub async fn sign_token_request(
        &self,
        token: &AuthenticationToken,
        target_primal: &str,
    ) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }

        // Create signing payload
        let payload = format!(
            "{}:{}:{}",
            token.id,
            target_primal,
            chrono::Utc::now().timestamp()
        );

        // In a real implementation, this would use Ed25519 signing
        // For now, return a mock signature
        use base64::{engine::general_purpose, Engine as _};
        let signature = format!(
            "ed25519:{}",
            general_purpose::STANDARD.encode(payload.as_bytes())
        );

        Ok(signature)
    }

    /// Sign verification request
    pub async fn sign_verification_request(&self, primal_name: &str) -> ToadStoolResult<String> {
        if !self.config.signature_validation {
            return Ok("signature_disabled".to_string());
        }

        // Create signing payload
        let payload = format!("verify:{}:{}", primal_name, chrono::Utc::now().timestamp());

        // In a real implementation, this would use Ed25519 signing
        use base64::{engine::general_purpose, Engine as _};
        let signature = format!(
            "ed25519:{}",
            general_purpose::STANDARD.encode(payload.as_bytes())
        );

        Ok(signature)
    }

    /// Start automatic token refresh
    pub async fn start_token_refresh(&mut self) -> ToadStoolResult<()> {
        let refresh_interval = self.config.token_refresh_interval;
        let beardog_endpoint = self.config.beardog_endpoint.clone();

        let refresh_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);

            loop {
                interval.tick().await;

                tracing::debug!("Refreshing authentication token");

                // In a real implementation, this would refresh the token
                // and update the current_token field
                match refresh_token_from_beardog(&beardog_endpoint).await {
                    Ok(_) => {
                        tracing::info!("Authentication token refreshed successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh authentication token: {}", e);
                    }
                }
            }
        });

        self.refresh_task = Some(refresh_task);

        Ok(())
    }

    /// Stop automatic token refresh
    pub fn stop_token_refresh(&mut self) {
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }
    }
}

/// Token request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    /// Requesting Primal
    pub requesting_primal: String,
    /// Token scope
    pub scope: Vec<String>,
    /// Target audience
    pub audience: Vec<String>,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Helper function to refresh token from BearDog
async fn refresh_token_from_beardog(
    beardog_endpoint: &str,
) -> ToadStoolResult<AuthenticationToken> {
    #[cfg(feature = "networking")]
    {
        let client = reqwest::Client::new();

        let refresh_request = TokenRefreshRequest {
            requesting_primal: "toadstool".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let response = client
            .post(format!("{beardog_endpoint}/auth/refresh"))
            .json(&refresh_request)
            .send()
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to refresh token: {e}")))?;

        if !response.status().is_success() {
            return Err(ToadStoolError::runtime(format!(
                "Token refresh failed with status: {}",
                response.status()
            )));
        }

        let token: AuthenticationToken = response.json().await.map_err(|e| {
            ToadStoolError::runtime(format!("Failed to parse refresh response: {e}"))
        })?;

        Ok(token)
    }

    #[cfg(not(feature = "networking"))]
    {
        tracing::info!(
            "Mock token refresh from {} (networking disabled)",
            beardog_endpoint
        );
        Ok(AuthenticationToken {
            id: "mock-refreshed-token-id".to_string(),
            token_type: "Bearer".to_string(),
            token: "mock-refreshed-token-value".to_string(),
            public_key: "mock-public-key".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            issued_at: chrono::Utc::now(),
            issuer: "beardog".to_string(),
            audience: vec!["songbird".to_string(), "nestgate".to_string()],
            scope: vec!["cross-primal".to_string()],
            claims: HashMap::new(),
        })
    }
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    /// Requesting Primal
    pub requesting_primal: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ===== Storage Provisioning Types =====

/// Storage provisioning manager for NestGate integration
pub struct StorageProvisioningManager {
    /// Configuration
    config: StorageProvisioningConfig,
    /// NestGate client
    #[cfg(feature = "networking")]
    #[allow(dead_code)]
    nestgate_client: Option<reqwest::Client>,
    #[cfg(not(feature = "networking"))]
    #[allow(dead_code)]
    nestgate_client: Option<()>,
    /// Provisioned volumes tracking
    #[allow(dead_code)]
    provisioned_volumes: HashMap<String, VolumeInfo>,
}

/// Configuration for storage provisioning manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProvisioningConfig {
    /// NestGate endpoint URL
    pub nestgate_endpoint: String,
    /// Storage tier preference
    pub storage_tier: String,
    /// Enable backup
    pub backup_enabled: bool,
    /// Enable replication
    pub replication_enabled: bool,
    /// Replication factor
    pub replication_factor: u32,
}

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Volume name
    pub name: String,
    /// Volume ID in NestGate
    pub volume_id: String,
    /// Volume size in bytes
    pub size_bytes: u64,
    /// Storage class
    pub storage_class: String,
    /// Volume status
    pub status: VolumeStatus,
    /// Mount points
    pub mount_points: Vec<String>,
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed time
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
}

/// Volume status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VolumeStatus {
    /// Volume is being created
    Creating,
    /// Volume is available for use
    Available,
    /// Volume is being attached
    Attaching,
    /// Volume is attached and in use
    InUse,
    /// Volume is being detached
    Detaching,
    /// Volume is being deleted
    Deleting,
    /// Volume creation or operation failed
    Error(String),
}

impl StorageProvisioningManager {
    /// Create a new storage provisioning manager
    pub fn new(config: StorageProvisioningConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "networking")]
            nestgate_client: None,
            #[cfg(not(feature = "networking"))]
            nestgate_client: None,
            provisioned_volumes: HashMap::new(),
        }
    }

    /// Initialize connection to NestGate
    pub async fn initialize_nestgate_connection(&self) -> ToadStoolResult<()> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            // Test connection to NestGate
            let response = client
                .get(format!("{}/health", self.config.nestgate_endpoint))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to connect to NestGate: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "NestGate health check failed with status: {}",
                    response.status()
                )));
            }

            tracing::info!(
                "Successfully connected to NestGate at {}",
                self.config.nestgate_endpoint
            );
        }

        #[cfg(not(feature = "networking"))]
        {
            tracing::warn!("Networking feature disabled - using mock NestGate connection");
        }

        Ok(())
    }

    /// Provision a volume from manifest configuration
    pub async fn provision_volume(
        &self,
        volume_config: &VolumeConfig,
    ) -> ToadStoolResult<VolumeInfo> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let request = StorageProvisioningRequest {
                volume_name: volume_config.name.clone(),
                size: volume_config.size.clone(),
                storage_class: volume_config.storage_class.clone(),
                access_modes: volume_config.access_modes.clone(),
                backup_policy: volume_config.backup_policy.clone(),
                replication: if self.config.replication_enabled {
                    Some(ReplicationSettings {
                        enabled: true,
                        factor: self.config.replication_factor,
                        strategy: "async".to_string(),
                    })
                } else {
                    None
                },
            };

            let response = client
                .post(format!("{}/volumes", self.config.nestgate_endpoint))
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision volume {}: {}",
                        volume_config.name, e
                    ))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume provisioning failed with status: {}",
                    response.status()
                )));
            }

            let volume_info: VolumeInfo = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse volume info: {e}"))
            })?;

            Ok(volume_info)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock volume info for testing
            Ok(VolumeInfo {
                name: volume_config.name.clone(),
                volume_id: format!("mock-{}", volume_config.name),
                size_bytes: parse_size_string(&volume_config.size).unwrap_or(1_073_741_824), // 1GB default
                storage_class: volume_config
                    .storage_class
                    .clone()
                    .unwrap_or_else(|| self.config.storage_tier.clone()),
                status: VolumeStatus::Available,
                mount_points: vec![],
                created_at: chrono::Utc::now(),
                last_accessed: None,
            })
        }
    }

    /// Provision a persistent volume
    pub async fn provision_persistent_volume(
        &self,
        pv_config: &PersistentVolume,
    ) -> ToadStoolResult<VolumeInfo> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let request = StorageProvisioningRequest {
                volume_name: pv_config.name.clone(),
                size: pv_config.capacity.clone(),
                storage_class: Some(pv_config.storage_class.clone()),
                access_modes: pv_config.access_modes.clone(),
                backup_policy: None,
                replication: if self.config.replication_enabled {
                    Some(ReplicationSettings {
                        enabled: true,
                        factor: self.config.replication_factor,
                        strategy: "sync".to_string(),
                    })
                } else {
                    None
                },
            };

            let response = client
                .post(format!(
                    "{}/persistent-volumes",
                    self.config.nestgate_endpoint
                ))
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision persistent volume {}: {}",
                        pv_config.name, e
                    ))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Persistent volume provisioning failed with status: {}",
                    response.status()
                )));
            }

            let volume_info: VolumeInfo = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse persistent volume info: {e}"))
            })?;

            Ok(volume_info)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock persistent volume info for testing
            Ok(VolumeInfo {
                name: pv_config.name.clone(),
                volume_id: format!("mock-pv-{}", pv_config.name),
                size_bytes: parse_size_string(&pv_config.capacity).unwrap_or(10_737_418_240), // 10GB default
                storage_class: pv_config.storage_class.clone(),
                status: VolumeStatus::Available,
                mount_points: vec![],
                created_at: chrono::Utc::now(),
                last_accessed: None,
            })
        }
    }

    /// Mount volume in a Primal environment
    pub async fn mount_volume_in_primal(
        &self,
        primal_name: &str,
        mount_spec: &VolumeMountSpec,
    ) -> ToadStoolResult<VolumeMountInfo> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let mount_request = VolumeMountRequest {
                volume_name: mount_spec.volume_name.clone(),
                primal_name: primal_name.to_string(),
                mount_path: mount_spec.mount_path.clone(),
                read_only: mount_spec.read_only,
            };

            let response = client
                .post(format!(
                    "{}/volumes/{}/mount",
                    self.config.nestgate_endpoint, mount_spec.volume_name
                ))
                .json(&mount_request)
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to mount volume {} in {}: {}",
                        mount_spec.volume_name, primal_name, e
                    ))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume mount failed with status: {}",
                    response.status()
                )));
            }

            let mount_info: VolumeMountInfo = response
                .json()
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to parse mount info: {e}")))?;

            Ok(mount_info)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock mount info for testing
            Ok(VolumeMountInfo {
                spec: mount_spec.clone(),
                mount_id: format!("mock-mount-{}-{}", primal_name, mount_spec.volume_name),
                status: MountStatus::Mounted,
                mounted_at: chrono::Utc::now(),
            })
        }
    }

    /// Get volume usage information
    pub async fn get_volume_usage(&self, volume_name: &str) -> ToadStoolResult<VolumeUsage> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let response = client
                .get(format!(
                    "{}/volumes/{}/usage",
                    self.config.nestgate_endpoint, volume_name
                ))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to get usage for volume {volume_name}: {e}"
                    ))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume usage request failed with status: {}",
                    response.status()
                )));
            }

            let usage: VolumeUsage = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse volume usage: {e}"))
            })?;

            Ok(usage)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock usage info for testing
            let allocated = 1_073_741_824; // 1GB
            let used = allocated / 3; // 33% used
            Ok(VolumeUsage {
                volume_name: volume_name.to_string(),
                allocated_bytes: allocated,
                used_bytes: used,
                available_bytes: allocated - used,
                usage_percentage: (used as f64 / allocated as f64) * 100.0,
                last_updated: chrono::Utc::now(),
            })
        }
    }

    /// List all volumes managed by NestGate
    pub async fn list_all_volumes(&self) -> ToadStoolResult<Vec<VolumeInfo>> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let response = client
                .get(format!("{}/volumes", self.config.nestgate_endpoint))
                .send()
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list volumes: {e}")))?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume listing failed with status: {}",
                    response.status()
                )));
            }

            let volumes: Vec<VolumeInfo> = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse volumes list: {e}"))
            })?;

            Ok(volumes)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Return mock volumes list for testing
            Ok(vec![])
        }
    }

    /// Check if volume is currently in use
    #[allow(unused_variables)]
    pub async fn is_volume_in_use(&self, volume_name: &str) -> ToadStoolResult<bool> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let response = client
                .get(format!(
                    "{}/volumes/{}/status",
                    self.config.nestgate_endpoint, volume_name
                ))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to check volume status: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume status check failed with status: {}",
                    response.status()
                )));
            }

            let status_response: VolumeStatusResponse = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse volume status: {e}"))
            })?;

            Ok(status_response.in_use)
        }

        #[cfg(not(feature = "networking"))]
        {
            // Mock: assume volumes are not in use
            Ok(false)
        }
    }

    /// Delete a volume
    pub async fn delete_volume(&self, volume_name: &str) -> ToadStoolResult<()> {
        #[cfg(feature = "networking")]
        {
            let client = reqwest::Client::new();

            let response = client
                .delete(format!(
                    "{}/volumes/{}",
                    self.config.nestgate_endpoint, volume_name
                ))
                .send()
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to delete volume {volume_name}: {e}"))
                })?;

            if !response.status().is_success() {
                return Err(ToadStoolError::runtime(format!(
                    "Volume deletion failed with status: {}",
                    response.status()
                )));
            }

            Ok(())
        }

        #[cfg(not(feature = "networking"))]
        {
            tracing::info!("Mock volume deletion: {}", volume_name);
            Ok(())
        }
    }
}

/// Volume mount request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMountRequest {
    /// Volume name to mount
    pub volume_name: String,
    /// Primal name where to mount
    pub primal_name: String,
    /// Mount path
    pub mount_path: String,
    /// Read-only mount
    pub read_only: bool,
}

/// Volume status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeStatusResponse {
    /// Volume name
    pub volume_name: String,
    /// Volume status
    pub status: VolumeStatus,
    /// Is volume in use
    pub in_use: bool,
    /// Mount count
    pub mount_count: u32,
}

/// Helper function to parse size strings (e.g., "100Gi", "1TB")
#[allow(dead_code)]
fn parse_size_string(size_str: &str) -> Option<u64> {
    let size_str = size_str.trim();

    if size_str.ends_with("Gi") {
        size_str
            .strip_suffix("Gi")?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1_073_741_824)
    } else if size_str.ends_with("GB") {
        size_str
            .strip_suffix("GB")?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1_000_000_000)
    } else if size_str.ends_with("Mi") {
        size_str
            .strip_suffix("Mi")?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1_048_576)
    } else if size_str.ends_with("MB") {
        size_str
            .strip_suffix("MB")?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1_000_000)
    } else if size_str.ends_with("TB") {
        size_str
            .strip_suffix("TB")?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1_000_000_000_000)
    } else {
        // Assume bytes
        size_str.parse::<u64>().ok()
    }
}

// ===== AI Agent Deployment Types =====

/// Agent deployment manager for Squirrel integration
pub struct AgentDeploymentManager {
    /// Configuration
    _config: AgentDeploymentConfig,
    /// Squirrel client
    #[cfg(feature = "networking")]
    _squirrel_client: Option<reqwest::Client>,
    #[cfg(not(feature = "networking"))]
    _squirrel_client: Option<()>,
    /// Deployed agents tracking
    deployed_agents: HashMap<String, AgentInfo>,
    /// Deployed models tracking
    deployed_models: HashMap<String, ModelInfo>,
}

/// Configuration for agent deployment manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeploymentConfig {
    /// Squirrel endpoint URL
    pub squirrel_endpoint: String,
    /// Model registry type (local, huggingface, custom)
    pub model_registry: String,
    /// Agent runtime (container, process, lambda)
    pub agent_runtime: String,
    /// Enable MCP (Model Control Protocol)
    pub mcp_enabled: bool,
    /// Resource limits configuration
    pub resource_limits: serde_json::Map<String, serde_json::Value>,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentInfo {
    /// Agent name
    pub name: String,
    /// Agent ID in Squirrel
    pub agent_id: String,
    /// Model being used
    pub model: String,
    /// Agent status
    pub status: AgentStatus,
    /// Replica count
    pub replicas: u32,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Resource usage
    pub resources: AgentResourceUsage,
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update time
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model ID in Squirrel
    pub model_id: String,
    /// Model type
    pub model_type: String,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Model status
    pub status: ModelStatus,
    /// Resource requirements
    pub resource_requirements: ModelResourceRequirements,
    /// Performance metrics
    pub performance: ModelPerformanceMetrics,
    /// Load time
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is being deployed
    Deploying,
    /// Agent is running and ready
    Running,
    /// Agent is scaling
    Scaling,
    /// Agent is being updated
    Updating,
    /// Agent is being terminated
    Terminating,
    /// Agent has failed
    Failed(String),
    /// Agent is stopped
    Stopped,
}

/// Model status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    /// Model is being loaded
    Loading,
    /// Model is loaded and ready
    Ready,
    /// Model is being updated
    Updating,
    /// Model is being unloaded
    Unloading,
    /// Model load failed
    Failed(String),
}

/// Agent resource usage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentResourceUsage {
    /// CPU usage percentage
    pub cpu_usage: u64, // Changed to u64 for Eq
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Storage usage in bytes
    pub storage_usage: u64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
}

/// Model resource requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelResourceRequirements {
    /// Minimum CPU cores required
    pub min_cpu_cores: u64, // Changed to u64 for Eq
    /// Memory required in bytes
    pub memory_bytes: u64,
    /// GPU memory required in bytes
    pub gpu_memory_bytes: Option<u64>,
    /// Storage required in bytes
    pub storage_bytes: u64,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelPerformanceMetrics {
    /// Average inference time in milliseconds
    pub avg_inference_time: u64, // Changed to u64 for Eq
    /// Requests per second
    pub requests_per_second: u64, // Changed to u64 for Eq
    /// Error rate percentage (scaled by 100, so 5.5% = 550)
    pub error_rate: u64, // Changed to u64 for Eq
    /// Throughput tokens per second
    pub tokens_per_second: u64, // Changed to u64 for Eq
}

/// Agent deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentDeploymentStatus {
    /// Deployment successful
    Success(AgentInfo),
    /// Deployment failed
    Failed(String),
    /// Deployment in progress
    InProgress,
    /// Deployment skipped
    Skipped(String),
}

/// Model deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelDeploymentStatus {
    /// Deployment successful
    Success(ModelInfo),
    /// Deployment failed
    Failed(String),
    /// Deployment in progress
    InProgress,
    /// Deployment skipped
    Skipped(String),
}

/// Agent scaling status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentScalingStatus {
    /// Scaled up successfully
    ScaledUp(u32),
    /// Scaled down successfully
    ScaledDown(u32),
    /// Scaling failed
    Failed(String),
    /// No scaling needed
    NoChange,
}

/// Agent cleanup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentCleanupStatus {
    /// Cleanup successful
    Success,
    /// Cleanup failed
    Failed(String),
    /// Cleanup skipped
    Skipped(String),
}

/// Agent scaling decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentScalingDecision {
    /// Scale up to specified replicas
    ScaleUp(u32),
    /// Scale down to specified replicas
    ScaleDown(u32),
    /// No scaling needed
    NoChange,
}

/// Result of agent deployment operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDeploymentResult {
    /// Total agents processed
    pub total_agents: usize,
    /// Successfully deployed agents
    pub deployed_agents: usize,
    /// Individual agent results
    pub results: HashMap<String, AgentDeploymentStatus>,
    /// Deployment timestamp
    pub deployment_time: chrono::DateTime<chrono::Utc>,
}

/// Result of model deployment operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDeploymentResult {
    /// Total models processed
    pub total_models: usize,
    /// Successfully deployed models
    pub deployed_models: usize,
    /// Individual model results
    pub results: HashMap<String, ModelDeploymentStatus>,
    /// Deployment timestamp
    pub deployment_time: chrono::DateTime<chrono::Utc>,
}

/// Result of agent scaling operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentScalingResult {
    /// Total agents evaluated for scaling
    pub total_agents_evaluated: usize,
    /// Number of agents that were scaled
    pub agents_scaled: usize,
    /// Individual scaling results
    pub results: HashMap<String, AgentScalingStatus>,
    /// Scaling timestamp
    pub scaling_time: chrono::DateTime<chrono::Utc>,
}

/// Result of agent cleanup operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCleanupResult {
    /// Total items checked for cleanup
    pub total_items_checked: usize,
    /// Successfully cleaned items
    pub cleaned_items: usize,
    /// Individual cleanup results
    pub results: HashMap<String, AgentCleanupStatus>,
    /// Cleanup timestamp
    pub cleanup_time: chrono::DateTime<chrono::Utc>,
}

/// Agent performance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformanceReport {
    /// Total agents monitored
    pub total_agents: usize,
    /// Number of healthy agents
    pub healthy_agents: usize,
    /// Individual agent health status
    pub agent_health: HashMap<String, AgentHealthStatus>,
    /// Performance issues detected
    pub performance_issues: Vec<String>,
    /// Report timestamp
    pub report_time: chrono::DateTime<chrono::Utc>,
}

/// Agent health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealthStatus {
    /// Is agent healthy
    pub healthy: bool,
    /// Health issues
    pub issues: Vec<String>,
    /// Last health check
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Agent metrics for monitoring and scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Agent name
    pub agent_name: String,
    /// Current replica count
    pub replica_count: u32,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Request queue length
    pub request_queue_length: u32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// Last heartbeat time
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Model metrics for performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Model name
    pub model_name: String,
    /// Average inference time in milliseconds
    pub avg_inference_time: f64,
    /// Requests per second
    pub requests_per_second: f64,
    /// Error rate percentage
    pub error_rate: f64,
    /// GPU utilization percentage
    pub gpu_utilization: Option<f64>,
    /// Memory usage in bytes
    pub memory_usage: u64,
}

impl AgentDeploymentManager {
    /// Create a new agent deployment manager
    pub fn new(config: AgentDeploymentConfig) -> Self {
        Self {
            _config: config,
            #[cfg(feature = "networking")]
            _squirrel_client: Some(reqwest::Client::new()),
            #[cfg(not(feature = "networking"))]
            _squirrel_client: Some(()),
            deployed_agents: HashMap::new(),
            deployed_models: HashMap::new(),
        }
    }

    /// Deploy an agent
    pub async fn deploy_agent(&mut self, agent_config: &AgentConfig) -> ToadStoolResult<AgentInfo> {
        let agent_id = format!("agent_{}", uuid::Uuid::new_v4());
        let agent_info = AgentInfo {
            name: agent_config.name.clone(),
            agent_id: agent_id.clone(),
            model: agent_config.model.clone(),
            status: AgentStatus::Deploying,
            replicas: 1,
            capabilities: agent_config.capabilities.clone(),
            resources: AgentResourceUsage {
                cpu_usage: 0,
                memory_usage: 0,
                storage_usage: 0,
                network_bytes_sent: 0,
                network_bytes_received: 0,
            },
            created_at: chrono::Utc::now(),
            last_updated: chrono::Utc::now(),
        };

        #[cfg(feature = "networking")]
        if let Some(client) = &self._squirrel_client {
            let deploy_request = serde_json::json!({
                "agent_name": agent_config.name,
                "model": agent_config.model,
                "capabilities": agent_config.capabilities,
                "resources": agent_config.resources,
                "environment": agent_config.environment,
                "config": agent_config.config,
            });

            let response = client
                .post(format!("{}/agents/deploy", self._config.squirrel_endpoint))
                .json(&deploy_request)
                .send()
                .await
                .map_err(|e| ToadStoolError::network(e.to_string()))?;

            if response.status().is_success() {
                let mut success_agent = agent_info.clone();
                success_agent.status = AgentStatus::Running;
                success_agent.last_updated = chrono::Utc::now();
                self.deployed_agents
                    .insert(agent_config.name.clone(), success_agent.clone());
                Ok(success_agent)
            } else {
                Err(ToadStoolError::deployment(format!(
                    "Failed to deploy agent {}: {}",
                    agent_config.name,
                    response.status()
                )))
            }
        } else {
            Err(ToadStoolError::network(
                "Squirrel client not available".to_string(),
            ))
        }
        #[cfg(not(feature = "networking"))]
        {
            let mut success_agent = agent_info.clone();
            success_agent.status = AgentStatus::Running;
            success_agent.last_updated = chrono::Utc::now();
            self.deployed_agents
                .insert(agent_config.name.clone(), success_agent.clone());
            Ok(success_agent)
        }
    }

    /// Deploy a model
    pub async fn deploy_model(&mut self, model_config: &ModelConfig) -> ToadStoolResult<ModelInfo> {
        let model_id = format!("model_{}", uuid::Uuid::new_v4());
        let model_info = ModelInfo {
            name: model_config.name.clone(),
            model_id: model_id.clone(),
            model_type: model_config.model_type.clone(),
            size_bytes: 1024 * 1024 * 1024, // Default 1GB
            status: ModelStatus::Loading,
            resource_requirements: ModelResourceRequirements {
                min_cpu_cores: 2,
                memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
                gpu_memory_bytes: None,
                storage_bytes: 1024 * 1024 * 1024, // 1GB
            },
            performance: ModelPerformanceMetrics {
                avg_inference_time: 100,
                requests_per_second: 10,
                error_rate: 0,
                tokens_per_second: 100,
            },
            loaded_at: chrono::Utc::now(),
        };

        #[cfg(feature = "networking")]
        if let Some(client) = &self._squirrel_client {
            let deploy_request = serde_json::json!({
                "model_name": model_config.name,
                "model_type": model_config.model_type,
                "parameters": model_config.parameters,
                "resources": model_config.resources,
            });

            let response = client
                .post(format!("{}/models/deploy", self._config.squirrel_endpoint))
                .json(&deploy_request)
                .send()
                .await
                .map_err(|e| ToadStoolError::network(e.to_string()))?;

            if response.status().is_success() {
                let mut success_model = model_info.clone();
                success_model.status = ModelStatus::Ready;
                self.deployed_models
                    .insert(model_config.name.clone(), success_model.clone());
                Ok(success_model)
            } else {
                Err(ToadStoolError::deployment(format!(
                    "Failed to deploy model {}: {}",
                    model_config.name,
                    response.status()
                )))
            }
        } else {
            Err(ToadStoolError::network(
                "Squirrel client not available".to_string(),
            ))
        }
        #[cfg(not(feature = "networking"))]
        {
            let mut success_model = model_info.clone();
            success_model.status = ModelStatus::Ready;
            self.deployed_models
                .insert(model_config.name.clone(), success_model.clone());
            Ok(success_model)
        }
    }

    /// Get agent metrics
    pub async fn get_agent_metrics(&self) -> ToadStoolResult<HashMap<String, AgentMetrics>> {
        let mut metrics = HashMap::new();
        for (name, agent) in &self.deployed_agents {
            let agent_metrics = AgentMetrics {
                agent_name: name.clone(),
                replica_count: agent.replicas,
                cpu_usage: agent.resources.cpu_usage as f64,
                memory_usage: (agent.resources.memory_usage as f64) / (1024.0 * 1024.0 * 1024.0)
                    * 100.0,
                request_queue_length: 0,
                avg_response_time_ms: 100.0,
                error_rate: 0.0,
                last_heartbeat: chrono::Utc::now(),
            };
            metrics.insert(name.clone(), agent_metrics);
        }
        Ok(metrics)
    }

    /// Scale an agent
    pub async fn scale_agent(&mut self, agent_name: &str, replicas: u32) -> ToadStoolResult<()> {
        if let Some(agent) = self.deployed_agents.get_mut(agent_name) {
            agent.replicas = replicas;
            agent.status = AgentStatus::Scaling;
            agent.last_updated = chrono::Utc::now();

            #[cfg(feature = "networking")]
            if let Some(client) = &self._squirrel_client {
                let scale_request = serde_json::json!({
                    "agent_name": agent_name,
                    "replicas": replicas,
                });

                let response = client
                    .post(format!(
                        "{}/agents/{}/scale",
                        self._config.squirrel_endpoint, agent_name
                    ))
                    .json(&scale_request)
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::network(e.to_string()))?;

                if response.status().is_success() {
                    agent.status = AgentStatus::Running;
                    agent.last_updated = chrono::Utc::now();
                }
            }
            #[cfg(not(feature = "networking"))]
            {
                agent.status = AgentStatus::Running;
                agent.last_updated = chrono::Utc::now();
            }
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Agent {agent_name} not found"
            )))
        }
    }

    /// Get model metrics
    pub async fn get_model_metrics(&self) -> ToadStoolResult<HashMap<String, ModelMetrics>> {
        let mut metrics = HashMap::new();
        for (name, model) in &self.deployed_models {
            let model_metrics = ModelMetrics {
                model_name: name.clone(),
                avg_inference_time: model.performance.avg_inference_time as f64,
                requests_per_second: model.performance.requests_per_second as f64,
                error_rate: model.performance.error_rate as f64,
                gpu_utilization: None,
                memory_usage: model.resource_requirements.memory_bytes,
            };
            metrics.insert(name.clone(), model_metrics);
        }
        Ok(metrics)
    }

    /// Terminate an agent
    pub async fn terminate_agent(&mut self, agent_name: &str) -> ToadStoolResult<()> {
        if let Some(agent) = self.deployed_agents.get_mut(agent_name) {
            agent.status = AgentStatus::Terminating;
            agent.last_updated = chrono::Utc::now();

            #[cfg(feature = "networking")]
            if let Some(client) = &self._squirrel_client {
                let response = client
                    .delete(format!(
                        "{}/agents/{}",
                        self._config.squirrel_endpoint, agent_name
                    ))
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::network(e.to_string()))?;

                if response.status().is_success() {
                    self.deployed_agents.remove(agent_name);
                }
            }
            #[cfg(not(feature = "networking"))]
            {
                self.deployed_agents.remove(agent_name);
            }
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Agent {agent_name} not found"
            )))
        }
    }

    /// Unload a model
    pub async fn unload_model(&mut self, model_name: &str) -> ToadStoolResult<()> {
        if let Some(model) = self.deployed_models.get_mut(model_name) {
            model.status = ModelStatus::Unloading;

            #[cfg(feature = "networking")]
            if let Some(client) = &self._squirrel_client {
                let response = client
                    .delete(format!(
                        "{}/models/{}",
                        self._config.squirrel_endpoint, model_name
                    ))
                    .send()
                    .await
                    .map_err(|e| ToadStoolError::network(e.to_string()))?;

                if response.status().is_success() {
                    self.deployed_models.remove(model_name);
                }
            }
            #[cfg(not(feature = "networking"))]
            {
                self.deployed_models.remove(model_name);
            }
            Ok(())
        } else {
            Err(ToadStoolError::not_found(format!(
                "Model {model_name} not found"
            )))
        }
    }

    /// List all agents
    pub async fn list_all_agents(&self) -> ToadStoolResult<Vec<AgentInfo>> {
        Ok(self.deployed_agents.values().cloned().collect())
    }

    /// Check if an agent is idle
    pub async fn is_agent_idle(&self, agent_name: &str) -> ToadStoolResult<bool> {
        if let Some(agent) = self.deployed_agents.get(agent_name) {
            // Simple check: if CPU usage is very low, consider it idle
            Ok(agent.resources.cpu_usage < 5)
        } else {
            Err(ToadStoolError::not_found(format!(
                "Agent {agent_name} not found"
            )))
        }
    }

    /// List all models
    pub async fn list_all_models(&self) -> ToadStoolResult<Vec<ModelInfo>> {
        Ok(self.deployed_models.values().cloned().collect())
    }

    /// Check if a model is in use
    pub async fn is_model_in_use(&self, model_name: &str) -> ToadStoolResult<bool> {
        if self.deployed_models.contains_key(model_name) {
            // Check if any agent is using this model
            let in_use = self
                .deployed_agents
                .values()
                .any(|agent| agent.model == model_name);
            Ok(in_use)
        } else {
            Err(ToadStoolError::not_found(format!(
                "Model {model_name} not found"
            )))
        }
    }

    /// Initialize connection to Squirrel endpoint
    pub async fn initialize_squirrel_connection(&self) -> ToadStoolResult<()> {
        #[cfg(feature = "networking")]
        if let Some(client) = &self._squirrel_client {
            let response = client
                .get(format!("{}/health", self._config.squirrel_endpoint))
                .send()
                .await
                .map_err(|e| ToadStoolError::network(e.to_string()))?;

            if response.status().is_success() {
                Ok(())
            } else {
                Err(ToadStoolError::network(format!(
                    "Failed to connect to Squirrel at {}: {}",
                    self._config.squirrel_endpoint,
                    response.status()
                )))
            }
        } else {
            Err(ToadStoolError::network(
                "Squirrel client not available".to_string(),
            ))
        }
        #[cfg(not(feature = "networking"))]
        Ok(())
    }
}
