//! Security and isolation management
//!
//! This module defines security contexts, capabilities, isolation levels,
//! and security policies for workload execution.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Security context for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Isolation level for the workload
    pub isolation_level: IsolationLevel,
    /// Capabilities granted to the workload
    pub capabilities: HashSet<Capability>,
    /// Security policies to enforce
    pub policies: Vec<SecurityPolicy>,
    /// User/group context
    pub user_context: Option<UserContext>,
    /// Network security settings
    pub network_security: NetworkSecurity,
    /// File system security settings
    pub filesystem_security: FilesystemSecurity,
    /// Additional security parameters
    pub custom_security: HashMap<String, serde_json::Value>,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            capabilities: HashSet::new(),
            policies: Vec::new(),
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
            custom_security: HashMap::new(),
        }
    }
}

/// Security isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// No isolation - full system access (dangerous)
    None,
    /// Basic isolation - process-level separation
    Basic,
    /// Standard isolation - container-like separation
    Standard,
    /// Enhanced isolation - additional security measures
    Enhanced,
    /// Maximum isolation - strictest security (may limit functionality)
    Maximum,
}

impl IsolationLevel {
    /// Get the capabilities allowed by this isolation level
    pub fn default_capabilities(&self) -> HashSet<Capability> {
        match self {
            Self::None => {
                // All capabilities (not recommended)
                Capability::all_capabilities()
            }
            Self::Basic => {
                // Basic execution capabilities
                vec![Capability::Execute, Capability::Read, Capability::WriteTemp]
                    .into_iter()
                    .collect()
            }
            Self::Standard => {
                // Standard container-like capabilities
                vec![
                    Capability::Execute,
                    Capability::Read,
                    Capability::WriteTemp,
                    Capability::NetworkClient,
                    Capability::ProcessSpawn,
                ]
                .into_iter()
                .collect()
            }
            Self::Enhanced => {
                // Limited capabilities with strict enforcement
                vec![Capability::Execute, Capability::Read, Capability::WriteTemp]
                    .into_iter()
                    .collect()
            }
            Self::Maximum => {
                // Minimal capabilities - execute only
                vec![Capability::Execute].into_iter().collect()
            }
        }
    }

    /// Check if this isolation level allows the given capability
    pub fn allows_capability(&self, capability: &Capability) -> bool {
        self.default_capabilities().contains(capability)
    }
}

/// Security capabilities that can be granted to workloads
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Execute code/programs
    Execute,
    /// Read from filesystem
    Read,
    /// Write to filesystem (general)
    Write,
    /// Write to temporary directories only
    WriteTemp,
    /// Create network connections as client
    NetworkClient,
    /// Listen for network connections as server
    NetworkServer,
    /// Access specific network resources
    NetworkAccess { hosts: Vec<String> },
    /// Spawn child processes
    ProcessSpawn,
    /// Access system information
    SystemInfo,
    /// Access environment variables
    EnvironmentAccess,
    /// Inter-process communication
    IpcAccess,
    /// Hardware device access
    DeviceAccess { devices: Vec<String> },
    /// Privilege escalation
    PrivilegeEscalation,
    /// Raw socket access
    RawSocket,
    /// System administration
    SystemAdmin,
    /// Kernel module loading
    KernelModule,
    /// Custom capability
    Custom { name: String, description: String },
}

impl Capability {
    /// Get all possible capabilities (used for no isolation)
    pub fn all_capabilities() -> HashSet<Self> {
        vec![
            Self::Execute,
            Self::Read,
            Self::Write,
            Self::NetworkClient,
            Self::NetworkServer,
            Self::ProcessSpawn,
            Self::SystemInfo,
            Self::EnvironmentAccess,
            Self::IpcAccess,
            Self::PrivilegeEscalation,
            Self::RawSocket,
            Self::SystemAdmin,
            Self::KernelModule,
        ]
        .into_iter()
        .collect()
    }

    /// Check if this is a dangerous capability
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            Self::PrivilegeEscalation
                | Self::RawSocket
                | Self::SystemAdmin
                | Self::KernelModule
                | Self::NetworkServer
        )
    }

    /// Get a human-readable description of this capability
    pub fn description(&self) -> &str {
        match self {
            Self::Execute => "Execute code and programs",
            Self::Read => "Read from filesystem",
            Self::Write => "Write to filesystem",
            Self::WriteTemp => "Write to temporary directories only",
            Self::NetworkClient => "Make outbound network connections",
            Self::NetworkServer => "Accept inbound network connections",
            Self::NetworkAccess { .. } => "Access specific network resources",
            Self::ProcessSpawn => "Spawn child processes",
            Self::SystemInfo => "Read system information",
            Self::EnvironmentAccess => "Access environment variables",
            Self::IpcAccess => "Inter-process communication",
            Self::DeviceAccess { .. } => "Access hardware devices",
            Self::PrivilegeEscalation => "Escalate privileges",
            Self::RawSocket => "Create raw network sockets",
            Self::SystemAdmin => "System administration functions",
            Self::KernelModule => "Load kernel modules",
            Self::Custom { description, .. } => description,
        }
    }
}

/// Security policy definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityPolicy {
    /// Deny specific system calls
    DenySystemCalls { syscalls: Vec<String> },
    /// Allow only specific system calls
    AllowSystemCalls { syscalls: Vec<String> },
    /// Restrict file access to specific paths
    RestrictFilesystem { allowed_paths: Vec<PathBuf> },
    /// Limit network access to specific hosts/ports
    RestrictNetwork {
        allowed_hosts: Vec<String>,
        allowed_ports: Vec<u16>,
    },
    /// Set resource limits
    ResourceLimits {
        cpu_percent: Option<u8>,
        memory_mb: Option<u64>,
    },
    /// Custom security policy
    Custom {
        name: String,
        config: HashMap<String, serde_json::Value>,
    },
}

/// User context for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    /// User ID to run as
    pub uid: Option<u32>,
    /// Group ID to run as
    pub gid: Option<u32>,
    /// Username to run as
    pub username: Option<String>,
    /// Group name to run as
    pub groupname: Option<String>,
    /// Additional groups
    pub supplementary_groups: Vec<u32>,
    /// Drop all capabilities
    pub drop_capabilities: bool,
}

impl Default for UserContext {
    fn default() -> Self {
        Self {
            uid: None,
            gid: None,
            username: None,
            groupname: None,
            supplementary_groups: Vec::new(),
            drop_capabilities: true,
        }
    }
}

/// Network security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurity {
    /// Allow internet access
    pub internet_access: bool,
    /// Allow internal network access
    pub internal_access: bool,
    /// Allowed destination hosts
    pub allowed_hosts: Vec<String>,
    /// Allowed destination ports
    pub allowed_ports: Vec<u16>,
    /// Denied hosts (takes precedence over allowed)
    pub denied_hosts: Vec<String>,
    /// DNS servers to use
    pub dns_servers: Vec<String>,
}

impl Default for NetworkSecurity {
    fn default() -> Self {
        Self {
            internet_access: false,
            internal_access: true,
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
            denied_hosts: Vec::new(),
            dns_servers: Vec::new(),
        }
    }
}

/// Filesystem security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSecurity {
    /// Read-only filesystem
    pub read_only: bool,
    /// Allowed read paths
    pub read_paths: Vec<PathBuf>,
    /// Allowed write paths
    pub write_paths: Vec<PathBuf>,
    /// Temporary directory access
    pub temp_access: bool,
    /// Hidden files/directories
    pub hidden_paths: Vec<PathBuf>,
    /// Maximum file size for writes
    pub max_file_size: Option<u64>,
}

impl Default for FilesystemSecurity {
    fn default() -> Self {
        Self {
            read_only: false,
            read_paths: Vec::new(),
            write_paths: Vec::new(),
            temp_access: true,
            hidden_paths: Vec::new(),
            max_file_size: Some(100 * 1024 * 1024), // 100 MB
        }
    }
}

impl SecurityContext {
    /// Create a security context for the given isolation level
    pub fn for_isolation_level(level: IsolationLevel) -> Self {
        Self {
            isolation_level: level,
            capabilities: level.default_capabilities(),
            ..Default::default()
        }
    }

    /// Add a capability to this security context
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.insert(capability);
        self
    }

    /// Add multiple capabilities to this security context
    pub fn with_capabilities(mut self, capabilities: impl IntoIterator<Item = Capability>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Add a security policy to this context
    pub fn with_policy(mut self, policy: SecurityPolicy) -> Self {
        self.policies.push(policy);
        self
    }

    /// Check if this context has the given capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Validate the security context for consistency
    pub fn validate(&self) -> crate::error::ToadStoolResult<()> {
        // Check for dangerous capability combinations
        if self.isolation_level == IsolationLevel::Maximum {
            let dangerous_caps: Vec<_> = self
                .capabilities
                .iter()
                .filter(|cap| cap.is_dangerous())
                .collect();

            if !dangerous_caps.is_empty() {
                return Err(crate::error::ToadStoolError::security(format!(
                    "Maximum isolation level cannot have dangerous capabilities: {:?}",
                    dangerous_caps
                )));
            }
        }

        // Validate network security
        if !self.network_security.internet_access
            && !self.network_security.internal_access
            && (self.has_capability(&Capability::NetworkClient)
                || self.has_capability(&Capability::NetworkServer))
        {
            return Err(crate::error::ToadStoolError::security(
                "Network capabilities granted but no network access allowed",
            ));
        }

        // Validate filesystem security
        if self.filesystem_security.read_only && !self.filesystem_security.write_paths.is_empty() {
            return Err(crate::error::ToadStoolError::security(
                "Filesystem is read-only but write paths specified",
            ));
        }

        Ok(())
    }
}
