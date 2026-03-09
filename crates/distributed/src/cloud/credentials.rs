// SPDX-License-Identifier: AGPL-3.0-only
//! Cloud credentials and authentication
//!
//! This module contains credential structures for different cloud providers
//! and authentication methods.

use serde::{Deserialize, Serialize};

// ============================================================================
// Cloud Provider Credentials
// ============================================================================

/// AWS credentials
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AWSCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
}

/// Azure credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureCredentials {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

/// GCP credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCPCredentials {
    pub service_account_key: String,
}

/// Kubernetes configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesConfig {
    pub kubeconfig_path: Option<String>,
    pub kubeconfig_content: Option<String>,
    pub cluster_endpoint: Option<String>,
    pub token: Option<String>,
}

/// Edge mesh configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeMeshConfig {
    pub mesh_id: String,
    pub discovery_endpoints: Vec<String>,
    pub encryption_enabled: bool,
}

// ============================================================================
// Authentication & Encryption
// ============================================================================

/// Encryption level options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionLevel {
    Standard,
    High,
    Maximum,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Token {
        token: String,
    },
    Certificate {
        cert_path: String,
        key_path: String,
    },
    BearDogAuth {
        endpoint: String,
        credentials: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aws_credentials_creation() {
        let creds = AWSCredentials {
            access_key_id: "AKIAIOSFODNN7EXAMPLE".to_string(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            session_token: Some("session-token".to_string()),
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
    fn test_azure_credentials_creation() {
        let creds = AzureCredentials {
            tenant_id: "tenant-123".to_string(),
            client_id: "client-456".to_string(),
            client_secret: "secret".to_string(),
        };
        assert_eq!(creds.tenant_id, "tenant-123");
        assert_eq!(creds.client_id, "client-456");
    }

    #[test]
    fn test_gcp_credentials_creation() {
        let creds = GCPCredentials {
            service_account_key: r#"{"type":"service_account"}"#.to_string(),
        };
        assert!(creds.service_account_key.contains("service_account"));
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
            token: "t".to_string(),
        };
        let _ = AuthMethod::Certificate {
            cert_path: "/cert".to_string(),
            key_path: "/key".to_string(),
        };
        let _ = AuthMethod::BearDogAuth {
            endpoint: "http://auth".to_string(),
            credentials: "creds".to_string(),
        };
    }

    #[test]
    fn test_aws_credentials_serialization() {
        let creds = AWSCredentials {
            access_key_id: "key".to_string(),
            secret_access_key: "secret".to_string(),
            session_token: None,
        };
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: AWSCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_key_id, creds.access_key_id);
    }

    #[test]
    fn test_azure_credentials_serialization() {
        let creds = AzureCredentials {
            tenant_id: "t".to_string(),
            client_id: "c".to_string(),
            client_secret: "s".to_string(),
        };
        let json = serde_json::to_string(&creds).unwrap();
        let parsed: AzureCredentials = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tenant_id, creds.tenant_id);
    }

    #[test]
    fn test_auth_method_serialization() {
        let auth = AuthMethod::Token {
            token: "my-token".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        let parsed: AuthMethod = serde_json::from_str(&json).unwrap();
        match parsed {
            AuthMethod::Token { token } => assert_eq!(token, "my-token"),
            _ => panic!("Expected Token variant"),
        }
    }
}
