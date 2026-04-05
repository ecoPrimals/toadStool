// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security and access control configuration
//!
//! This module contains configuration types for security including:
//! - Authentication (JWT, sessions)
//! - Authorization (permissions, roles)
//! - Encryption (at-rest, in-transit)
//! - Audit logging
//! - Sandboxing (seccomp, namespaces)

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::app;

/// Security configuration
///
/// Top-level security settings encompassing authentication, authorization,
/// encryption, auditing, and sandboxing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Authentication settings
    pub auth: AuthConfig,

    /// Authorization settings
    pub authz: AuthzConfig,

    /// Encryption settings
    pub encryption: EncryptionConfig,

    /// Audit logging settings
    pub audit: AuditConfig,

    /// Sandbox isolation settings
    pub sandbox: SandboxConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,

    /// Authentication provider (local, oauth, ldap)
    pub provider: String,

    /// JWT secret key for token signing
    pub jwt_secret: Option<String>,

    /// Session timeout duration
    pub session_timeout: Duration,

    /// Maximum login attempts before lockout
    pub max_login_attempts: u32,

    /// Lockout duration after max attempts
    pub lockout_duration: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            jwt_secret: None,
            session_timeout: Duration::from_secs(app::DEFAULT_SESSION_TIMEOUT_SECS),
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzConfig {
    /// Enable authorization
    pub enabled: bool,

    /// Authorization provider (local, rbac, abac)
    pub provider: String,

    /// Default permissions for authenticated users
    pub default_permissions: Vec<String>,

    /// Admin-level permissions
    pub admin_permissions: Vec<String>,
}

impl Default for AuthzConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            default_permissions: vec!["read".to_string()],
            admin_permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,

    /// Encryption algorithm (aes-256-gcm, chacha20-poly1305)
    pub algorithm: String,

    /// Key derivation function (pbkdf2, argon2)
    pub key_derivation: String,

    /// Key length in bytes
    pub key_length: usize,

    /// Encrypt data at rest
    pub encrypt_at_rest: bool,

    /// Encrypt data in transit (TLS)
    pub encrypt_in_transit: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "aes-256-gcm".to_string(),
            key_derivation: "pbkdf2".to_string(),
            key_length: app::DEFAULT_ENCRYPTION_KEY_LENGTH,
            encrypt_at_rest: false,
            encrypt_in_transit: true,
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Audit log file path
    pub log_file: String,

    /// Audit log level (debug, info, warn, error)
    pub log_level: String,

    /// Audit log format (json, text)
    pub log_format: String,

    /// Enable log rotation
    pub log_rotation: bool,

    /// Maximum log file size in bytes
    pub max_log_size: u64,

    /// Maximum number of rotated log files
    pub max_log_files: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: "audit.log".to_string(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            log_rotation: true,
            max_log_size: app::DEFAULT_MAX_LOG_SIZE,
            max_log_files: app::DEFAULT_MAX_LOG_FILES,
        }
    }
}

/// Sandbox configuration
///
/// Controls process isolation and security restrictions using
/// Linux namespaces, seccomp, and similar technologies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing
    pub enabled: bool,

    /// Sandbox type (seccomp, namespace, apparmor, selinux)
    pub sandbox_type: String,

    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,

    /// Blocked system calls
    pub blocked_syscalls: Vec<String>,

    /// Allow network access from sandbox
    pub allow_network: bool,

    /// Allow file system access from sandbox
    pub allow_file_access: bool,

    /// Allowed directories for file access
    pub allowed_dirs: Vec<String>,

    /// Blocked directories (deny access)
    pub blocked_dirs: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sandbox_type: "seccomp".to_string(),
            allowed_syscalls: vec![
                "read".to_string(),
                "write".to_string(),
                "open".to_string(),
                "close".to_string(),
            ],
            blocked_syscalls: vec![
                "execve".to_string(),
                "fork".to_string(),
                "clone".to_string(),
            ],
            allow_network: false,
            allow_file_access: true,
            allowed_dirs: vec!["/tmp".to_string()],
            blocked_dirs: vec!["/etc".to_string(), "/proc".to_string(), "/sys".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::default();
        assert!(!config.auth.enabled); // Disabled by default
        assert!(config.sandbox.enabled); // Enabled by default
    }

    #[test]
    fn test_sandbox_config_defaults() {
        let config = SandboxConfig::default();
        assert!(config.enabled);
        assert!(!config.allowed_syscalls.is_empty());
        assert!(!config.blocked_syscalls.is_empty());
    }

    #[test]
    fn test_encryption_config_defaults() {
        let config = EncryptionConfig::default();
        assert_eq!(config.algorithm, "aes-256-gcm");
        assert!(config.encrypt_in_transit);
    }
}
