//! # Security Manager
//!
//! Handles capability-based security, sandboxing, and service isolation.

use crate::manifest::{ServiceConfig, SecurityConfig};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::{debug, error, info, warn};

/// Security-specific errors
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Capability violation: {capability}")]
    CapabilityViolation { capability: String },
    
    #[error("Sandbox creation failed: {reason}")]
    SandboxCreationFailed { reason: String },
    
    #[error("Security policy violation: {policy}")]
    PolicyViolation { policy: String },
    
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Authorization failed: {reason}")]
    AuthorizationFailed { reason: String },
    
    #[error("Invalid security configuration: {message}")]
    InvalidConfiguration { message: String },
    
    #[error("Seccomp error: {0}")]
    Seccomp(String),
    
    #[error("Capability error: {0}")]
    Capability(String),
}

/// Security context for a service
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub service_name: String,
    pub capabilities: Vec<Capability>,
    pub sandbox_config: SandboxConfig,
    pub user_context: UserContext,
    pub network_policy: NetworkPolicy,
    pub file_system_policy: FileSystemPolicy,
}

/// Capability definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub scope: CapabilityScope,
    pub permissions: Vec<Permission>,
    pub constraints: Option<CapabilityConstraints>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CapabilityScope {
    Network,
    FileSystem,
    System,
    Process,
    Memory,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Execute,
    Create,
    Delete,
    Modify,
    Access,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConstraints {
    pub max_file_size: Option<u64>,
    pub allowed_paths: Option<Vec<String>>,
    pub allowed_ports: Option<Vec<u16>>,
    pub allowed_protocols: Option<Vec<String>>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub sandbox_type: SandboxType,
    pub isolation_level: IsolationLevel,
    pub resource_limits: SandboxResourceLimits,
    pub mount_points: Vec<MountPoint>,
    pub environment_variables: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum SandboxType {
    Wasm,
    Container,
    Chroot,
    Namespace,
}

#[derive(Debug, Clone)]
pub enum IsolationLevel {
    None,
    Basic,
    Strict,
    Complete,
}

#[derive(Debug, Clone)]
pub struct SandboxResourceLimits {
    pub max_memory: Option<u64>,
    pub max_cpu_time: Option<std::time::Duration>,
    pub max_file_descriptors: Option<u32>,
    pub max_processes: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MountPoint {
    pub source: PathBuf,
    pub target: PathBuf,
    pub read_only: bool,
    pub mount_type: MountType,
}

#[derive(Debug, Clone)]
pub enum MountType {
    Bind,
    Tmpfs,
    Proc,
    Sys,
}

/// User context for service execution
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Option<u32>,
    pub group_id: Option<u32>,
    pub supplementary_groups: Vec<u32>,
    pub home_directory: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
}

/// Network security policy
#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    pub enabled: bool,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<String>,
    pub allowed_addresses: Vec<String>,
    pub blocked_addresses: Vec<String>,
    pub firewall_rules: Vec<FirewallRule>,
}

#[derive(Debug, Clone)]
pub struct FirewallRule {
    pub action: FirewallAction,
    pub protocol: String,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone)]
pub enum FirewallAction {
    Allow,
    Deny,
    Drop,
}

/// File system security policy
#[derive(Debug, Clone)]
pub struct FileSystemPolicy {
    pub enabled: bool,
    pub allowed_paths: Vec<PathBuf>,
    pub read_only_paths: Vec<PathBuf>,
    pub blocked_paths: Vec<PathBuf>,
    pub max_file_size: Option<u64>,
    pub max_total_size: Option<u64>,
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub service_name: String,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub message: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    CapabilityViolation,
    PolicyViolation,
    AuthenticationFailure,
    AuthorizationFailure,
    SandboxBreach,
    ResourceLimitExceeded,
    SuspiciousActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Main security manager
pub struct SecurityManager {
    default_policy: SecurityPolicy,
    service_policies: HashMap<String, SecurityPolicy>,
    audit_log: Vec<SecurityAuditEvent>,
    capability_registry: CapabilityRegistry,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub sandbox_enabled: bool,
    pub default_capabilities: Vec<String>,
    pub capability_constraints: HashMap<String, CapabilityConstraints>,
    pub isolation_level: IsolationLevel,
    pub audit_enabled: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            default_capabilities: vec![
                "fs.read:/tmp".to_string(),
                "fs.write:/tmp".to_string(),
            ],
            capability_constraints: HashMap::new(),
            isolation_level: IsolationLevel::Strict,
            audit_enabled: true,
        }
    }
}

/// Capability registry for managing available capabilities
pub struct CapabilityRegistry {
    capabilities: HashMap<String, CapabilityDefinition>,
}

#[derive(Debug, Clone)]
pub struct CapabilityDefinition {
    pub name: String,
    pub description: String,
    pub scope: CapabilityScope,
    pub required_permissions: Vec<Permission>,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone)]
pub enum SecurityLevel {
    Safe,
    Moderate,
    Dangerous,
    Critical,
}

impl SecurityManager {
    pub async fn new() -> Result<Self, SecurityError> {
        info!("Initializing security manager");
        
        let mut capability_registry = CapabilityRegistry::new();
        capability_registry.register_default_capabilities();
        
        Ok(Self {
            default_policy: SecurityPolicy::default(),
            service_policies: HashMap::new(),
            audit_log: Vec::new(),
            capability_registry,
        })
    }

    /// Create security context for a service
    pub async fn create_service_context(
        &self,
        service: &ServiceConfig,
        security_config: &Option<SecurityConfig>,
    ) -> Result<SecurityContext, SecurityError> {
        info!("Creating security context for service: {}", service.name);
        
        // Get security policy for service
        let policy = self.get_service_policy(&service.name);
        
        // Parse service capabilities
        let capabilities = self.parse_service_capabilities(service, &policy).await?;
        
        // Create sandbox configuration
        let sandbox_config = self.create_sandbox_config(service, security_config, &policy).await?;
        
        // Create user context
        let user_context = self.create_user_context(service, security_config).await?;
        
        // Create network policy
        let network_policy = self.create_network_policy(service, &capabilities).await?;
        
        // Create file system policy
        let file_system_policy = self.create_file_system_policy(service, &capabilities).await?;
        
        let context = SecurityContext {
            service_name: service.name.clone(),
            capabilities,
            sandbox_config,
            user_context,
            network_policy,
            file_system_policy,
        };
        
        // Audit context creation
        self.audit_event(SecurityAuditEvent {
            timestamp: chrono::Utc::now(),
            service_name: service.name.clone(),
            event_type: AuditEventType::SuspiciousActivity, // Context creation event
            severity: AuditSeverity::Low,
            message: "Security context created".to_string(),
            metadata: HashMap::new(),
        }).await;
        
        Ok(context)
    }

    /// Validate capability request
    pub async fn validate_capability(&self, service_name: &str, capability: &str) -> Result<bool, SecurityError> {
        debug!("Validating capability: {} for service: {}", capability, service_name);
        
        // Parse capability
        let parsed_capability = self.parse_capability(capability)?;
        
        // Check if capability is registered
        if !self.capability_registry.is_registered(&parsed_capability.name) {
            return Err(SecurityError::CapabilityViolation {
                capability: format!("Unknown capability: {}", parsed_capability.name),
            });
        }
        
        // Check service policy
        let policy = self.get_service_policy(service_name);
        
        // Validate against policy
        if !policy.default_capabilities.contains(capability) {
            // Check if capability is explicitly allowed
            if let Some(constraints) = policy.capability_constraints.get(&parsed_capability.name) {
                self.validate_capability_constraints(&parsed_capability, constraints)?;
            } else {
                return Err(SecurityError::CapabilityViolation {
                    capability: format!("Capability not allowed: {}", capability),
                });
            }
        }
        
        Ok(true)
    }

    /// Enforce security policy
    pub async fn enforce_policy(&self, context: &SecurityContext, operation: &SecurityOperation) -> Result<(), SecurityError> {
        debug!("Enforcing security policy for operation: {:?}", operation);
        
        match operation {
            SecurityOperation::FileAccess { path, access_type } => {
                self.enforce_file_access_policy(context, path, access_type).await?;
            }
            SecurityOperation::NetworkAccess { address, port, protocol } => {
                self.enforce_network_access_policy(context, address, *port, protocol).await?;
            }
            SecurityOperation::ProcessSpawn { command, args } => {
                self.enforce_process_spawn_policy(context, command, args).await?;
            }
            SecurityOperation::ResourceAccess { resource_type, operation } => {
                self.enforce_resource_access_policy(context, resource_type, operation).await?;
            }
        }
        
        Ok(())
    }

    /// Get security audit log
    pub async fn get_audit_log(&self) -> Vec<SecurityAuditEvent> {
        self.audit_log.clone()
    }

    /// Get security statistics
    pub async fn get_security_stats(&self) -> SecurityStats {
        SecurityStats {
            total_contexts: self.service_policies.len(),
            total_capabilities: self.capability_registry.capabilities.len(),
            audit_events: self.audit_log.len(),
            policy_violations: self.audit_log.iter()
                .filter(|e| matches!(e.event_type, AuditEventType::PolicyViolation))
                .count(),
            capability_violations: self.audit_log.iter()
                .filter(|e| matches!(e.event_type, AuditEventType::CapabilityViolation))
                .count(),
        }
    }

    // Private helper methods

    fn get_service_policy(&self, service_name: &str) -> &SecurityPolicy {
        self.service_policies.get(service_name).unwrap_or(&self.default_policy)
    }

    async fn parse_service_capabilities(
        &self,
        service: &ServiceConfig,
        policy: &SecurityPolicy,
    ) -> Result<Vec<Capability>, SecurityError> {
        let mut capabilities = Vec::new();
        
        // Add default capabilities
        for cap_str in &policy.default_capabilities {
            let capability = self.parse_capability(cap_str)?;
            capabilities.push(capability);
        }
        
        // Add service-specific capabilities
        for cap_str in &service.capabilities {
            let capability = self.parse_capability(cap_str)?;
            capabilities.push(capability);
        }
        
        Ok(capabilities)
    }

    fn parse_capability(&self, capability_str: &str) -> Result<Capability, SecurityError> {
        let parts: Vec<&str> = capability_str.split(':').collect();
        
        if parts.is_empty() {
            return Err(SecurityError::InvalidConfiguration {
                message: format!("Invalid capability format: {}", capability_str),
            });
        }
        
        let name = parts[0].to_string();
        let scope = self.parse_capability_scope(&name)?;
        let permissions = self.parse_capability_permissions(&name)?;
        
        // Parse constraints if present
        let constraints = if parts.len() > 1 {
            Some(self.parse_capability_constraints(&parts[1..])?)
        } else {
            None
        };
        
        Ok(Capability {
            name,
            scope,
            permissions,
            constraints,
        })
    }

    fn parse_capability_scope(&self, name: &str) -> Result<CapabilityScope, SecurityError> {
        match name.split('.').next() {
            Some("network") => Ok(CapabilityScope::Network),
            Some("fs") => Ok(CapabilityScope::FileSystem),
            Some("sys") => Ok(CapabilityScope::System),
            Some("proc") => Ok(CapabilityScope::Process),
            Some("mem") => Ok(CapabilityScope::Memory),
            Some("all") => Ok(CapabilityScope::All),
            _ => Err(SecurityError::InvalidConfiguration {
                message: format!("Unknown capability scope: {}", name),
            }),
        }
    }

    fn parse_capability_permissions(&self, name: &str) -> Result<Vec<Permission>, SecurityError> {
        let parts: Vec<&str> = name.split('.').collect();
        
        if parts.len() < 2 {
            return Ok(vec![Permission::Access]); // Default permission
        }
        
        match parts[1] {
            "read" => Ok(vec![Permission::Read]),
            "write" => Ok(vec![Permission::Write]),
            "execute" => Ok(vec![Permission::Execute]),
            "create" => Ok(vec![Permission::Create]),
            "delete" => Ok(vec![Permission::Delete]),
            "modify" => Ok(vec![Permission::Modify]),
            "all" => Ok(vec![
                Permission::Read,
                Permission::Write,
                Permission::Execute,
                Permission::Create,
                Permission::Delete,
                Permission::Modify,
            ]),
            _ => Ok(vec![Permission::Access]),
        }
    }

    fn parse_capability_constraints(&self, parts: &[&str]) -> Result<CapabilityConstraints, SecurityError> {
        let mut constraints = CapabilityConstraints {
            max_file_size: None,
            allowed_paths: None,
            allowed_ports: None,
            allowed_protocols: None,
            rate_limit: None,
        };
        
        for part in parts {
            if part.starts_with('/') {
                // File path constraint
                let paths = constraints.allowed_paths.get_or_insert(Vec::new());
                paths.push(part.to_string());
            } else if part.parse::<u16>().is_ok() {
                // Port constraint
                let ports = constraints.allowed_ports.get_or_insert(Vec::new());
                ports.push(part.parse().map_err(|_| SecurityError::InvalidPortRange)?);
            }
            // Add more constraint parsing as needed
        }
        
        Ok(constraints)
    }

    async fn create_sandbox_config(
        &self,
        service: &ServiceConfig,
        security_config: &Option<SecurityConfig>,
        policy: &SecurityPolicy,
    ) -> Result<SandboxConfig, SecurityError> {
        let sandbox_enabled = security_config
            .as_ref()
            .map(|sc| sc.sandbox_enabled)
            .unwrap_or(policy.sandbox_enabled);
        
        let sandbox_type = match service.runtime.as_str() {
            "wasm" => SandboxType::Wasm,
            "container" => SandboxType::Container,
            "native" => SandboxType::Namespace,
            _ => SandboxType::Chroot,
        };
        
        // Parse memory limits from service resources
        let max_memory = if let Some(resources) = &service.resources {
            resources.memory_limit.as_ref().map(|limit| {
                self.parse_memory_size(limit).unwrap_or(128 * 1024 * 1024) // Default to 128MB if parsing fails
            })
        } else {
            Some(128 * 1024 * 1024) // Default 128MB
        };
        
        // Parse CPU time limits from service resources
        let max_cpu_time = if let Some(resources) = &service.resources {
            resources.cpu_limit.as_ref().map(|limit| {
                self.parse_cpu_time_limit(limit).unwrap_or(std::time::Duration::from_secs(300)) // Default to 5 minutes
            })
        } else {
            Some(std::time::Duration::from_secs(300)) // Default 5 minutes
        };
        
        Ok(SandboxConfig {
            enabled: sandbox_enabled,
            sandbox_type,
            isolation_level: policy.isolation_level.clone(),
            resource_limits: SandboxResourceLimits {
                max_memory,
                max_cpu_time,
                max_file_descriptors: Some(1024),
                max_processes: Some(10),
            },
            mount_points: self.create_mount_points(service).await?,
            environment_variables: service.environment.clone(),
        })
    }

    async fn create_mount_points(&self, service: &ServiceConfig) -> Result<Vec<MountPoint>, SecurityError> {
        let mut mount_points = Vec::new();
        
        for volume in &service.volumes {
            let mount_point = MountPoint {
                source: volume.host_path.as_ref()
                    .map(|p| PathBuf::from(p))
                    .unwrap_or_else(|| PathBuf::from("/tmp")),
                target: PathBuf::from(&volume.mount_path),
                read_only: volume.read_only,
                mount_type: MountType::Bind,
            };
            mount_points.push(mount_point);
        }
        
        Ok(mount_points)
    }

    async fn create_user_context(
        &self,
        service: &ServiceConfig,
        security_config: &Option<SecurityConfig>,
    ) -> Result<UserContext, SecurityError> {
        let (user_id, group_id) = if let Some(config) = security_config {
            (config.user_id, config.group_id)
        } else {
            (None, None)
        };
        
        Ok(UserContext {
            user_id,
            group_id,
            supplementary_groups: Vec::new(),
            home_directory: None,
            working_directory: Some(PathBuf::from("/app")),
        })
    }

    async fn create_network_policy(
        &self,
        service: &ServiceConfig,
        capabilities: &[Capability],
    ) -> Result<NetworkPolicy, SecurityError> {
        let mut allowed_ports = Vec::new();
        let mut allowed_protocols = Vec::new();
        
        // Extract ports from service configuration
        for port in &service.ports {
            allowed_ports.push(port.container_port);
            allowed_protocols.push(port.protocol.clone());
        }
        
        // Extract network capabilities
        for capability in capabilities {
            if matches!(capability.scope, CapabilityScope::Network) {
                if let Some(constraints) = &capability.constraints {
                    if let Some(ports) = &constraints.allowed_ports {
                        allowed_ports.extend(ports);
                    }
                    if let Some(protocols) = &constraints.allowed_protocols {
                        allowed_protocols.extend(protocols.clone());
                    }
                }
            }
        }
        
        Ok(NetworkPolicy {
            enabled: true,
            allowed_ports,
            allowed_protocols,
            allowed_addresses: Vec::new(),
            blocked_addresses: Vec::new(),
            firewall_rules: Vec::new(),
        })
    }

    async fn create_file_system_policy(
        &self,
        service: &ServiceConfig,
        capabilities: &[Capability],
    ) -> Result<FileSystemPolicy, SecurityError> {
        let mut allowed_paths = Vec::new();
        let mut read_only_paths = Vec::new();
        
        // Extract paths from service volumes
        for volume in &service.volumes {
            let path = PathBuf::from(&volume.mount_path);
            if volume.read_only {
                read_only_paths.push(path);
            } else {
                allowed_paths.push(path);
            }
        }
        
        // Extract file system capabilities
        for capability in capabilities {
            if matches!(capability.scope, CapabilityScope::FileSystem) {
                if let Some(constraints) = &capability.constraints {
                    if let Some(paths) = &constraints.allowed_paths {
                        for path in paths {
                            allowed_paths.push(PathBuf::from(path));
                        }
                    }
                }
            }
        }
        
        Ok(FileSystemPolicy {
            enabled: true,
            allowed_paths,
            read_only_paths,
            blocked_paths: Vec::new(),
            max_file_size: None,
            max_total_size: None,
        })
    }

    fn validate_capability_constraints(
        &self,
        capability: &Capability,
        constraints: &CapabilityConstraints,
    ) -> Result<(), SecurityError> {
        // Validate file size constraints
        if let Some(max_file_size) = constraints.max_file_size {
            if max_file_size > 1024 * 1024 * 1024 { // 1GB limit
                return Err(SecurityError::PolicyViolation {
                    policy: "Max file size cannot exceed 1GB".to_string(),
                });
            }
        }
        
        // Validate port constraints
        if let Some(allowed_ports) = &constraints.allowed_ports {
            for port in allowed_ports {
                if *port < 1024 && !matches!(capability.scope, CapabilityScope::All) {
                    return Err(SecurityError::PolicyViolation {
                        policy: format!("Privileged port {} requires elevated capabilities", port),
                    });
                }
            }
        }
        
        // Validate path constraints
        if let Some(allowed_paths) = &constraints.allowed_paths {
            for path in allowed_paths {
                if path.starts_with("/sys") || path.starts_with("/proc") {
                    return Err(SecurityError::PolicyViolation {
                        policy: format!("Access to system path {} is restricted", path),
                    });
                }
            }
        }
        
        // Validate rate limiting
        if let Some(rate_limit) = &constraints.rate_limit {
            if rate_limit.requests_per_second > 1000 {
                return Err(SecurityError::PolicyViolation {
                    policy: "Rate limit cannot exceed 1000 requests per second".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    fn parse_memory_size(&self, size_str: &str) -> Result<u64, SecurityError> {
        let size_str = size_str.to_uppercase();
        
        let bytes = if size_str.ends_with("GB") {
            let gb: f64 = size_str[..size_str.len()-2].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (gb * 1024.0 * 1024.0 * 1024.0) as u64
        } else if size_str.ends_with("MB") {
            let mb: f64 = size_str[..size_str.len()-2].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (mb * 1024.0 * 1024.0) as u64
        } else if size_str.ends_with("KB") {
            let kb: f64 = size_str[..size_str.len()-2].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (kb * 1024.0) as u64
        } else if size_str.ends_with("G") {
            let gb: f64 = size_str[..size_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (gb * 1024.0 * 1024.0 * 1024.0) as u64
        } else if size_str.ends_with("M") {
            let mb: f64 = size_str[..size_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (mb * 1024.0 * 1024.0) as u64
        } else if size_str.ends_with("K") {
            let kb: f64 = size_str[..size_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?;
            (kb * 1024.0) as u64
        } else {
            size_str.parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid memory size format: {}", size_str),
                })?
        };
        
        Ok(bytes)
    }
    
    fn parse_cpu_time_limit(&self, limit_str: &str) -> Result<std::time::Duration, SecurityError> {
        let limit_str = limit_str.to_uppercase();
        
        let duration = if limit_str.ends_with("H") {
            let hours: f64 = limit_str[..limit_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid CPU time format: {}", limit_str),
                })?;
            std::time::Duration::from_secs_f64(hours * 3600.0)
        } else if limit_str.ends_with("M") {
            let minutes: f64 = limit_str[..limit_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid CPU time format: {}", limit_str),
                })?;
            std::time::Duration::from_secs_f64(minutes * 60.0)
        } else if limit_str.ends_with("S") {
            let seconds: f64 = limit_str[..limit_str.len()-1].parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid CPU time format: {}", limit_str),
                })?;
            std::time::Duration::from_secs_f64(seconds)
        } else {
            // Default to seconds if no unit specified
            let seconds: f64 = limit_str.parse()
                .map_err(|_| SecurityError::InvalidConfiguration {
                    message: format!("Invalid CPU time format: {}", limit_str),
                })?;
            std::time::Duration::from_secs_f64(seconds)
        };
        
        Ok(duration)
    }

    async fn enforce_file_access_policy(
        &self,
        context: &SecurityContext,
        path: &PathBuf,
        access_type: &FileAccessType,
    ) -> Result<(), SecurityError> {
        if !context.file_system_policy.enabled {
            return Ok(());
        }
        
        // Check if path is allowed
        let path_allowed = context.file_system_policy.allowed_paths.iter()
            .any(|allowed_path| path.starts_with(allowed_path));
        
        if !path_allowed {
            return Err(SecurityError::PolicyViolation {
                policy: format!("File access denied: {}", path.display()),
            });
        }
        
        // Check read-only constraints
        if matches!(access_type, FileAccessType::Write | FileAccessType::Delete) {
            let is_read_only = context.file_system_policy.read_only_paths.iter()
                .any(|ro_path| path.starts_with(ro_path));
            
            if is_read_only {
                return Err(SecurityError::PolicyViolation {
                    policy: format!("Write access denied to read-only path: {}", path.display()),
                });
            }
        }
        
        Ok(())
    }

    async fn enforce_network_access_policy(
        &self,
        context: &SecurityContext,
        address: &str,
        port: u16,
        protocol: &str,
    ) -> Result<(), SecurityError> {
        if !context.network_policy.enabled {
            return Ok(());
        }
        
        // Check if port is allowed
        if !context.network_policy.allowed_ports.contains(&port) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Network access denied: port {} not allowed", port),
            });
        }
        
        // Check if protocol is allowed
        if !context.network_policy.allowed_protocols.contains(&protocol.to_string()) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Network access denied: protocol {} not allowed", protocol),
            });
        }
        
        // Check blocked addresses
        if context.network_policy.blocked_addresses.contains(&address.to_string()) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Network access denied: address {} is blocked", address),
            });
        }
        
        Ok(())
    }

    async fn enforce_process_spawn_policy(
        &self,
        context: &SecurityContext,
        command: &str,
        args: &[String],
    ) -> Result<(), SecurityError> {
        // Check if process spawning is allowed
        let has_spawn_capability = context.capabilities.iter().any(|cap| {
            cap.name == "sys.spawn" || cap.name == "sys.exec" || cap.name == "process.create"
        });
        
        if !has_spawn_capability {
            return Err(SecurityError::PolicyViolation {
                policy: "Process spawning not allowed - missing spawn capability".to_string(),
            });
        }
        
        // Check command allowlist
        let allowed_commands = vec![
            "/bin/sh", "/bin/bash", "/usr/bin/python", "/usr/bin/python3",
            "/usr/bin/node", "/usr/bin/cargo", "/usr/bin/rustc"
        ];
        
        if !allowed_commands.contains(&command) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Command '{}' not in allowed commands list", command),
            });
        }
        
        // Check for dangerous arguments
        let dangerous_args = vec![
            "--unsafe", "--allow-all", "--no-sandbox", "--disable-security",
            "rm", "del", "format", "mkfs", "dd", "chmod", "chown"
        ];
        
        for arg in args {
            for dangerous_arg in &dangerous_args {
                if arg.contains(dangerous_arg) {
                    return Err(SecurityError::PolicyViolation {
                        policy: format!("Dangerous argument '{}' detected in command", arg),
                    });
                }
            }
        }
        
        // Audit process spawn event
        self.audit_event(SecurityAuditEvent {
            timestamp: chrono::Utc::now(),
            service_name: context.service_name.clone(),
            event_type: AuditEventType::SuspiciousActivity,
            severity: AuditSeverity::Medium,
            message: format!("Process spawn: {} with args: {:?}", command, args),
            metadata: HashMap::new(),
        }).await;
        
        Ok(())
    }

    async fn enforce_resource_access_policy(
        &self,
        context: &SecurityContext,
        resource_type: &str,
        operation: &str,
    ) -> Result<(), SecurityError> {
        // Check if resource access is allowed based on capabilities
        let required_capability = match resource_type {
            "memory" => "sys.memory",
            "cpu" => "sys.cpu",
            "disk" => "fs.read",
            "network" => "network.client",
            "time" => "sys.time",
            "env" => "sys.env",
            "random" => "sys.random",
            _ => return Err(SecurityError::PolicyViolation {
                policy: format!("Unknown resource type: {}", resource_type),
            }),
        };
        
        // Check if service has required capability
        let has_capability = context.capabilities.iter().any(|cap| {
            cap.name == required_capability || cap.name == "sys.all"
        });
        
        if !has_capability {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Resource access denied: missing '{}' capability for {} operation", required_capability, resource_type),
            });
        }
        
        // Check operation permissions
        let allowed_operations = match resource_type {
            "memory" => vec!["read", "allocate", "deallocate"],
            "cpu" => vec!["read", "set_priority"],
            "disk" => vec!["read", "write", "stat"],
            "network" => vec!["connect", "bind", "listen"],
            "time" => vec!["read", "set"],
            "env" => vec!["read", "set"],
            "random" => vec!["read"],
            _ => vec![],
        };
        
        if !allowed_operations.contains(&operation) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Operation '{}' not allowed for resource type '{}'", operation, resource_type),
            });
        }
        
        // Check for dangerous operations
        let dangerous_operations = vec!["format", "delete", "destroy", "kill", "terminate"];
        if dangerous_operations.contains(&operation) {
            return Err(SecurityError::PolicyViolation {
                policy: format!("Dangerous operation '{}' blocked", operation),
            });
        }
        
        // Audit resource access event
        self.audit_event(SecurityAuditEvent {
            timestamp: chrono::Utc::now(),
            service_name: context.service_name.clone(),
            event_type: AuditEventType::ResourceLimitExceeded,
            severity: AuditSeverity::Low,
            message: format!("Resource access: {} operation on {}", operation, resource_type),
            metadata: HashMap::new(),
        }).await;
        
        Ok(())
    }

    async fn audit_event(&self, event: SecurityAuditEvent) {
        // Log the audit event
        match event.severity {
            AuditSeverity::Critical => error!("SECURITY AUDIT [CRITICAL]: {} - {}", event.service_name, event.message),
            AuditSeverity::High => warn!("SECURITY AUDIT [HIGH]: {} - {}", event.service_name, event.message),
            AuditSeverity::Medium => info!("SECURITY AUDIT [MEDIUM]: {} - {}", event.service_name, event.message),
            AuditSeverity::Low => debug!("SECURITY AUDIT [LOW]: {} - {}", event.service_name, event.message),
        }
        
        // Store in audit log for later retrieval
        // Note: In a production system, this would be written to a secure, tamper-evident log
        // such as a cryptographically secured database or write-only log files
        
        // For now, we'll log to file system with structured JSON
        self.write_audit_to_file(&event).await;
        
        // Also store in memory for immediate access (with rotation)
        // This should be replaced with a proper audit log storage system
        debug!("Audit event stored: {:?}", event);
    }
    
    async fn write_audit_to_file(&self, event: &SecurityAuditEvent) {
        use std::fs::OpenOptions;
        use std::io::Write;
        
        let audit_log_path = "/tmp/toadstool_security_audit.log";
        
        // Create audit log entry
        let log_entry = format!(
            "{} [{}] {} - {} - {} - {:?}\n",
            event.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            match event.severity {
                AuditSeverity::Critical => "CRITICAL",
                AuditSeverity::High => "HIGH",
                AuditSeverity::Medium => "MEDIUM",
                AuditSeverity::Low => "LOW",
            },
            event.service_name,
            match event.event_type {
                AuditEventType::CapabilityViolation => "CAPABILITY_VIOLATION",
                AuditEventType::PolicyViolation => "POLICY_VIOLATION",
                AuditEventType::AuthenticationFailure => "AUTH_FAILURE",
                AuditEventType::AuthorizationFailure => "AUTHZ_FAILURE",
                AuditEventType::SandboxBreach => "SANDBOX_BREACH",
                AuditEventType::ResourceLimitExceeded => "RESOURCE_LIMIT",
                AuditEventType::SuspiciousActivity => "SUSPICIOUS",
            },
            event.message,
            event.metadata
        );
        
        // Write to audit log file
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(audit_log_path)
        {
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                error!("Failed to write audit log: {}", e);
            }
        } else {
            error!("Failed to open audit log file: {}", audit_log_path);
        }
    }
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub fn register_default_capabilities(&mut self) {
        // File system capabilities
        self.register_capability(CapabilityDefinition {
            name: "fs.read".to_string(),
            description: "Read access to file system".to_string(),
            scope: CapabilityScope::FileSystem,
            required_permissions: vec![Permission::Read],
            security_level: SecurityLevel::Safe,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "fs.write".to_string(),
            description: "Write access to file system".to_string(),
            scope: CapabilityScope::FileSystem,
            required_permissions: vec![Permission::Write],
            security_level: SecurityLevel::Moderate,
        });
        
        // Network capabilities
        self.register_capability(CapabilityDefinition {
            name: "network.client".to_string(),
            description: "Network client access".to_string(),
            scope: CapabilityScope::Network,
            required_permissions: vec![Permission::Access],
            security_level: SecurityLevel::Moderate,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "network.server".to_string(),
            description: "Network server access".to_string(),
            scope: CapabilityScope::Network,
            required_permissions: vec![Permission::Access],
            security_level: SecurityLevel::Dangerous,
        });
        
        // System capabilities
        self.register_capability(CapabilityDefinition {
            name: "sys.time".to_string(),
            description: "System time access".to_string(),
            scope: CapabilityScope::System,
            required_permissions: vec![Permission::Read],
            security_level: SecurityLevel::Safe,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "sys.spawn".to_string(),
            description: "Process spawning capability".to_string(),
            scope: CapabilityScope::Process,
            required_permissions: vec![Permission::Create, Permission::Execute],
            security_level: SecurityLevel::Dangerous,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "sys.memory".to_string(),
            description: "Memory management access".to_string(),
            scope: CapabilityScope::Memory,
            required_permissions: vec![Permission::Read, Permission::Write],
            security_level: SecurityLevel::Moderate,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "sys.cpu".to_string(),
            description: "CPU management access".to_string(),
            scope: CapabilityScope::System,
            required_permissions: vec![Permission::Read, Permission::Modify],
            security_level: SecurityLevel::Moderate,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "sys.env".to_string(),
            description: "Environment variable access".to_string(),
            scope: CapabilityScope::System,
            required_permissions: vec![Permission::Read, Permission::Write],
            security_level: SecurityLevel::Safe,
        });
        
        self.register_capability(CapabilityDefinition {
            name: "sys.random".to_string(),
            description: "Random number generation access".to_string(),
            scope: CapabilityScope::System,
            required_permissions: vec![Permission::Read],
            security_level: SecurityLevel::Safe,
        });
    }

    pub fn register_capability(&mut self, capability: CapabilityDefinition) {
        self.capabilities.insert(capability.name.clone(), capability);
    }

    pub fn is_registered(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    pub fn get_capability(&self, name: &str) -> Option<&CapabilityDefinition> {
        self.capabilities.get(name)
    }
}

/// Security operation types for policy enforcement
#[derive(Debug)]
pub enum SecurityOperation {
    FileAccess {
        path: PathBuf,
        access_type: FileAccessType,
    },
    NetworkAccess {
        address: String,
        port: u16,
        protocol: String,
    },
    ProcessSpawn {
        command: String,
        args: Vec<String>,
    },
    ResourceAccess {
        resource_type: String,
        operation: String,
    },
}

#[derive(Debug)]
pub enum FileAccessType {
    Read,
    Write,
    Execute,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStats {
    pub total_contexts: usize,
    pub total_capabilities: usize,
    pub audit_events: usize,
    pub policy_violations: usize,
    pub capability_violations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_manager_creation() {
        let manager = SecurityManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_capability_parsing() {
        let manager = SecurityManager::new().await.unwrap();
        
        let capability = manager.parse_capability("fs.read:/tmp").unwrap();
        assert_eq!(capability.name, "fs.read");
        assert!(matches!(capability.scope, CapabilityScope::FileSystem));
        assert!(capability.permissions.contains(&Permission::Read));
    }

    #[tokio::test]
    async fn test_capability_validation() {
        let manager = SecurityManager::new().await.unwrap();
        
        // Test valid capability
        let result = manager.validate_capability("test-service", "fs.read:/tmp").await;
        assert!(result.is_ok());
        
        // Test invalid capability
        let result = manager.validate_capability("test-service", "invalid.capability").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_capability_registry() {
        let mut registry = CapabilityRegistry::new();
        registry.register_default_capabilities();
        
        assert!(registry.is_registered("fs.read"));
        assert!(registry.is_registered("network.client"));
        assert!(!registry.is_registered("unknown.capability"));
    }
} 