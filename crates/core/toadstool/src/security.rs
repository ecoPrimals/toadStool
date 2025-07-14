//! Security context and policies for ToadStool workloads

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{ToadStoolError, ToadStoolResult};

/// Security context for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Capabilities granted to the workload
    pub capabilities: Vec<Capability>,
    /// User context
    pub user_context: Option<UserContext>,
    /// Network security settings
    pub network_security: NetworkSecurity,
    /// File system security settings
    pub filesystem_security: FilesystemSecurity,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            capabilities: vec![Capability::Execute, Capability::Read],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        }
    }
}

impl SecurityContext {
    /// Create a security context for a specific isolation level
    pub fn for_isolation_level(level: IsolationLevel) -> Self {
        Self {
            isolation_level: level,
            capabilities: vec![Capability::Execute],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        }
    }

    /// Add a capability to the security context
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Check if a capability is granted
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Set user context
    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }

    /// Validate the security context
    pub fn validate(&self) -> ToadStoolResult<()> {
        // Basic validation of the security context
        if self.capabilities.is_empty() {
            return Err(ToadStoolError::validation(
                "Security context must have at least one capability",
            ));
        }

        // Check for conflicting capabilities
        if self.capabilities.contains(&Capability::Read)
            && self.capabilities.contains(&Capability::Write)
            && self.filesystem_security.read_only
        {
            return Err(ToadStoolError::validation(
                "Cannot have write capability with read-only filesystem",
            ));
        }

        Ok(())
    }
}

/// Isolation levels for workload execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IsolationLevel {
    /// No isolation
    None,
    /// Basic isolation
    Basic,
    /// Standard isolation
    Standard,
    /// Enhanced isolation
    Enhanced,
    /// Maximum isolation
    Maximum,
}

/// Capabilities that can be granted to workloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Execute programs
    Execute,
    /// Read files
    Read,
    /// Write files
    Write,
    /// Network client access
    NetworkClient,
    /// Network server access
    NetworkServer,
    /// System information access
    SystemInfo,
    /// Process management
    ProcessManagement,
    /// Custom capability
    Custom(String),
}

/// User context for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// Username
    pub username: Option<String>,
    /// User ID
    pub uid: Option<u32>,
    /// Group ID
    pub gid: Option<u32>,
    /// Additional groups
    pub groups: Vec<u32>,
}

/// Network security settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkSecurity {
    /// Allow outbound connections
    pub allow_outbound: bool,
    /// Allow inbound connections
    pub allow_inbound: bool,
    /// Allowed domains
    pub allowed_domains: Vec<String>,
    /// Blocked domains
    pub blocked_domains: Vec<String>,
    /// Allowed ports
    pub allowed_ports: Vec<u16>,
    /// Blocked ports
    pub blocked_ports: Vec<u16>,
}

/// File system security settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilesystemSecurity {
    /// Read-only file system
    pub read_only: bool,
    /// Allowed read paths
    pub allowed_read_paths: Vec<String>,
    /// Allowed write paths
    pub allowed_write_paths: Vec<String>,
    /// Blocked paths
    pub blocked_paths: Vec<String>,
}

/// Security settings for runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Default isolation level
    pub default_isolation_level: IsolationLevel,
    /// Default capabilities
    pub default_capabilities: Vec<Capability>,
    /// Security policies
    pub security_policies: HashMap<String, SecurityPolicy>,
    /// Audit settings
    pub audit_settings: AuditSettings,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            default_isolation_level: IsolationLevel::Standard,
            default_capabilities: vec![Capability::Execute],
            security_policies: HashMap::new(),
            audit_settings: AuditSettings::default(),
        }
    }
}

/// Security policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Policy version
    pub version: String,
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Allowed capabilities
    pub allowed_capabilities: Vec<Capability>,
    /// Denied capabilities
    pub denied_capabilities: Vec<Capability>,
    /// Network restrictions
    pub network_restrictions: NetworkSecurity,
    /// File system restrictions
    pub filesystem_restrictions: FilesystemSecurity,
}

/// Audit settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSettings {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log level
    pub log_level: String,
    /// Audit events to log
    pub events: Vec<AuditEvent>,
}

impl Default for AuditSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            events: vec![
                AuditEvent::ExecutionStart,
                AuditEvent::ExecutionEnd,
                AuditEvent::SecurityViolation,
            ],
        }
    }
}

/// Audit events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEvent {
    /// Execution started
    ExecutionStart,
    /// Execution ended
    ExecutionEnd,
    /// Security violation occurred
    SecurityViolation,
    /// Capability used
    CapabilityUsed,
    /// Network access
    NetworkAccess,
    /// File system access
    FilesystemAccess,
}

/// Security provider trait
#[async_trait]
pub trait SecurityProvider: Send + Sync {
    /// Create a security context
    async fn create_security_context(
        &self,
        policy: &SecurityPolicy,
    ) -> ToadStoolResult<SecurityContext>;

    /// Validate a security context
    async fn validate_security_context(&self, context: &SecurityContext) -> ToadStoolResult<()>;

    /// Apply security context to a workload
    async fn apply_security_context(
        &self,
        context: &SecurityContext,
        workload_id: &str,
    ) -> ToadStoolResult<()>;

    /// Remove security context from a workload
    async fn remove_security_context(&self, workload_id: &str) -> ToadStoolResult<()>;

    /// Check if a capability is allowed
    async fn check_capability(
        &self,
        context: &SecurityContext,
        capability: &Capability,
    ) -> ToadStoolResult<bool>;

    /// Audit security event
    async fn audit_event(
        &self,
        event: AuditEvent,
        context: &SecurityContext,
    ) -> ToadStoolResult<()>;
}
