//! Canonical authentication configuration for ToadStool services
//!
//! This module provides unified authentication configuration types for service-to-service
//! authentication across the ToadStool platform.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication type for service-to-service communication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum AuthType {
    /// No authentication
    #[default]
    None,
    /// Basic authentication (username/password)
    Basic,
    /// Bearer token authentication
    Bearer,
    /// API key authentication
    ApiKey,
    /// `OAuth2` authentication
    OAuth2,
    /// Mutual TLS authentication
    MutualTLS,
    /// Custom authentication scheme
    Custom(String),
}


/// Service authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthCredentials {
    /// Username for basic auth
    pub username: Option<String>,
    /// Password for basic auth
    pub password: Option<String>,
    /// Bearer token
    pub token: Option<String>,
    /// API key
    pub api_key: Option<String>,
    /// Certificate path for mTLS
    pub cert_path: Option<String>,
    /// Private key path for mTLS
    pub key_path: Option<String>,
    /// CA certificate path for mTLS
    pub ca_path: Option<String>,
    /// Additional key-value credentials
    pub extra: HashMap<String, String>,
}

impl AuthCredentials {
    /// Create empty credentials
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create credentials with a bearer token
    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..Default::default()
        }
    }

    /// Create credentials with an API key
    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            api_key: Some(key.into()),
            ..Default::default()
        }
    }

    /// Create credentials with username and password
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: Some(username.into()),
            password: Some(password.into()),
            ..Default::default()
        }
    }

    /// Create credentials for mTLS
    pub fn mtls(
        cert_path: impl Into<String>,
        key_path: impl Into<String>,
        ca_path: Option<String>,
    ) -> Self {
        Self {
            cert_path: Some(cert_path.into()),
            key_path: Some(key_path.into()),
            ca_path,
            ..Default::default()
        }
    }

    /// Create from a simple `HashMap` (for backward compatibility)
    #[must_use]
    pub fn from_map(map: HashMap<String, String>) -> Self {
        let mut creds = Self::new();
        creds.extra = map;
        creds
    }

    /// Convert to `HashMap` (for backward compatibility)
    #[must_use]
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = self.extra.clone();

        if let Some(ref username) = self.username {
            map.insert("username".to_string(), username.clone());
        }
        if let Some(ref password) = self.password {
            map.insert("password".to_string(), password.clone());
        }
        if let Some(ref token) = self.token {
            map.insert("token".to_string(), token.clone());
        }
        if let Some(ref api_key) = self.api_key {
            map.insert("api_key".to_string(), api_key.clone());
        }
        if let Some(ref cert_path) = self.cert_path {
            map.insert("cert_path".to_string(), cert_path.clone());
        }
        if let Some(ref key_path) = self.key_path {
            map.insert("key_path".to_string(), key_path.clone());
        }
        if let Some(ref ca_path) = self.ca_path {
            map.insert("ca_path".to_string(), ca_path.clone());
        }

        map
    }
}

/// Canonical service authentication configuration
///
/// This is the unified authentication configuration for all service-to-service
/// authentication in ToadStool. It supports multiple authentication schemes
/// and provides a flexible credential system.
///
/// # Examples
///
/// ```
/// use toadstool_common::auth::{ServiceAuthConfig, AuthType, AuthCredentials};
///
/// // Bearer token auth
/// let config = ServiceAuthConfig {
///     auth_type: AuthType::Bearer,
///     credentials: AuthCredentials::bearer("my-token"),
/// };
///
/// // mTLS auth
/// let config = ServiceAuthConfig {
///     auth_type: AuthType::MutualTLS,
///     credentials: AuthCredentials::mtls(
///         "/path/to/cert.pem",
///         "/path/to/key.pem",
///         Some("/path/to/ca.pem".to_string()),
///     ),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAuthConfig {
    /// Authentication type
    pub auth_type: AuthType,

    /// Authentication credentials
    pub credentials: AuthCredentials,
}

impl Default for ServiceAuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            credentials: AuthCredentials::default(),
        }
    }
}

impl ServiceAuthConfig {
    /// Create a new auth config with no authentication
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Create auth config with bearer token
    pub fn bearer(token: impl Into<String>) -> Self {
        Self {
            auth_type: AuthType::Bearer,
            credentials: AuthCredentials::bearer(token),
        }
    }

    /// Create auth config with API key
    pub fn api_key(key: impl Into<String>) -> Self {
        Self {
            auth_type: AuthType::ApiKey,
            credentials: AuthCredentials::api_key(key),
        }
    }

    /// Create auth config with basic authentication
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            auth_type: AuthType::Basic,
            credentials: AuthCredentials::basic(username, password),
        }
    }

    /// Create auth config for mTLS
    pub fn mtls(
        cert_path: impl Into<String>,
        key_path: impl Into<String>,
        ca_path: Option<String>,
    ) -> Self {
        Self {
            auth_type: AuthType::MutualTLS,
            credentials: AuthCredentials::mtls(cert_path, key_path, ca_path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_credentials_bearer() {
        let creds = AuthCredentials::bearer("test-token");
        assert_eq!(creds.token, Some("test-token".to_string()));
        assert!(creds.username.is_none());
        assert!(creds.password.is_none());
    }

    #[test]
    fn test_auth_credentials_basic() {
        let creds = AuthCredentials::basic("user", "pass");
        assert_eq!(creds.username, Some("user".to_string()));
        assert_eq!(creds.password, Some("pass".to_string()));
    }

    #[test]
    fn test_auth_credentials_mtls() {
        let creds = AuthCredentials::mtls("/cert.pem", "/key.pem", Some("/ca.pem".to_string()));
        assert_eq!(creds.cert_path, Some("/cert.pem".to_string()));
        assert_eq!(creds.key_path, Some("/key.pem".to_string()));
        assert_eq!(creds.ca_path, Some("/ca.pem".to_string()));
    }

    #[test]
    fn test_auth_credentials_to_map() {
        let creds = AuthCredentials::bearer("test-token");
        let map = creds.to_map();
        assert_eq!(map.get("token"), Some(&"test-token".to_string()));
    }

    #[test]
    fn test_service_auth_config_constructors() {
        let config = ServiceAuthConfig::bearer("token");
        assert_eq!(config.auth_type, AuthType::Bearer);

        let config = ServiceAuthConfig::none();
        assert_eq!(config.auth_type, AuthType::None);

        let config = ServiceAuthConfig::api_key("key");
        assert_eq!(config.auth_type, AuthType::ApiKey);
    }
}
