// SPDX-License-Identifier: AGPL-3.0-or-later
//! Cloud credentials and authentication
//!
//! Secret fields use [`toadstool_common::SecretString`] so they are
//! zeroized on drop and never leaked through `Debug`, `Display`, or
//! `Serialize`. Non-secret identifiers remain plain `String`.

use serde::{Deserialize, Serialize};
use toadstool_common::SecretString;

// ============================================================================
// Cloud Provider Credentials
// ============================================================================

/// AWS credentials
///
/// `access_key_id` is an identifier (safe to log).
/// `secret_access_key` and `session_token` are secrets (redacted everywhere).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AWSCredentials {
    /// AWS access key ID (identifier, safe to log).
    pub access_key_id: String,
    /// AWS secret access key (redacted).
    pub secret_access_key: SecretString,
    /// Session token for temporary credentials (optional).
    pub session_token: Option<SecretString>,
}

impl Default for AWSCredentials {
    fn default() -> Self {
        Self {
            access_key_id: String::new(),
            secret_access_key: SecretString::new(String::new()),
            session_token: None,
        }
    }
}

/// Azure credentials.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AzureCredentials {
    /// Azure tenant ID.
    pub tenant_id: String,
    /// Azure client ID.
    pub client_id: String,
    /// Azure client secret (redacted).
    pub client_secret: SecretString,
}

/// GCP credentials.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GCPCredentials {
    /// GCP service account key JSON (redacted).
    pub service_account_key: SecretString,
}

/// Kubernetes configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    /// Path to kubeconfig file.
    pub kubeconfig_path: Option<String>,
    /// Inline kubeconfig content.
    pub kubeconfig_content: Option<String>,
    /// Cluster API endpoint.
    pub cluster_endpoint: Option<String>,
    /// Bearer token for auth.
    pub token: Option<String>,
}

/// Edge mesh configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMeshConfig {
    /// Mesh identifier.
    pub mesh_id: String,
    /// Discovery endpoint URLs.
    pub discovery_endpoints: Vec<String>,
    /// Enable encryption.
    pub encryption_enabled: bool,
}

// ============================================================================
// Authentication & Encryption
// ============================================================================

/// Encryption level options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionLevel {
    /// Standard encryption.
    Standard,
    /// High encryption.
    High,
    /// Maximum encryption.
    Maximum,
}

/// Authentication methods for self-hosted/cloud.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Token-based auth.
    Token {
        /// Bearer token (redacted).
        token: SecretString,
    },
    /// Certificate-based auth.
    Certificate {
        /// Path to certificate.
        cert_path: String,
        /// Path to private key.
        key_path: String,
    },
    /// Auth against a dedicated security service endpoint.
    #[serde(alias = "BearDogAuth")] // legacy alias
    SecurityServiceAuth {
        /// Auth endpoint URL.
        endpoint: String,
        /// Credentials (redacted).
        credentials: SecretString,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_credentials_creation() {
        let creds = AWSCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: SecretString::from("test-secret-value"),
            session_token: Some(SecretString::from("session-token")),
        };
        assert_eq!(creds.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert!(creds.session_token.is_some());
    }

    #[test]
    fn test_aws_credentials_default() {
        let creds = AWSCredentials::default();
        assert!(creds.access_key_id.is_empty());
        assert!(creds.secret_access_key.is_empty());
        assert!(creds.session_token.is_none());
    }

    #[test]
    fn test_aws_secret_is_redacted_in_debug() {
        let creds = AWSCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: SecretString::from("should-not-appear"),
            session_token: None,
        };
        let debug = format!("{creds:?}");
        assert!(debug.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!debug.contains("should-not-appear"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_aws_secret_is_redacted_in_json() {
        let creds = AWSCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: SecretString::from("super-secret"),
            session_token: None,
        };
        let json = serde_json::to_string(&creds).expect("serialize");
        assert!(!json.contains("super-secret"));
        assert!(json.contains("[REDACTED]"));
    }

    #[test]
    fn test_azure_credentials_creation() {
        let creds = AzureCredentials {
            tenant_id: "tenant-123".to_string(),
            client_id: "client-456".to_string(),
            client_secret: SecretString::from("secret"),
        };
        assert_eq!(creds.tenant_id, "tenant-123");
        assert_eq!(creds.client_id, "client-456");
    }

    #[test]
    fn test_gcp_credentials_creation() {
        let creds = GCPCredentials {
            service_account_key: SecretString::from(r#"{"type":"service_account"}"#),
        };
        assert!(
            creds
                .service_account_key
                .expose_secret()
                .contains("service_account")
        );
    }

    #[test]
    fn test_kubernetes_config_creation() {
        let config = KubernetesConfig {
            kubeconfig_path: Some("/path/to/kubeconfig".to_string()),
            kubeconfig_content: None,
            cluster_endpoint: Some("https://cluster.example.com".to_string()),
            token: Some("bearer-token".to_string()),
        };
        assert_eq!(
            config.kubeconfig_path.as_deref(),
            Some("/path/to/kubeconfig")
        );
        assert_eq!(
            config.cluster_endpoint.as_deref(),
            Some("https://cluster.example.com")
        );
    }

    #[test]
    fn test_edge_mesh_config_creation() {
        let config = EdgeMeshConfig {
            mesh_id: "mesh-1".to_string(),
            discovery_endpoints: vec!["http://discovery:8080".to_string()],
            encryption_enabled: true,
        };
        assert_eq!(config.mesh_id, "mesh-1");
        assert!(config.encryption_enabled);
    }

    #[test]
    fn test_encryption_level_variants() {
        let _ = EncryptionLevel::Standard;
        let _ = EncryptionLevel::High;
        let _ = EncryptionLevel::Maximum;
    }

    #[test]
    fn test_auth_method_variants() {
        let _ = AuthMethod::Token {
            token: SecretString::from("t"),
        };
        let _ = AuthMethod::Certificate {
            cert_path: "/cert".to_string(),
            key_path: "/key".to_string(),
        };
        let _ = AuthMethod::SecurityServiceAuth {
            endpoint: "http://auth".to_string(),
            credentials: SecretString::from("creds"),
        };
    }

    #[test]
    fn test_auth_method_token_redacted_in_json() {
        let auth = AuthMethod::Token {
            token: SecretString::from("my-real-token"),
        };
        let json = serde_json::to_string(&auth).expect("serialize");
        assert!(!json.contains("my-real-token"));
        assert!(json.contains("[REDACTED]"));
    }
}
