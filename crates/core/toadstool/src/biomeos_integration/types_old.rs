//! # BiomeOS Integration Types
//!
//! This module contains 27+ configuration structures for BiomeOS integration,
//! organized into three functional domains: Authentication, Storage, and Agent Management.
//!
//! ## Configuration Domains
//!
//! ### Authentication Configuration (8 configs)
//! Token-based authentication and session management for BiomeOS integration:
//! - [`BiomeSecurity`] - Security policies and authentication settings
//! - `AuthToken` - Authentication token structure (planned)
//! - `TokenConfig` - Token generation and validation (planned)
//! - `SessionConfig` - Session management (planned)
//! - `PermissionModel` - Role-based access control (planned)
//!
//! ### Storage Configuration (10 configs)
//! S3-compatible object storage integration with BearDog/NestGate:
//! - [`BiomeStorage`] - Overall storage configuration
//! - `StorageBucket` - Bucket configuration and provisioning (planned)
//! - `ObjectPolicy` - Object access policies (planned)
//! - [`ReplicationConfig`] - Cross-cluster replication
//! - [`BackupConfig`] - Backup and recovery settings
//!
//! ### Agent Configuration (9 configs)
//! AI agent deployment, lifecycle management, and communication:
//! - [`AgentConfig`] - Agent deployment configuration
//! - `AgentLifecycle` - Lifecycle management (start, stop, restart) (planned)
//! - `AgentCommunication` - Inter-agent communication settings (planned)
//! - `AgentResources` - Resource allocation for agents (planned)
//! - `AgentMonitoring` - Health and performance monitoring (planned)
//!
//! ## Integration Patterns
//!
//! ### Basic BiomeOS Authentication
//! ```rust,ignore
//! use toadstool::biomeos_integration::*;
//!
//! let auth_config = BiomeSecurity {
//!     authentication: AuthConfig {
//!         provider: "beardog".to_string(),
//!         endpoint: "http://beardog:8081".to_string(),
//!         token_ttl: Duration::from_secs(3600),
//!     },
//!     team_isolation: true,
//!     permission_model: PermissionModel::RoleBased,
//! };
//! ```
//!
//! ### Storage Integration
//! ```rust,ignore
//! let storage = BiomeStorage {
//!     backend: "nestgate".to_string(),
//!     endpoint: "s3://nestgate:9000".to_string(),
//!     buckets: vec![
//!         StorageBucket {
//!             name: "biome-data".to_string(),
//!             access_policy: ObjectPolicy::TeamPrivate,
//!             versioning: true,
//!             replication: Some(ReplicationConfig {
//!                 enabled: true,
//!                 target_clusters: vec!["cluster-b".to_string()],
//!             }),
//!         },
//!     ],
//!     credentials: StorageCredentials::EnvVar,
//! };
//! ```
//!
//! ### Agent Deployment
//! ```rust,ignore
//! let agent = AgentConfig {
//!     name: "data-processor".to_string(),
//!     image: "biome/agent:latest".to_string(),
//!     resources: AgentResources {
//!         cpu_cores: 2.0,
//!         memory_gb: 4.0,
//!         gpu: None,
//!     },
//!     communication: AgentCommunication {
//!         protocol: "grpc".to_string(),
//!         port: 9090,
//!         tls_enabled: true,
//!     },
//!     lifecycle: AgentLifecycle {
//!         startup_timeout: Duration::from_secs(60),
//!         shutdown_timeout: Duration::from_secs(30),
//!         restart_policy: "on-failure".to_string(),
//!     },
//! };
//! ```
//!
//! ## Manifest Structure
//!
//! BiomeOS uses a Kubernetes-inspired manifest format:
//! ```yaml
//! apiVersion: biomeOS/v1
//! kind: Biome
//! metadata:
//!   name: my-biome
//!   team: engineering
//! primals:
//!   toadstool:
//!     enabled: true
//!     orchestrator: true
//!   beardog:
//!     enabled: true
//!   nestgate:
//!     enabled: true
//! storage:
//!   backend: nestgate
//!   buckets: [...]
//! agents:
//!   - name: agent-1
//!     image: my-agent:latest
//! ```
//!
//! ## Type Relationships
//!
//! ```text
//! BiomeManifest (root)
//! ├── BiomeMetadata
//! ├── PrimalsConfig
//! │   ├── ToadStoolConfig
//! │   ├── SongbirdConfig
//! │   ├── BearDogConfig
//! │   ├── NestGateConfig
//! │   └── SquirrelConfig
//! ├── BiomeStorage
//! │   └── StorageBucket[]
//! ├── AgentConfig[]
//! ├── BiomeSecurity
//! └── BiomeNetworking
//! ```

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Enhanced `BiomeManifest` structure for Phase 4 Universal Orchestration
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
                    health_checks: Some(BiomeHealthCheckConfig {
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
    /// `ToadStool` (Universal Compute) configuration
    pub toadstool: Option<ToadStoolConfig>,
    /// Songbird (Network Coordination) configuration
    pub songbird: Option<SongbirdConfig>,
    /// `BearDog` (Security) configuration
    pub beardog: Option<BearDogConfig>,
    /// `NestGate` (Storage) configuration
    pub nestgate: Option<NestGateConfig>,
    /// Squirrel (AI) configuration
    pub squirrel: Option<SquirrelConfig>,
    /// biomeOS (Universal OS) configuration
    pub biomeos: Option<BiomeOSConfig>,
}

/// `ToadStool` Universal Compute configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToadStoolConfig {
    /// Enable `ToadStool`
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
    pub health_checks: Option<BiomeHealthCheckConfig>,
    /// Configuration overrides
    pub config: HashMap<String, serde_json::Value>,
}

/// `BearDog` Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogConfig {
    /// Enable `BearDog`
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

/// `NestGate` Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateConfig {
    /// Enable `NestGate`
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

/// Health check configuration for biomeOS integration
///
/// # Design Note
///
/// This is **intentionally distinct** from `toadstool_common::config_bases::HealthCheckConfig`.
/// The base HealthCheckConfig is designed for HTTP/network service health checks with concepts
/// like healthy_threshold, unhealthy_threshold, and HTTP-specific parameters.
///
/// BiomeHealthCheckConfig is for **biome-level orchestration** health checks, which operate at
/// a higher level (checking if entire primals/services are operational) and use different
/// semantics (initial_delay for startup grace periods, simple retry counts).
///
/// **Key Differences**:
/// - Base: Service-level HTTP health checks (path, status code, thresholds)
/// - Biome: Orchestration-level readiness checks (initial delay, simple retries)
///
/// This separation maintains clean bounded contexts and prevents mixing concerns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeHealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    /// Health check timeout
    pub timeout: Duration,
    /// Number of retries before marking as unhealthy
    pub retries: u32,
    /// Initial delay before first check (startup grace period)
    pub initial_delay: Duration,
}

impl Default for BiomeHealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            retries: 3,
            initial_delay: Duration::from_secs(30),
        }
    }
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
    /// Enable `NestGate` integration
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
    /// Enable `BearDog` integration
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
    pub health_check: Option<BiomeHealthCheckConfig>,
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

/// Storage provisioning request to `NestGate`
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

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Volume name
    pub name: String,
    /// Volume ID
    pub id: String,
    /// Size
    pub size: String,
    /// Storage class
    pub storage_class: String,
    /// Status
    pub status: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
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
