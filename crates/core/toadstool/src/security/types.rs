// SPDX-License-Identifier: AGPL-3.0-only
//! Security types and data structures

use serde::{Deserialize, Serialize};

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
