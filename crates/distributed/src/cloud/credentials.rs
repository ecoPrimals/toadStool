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
