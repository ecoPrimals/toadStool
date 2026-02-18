//! Common types for security providers

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Permission request for security provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    /// Requester identification
    pub requester_id: String,

    /// What external integration needs access
    pub target: ExternalTarget,

    /// Requested permission scope
    pub scope: PermissionScope,

    /// How long permission should be valid
    pub validity_duration: std::time::Duration,

    /// Optional delegation information
    pub delegation_info: Option<DelegationInfo>,
}

// Re-export supporting enums (used by ExternalTarget)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    Azure,
    GCP,
    DigitalOcean,
    Linode,
    Vultr,
    Hetzner,
    OVH,
    Scaleway,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContainerPlatform {
    Docker,
    Kubernetes,
    Nomad,
    OpenShift,
    DockerSwarm,
    Podman,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantumProvider {
    IBM,
    Google,
    IonQ,
    Rigetti,
    AWSBraket,
    AzureQuantum,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum HPCScheduler {
    SLURM,
    PBS,
    SGE,
    LSF,
    Custom,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServiceTier {
    Basic,
    Professional,
    Enterprise,
    Premium,
}

/// External target that needs permission (unified with crypto_lock)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExternalTarget {
    /// Cloud provider APIs
    CloudProvider {
        provider: CloudProvider,
        regions: Vec<String>,
        services: Vec<String>,
    },
    /// Container orchestration platforms
    ContainerPlatform {
        platform: ContainerPlatform,
        clusters: Vec<String>,
        namespaces: Vec<String>,
    },
    /// External tools and services
    ExternalTool {
        tool_name: String,
        api_endpoints: Vec<String>,
        feature_set: Vec<String>,
    },
    /// Quantum computing platforms
    QuantumProvider {
        provider: QuantumProvider,
        backends: Vec<String>,
        qubit_limits: Option<u32>,
    },
    /// HPC and supercomputing clusters
    HPCCluster {
        cluster_name: String,
        scheduler: HPCScheduler,
        partitions: Vec<String>,
    },
    /// Enterprise and commercial services
    EnterpriseService {
        service_name: String,
        tier: ServiceTier,
        features: Vec<String>,
    },
}

/// Permission scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionScope {
    /// Operations allowed
    pub operations: Vec<String>,

    /// Resource limits
    pub resource_limits: ResourceLimits,

    /// Geographic restrictions
    pub geo_restrictions: Vec<String>,
}

/// Resource limits for permissions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimits {
    /// Maximum CPU cores
    pub max_cpu: Option<u32>,

    /// Maximum memory in GB
    pub max_memory_gb: Option<f64>,

    /// Maximum storage in GB
    pub max_storage_gb: Option<f64>,

    /// Maximum API calls per hour
    pub max_api_calls_per_hour: Option<u64>,
}

/// Delegation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationInfo {
    /// Original permission holder
    pub original_holder: String,

    /// Delegation scope (may be more restrictive)
    pub delegated_scope: PermissionScope,

    /// Maximum delegation depth
    pub max_depth: u32,
}

/// Security provider permission (generic, primal-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPermission {
    /// Permission ID
    pub permission_id: Uuid,

    /// Who holds this permission
    pub holder_id: String,

    /// What this permission grants access to
    pub target: ExternalTarget,

    /// Permission scope
    pub scope: PermissionScope,

    /// Valid time range
    pub valid_from: SystemTime,
    pub valid_until: SystemTime,

    /// Cryptographic proof
    pub proof: SecurityProof,

    /// Provider metadata (who issued this)
    pub provider_metadata: ProviderMetadata,
}

/// Cryptographic proof from security provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProof {
    /// Signature bytes
    pub signature: Vec<u8>,

    /// Algorithm used
    pub algorithm: SignatureAlgorithm,

    /// Public key identifier
    pub public_key_id: String,

    /// Timestamp when signed
    pub signed_at: SystemTime,
}

/// Signature algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    EcdsaP256,
    EcdsaP384,
    Ed25519,
    Rsa4096,
    Custom,
}

/// Provider metadata (identifies WHO issued permission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    /// Provider ID (NOT primal name! UUID or similar)
    pub provider_id: String,

    /// Provider type (beardog, hsm, kms, local-keyring, etc.)
    pub provider_type: String,

    /// Provider version
    pub provider_version: String,

    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Encryption result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionResult {
    /// Encrypted data
    pub ciphertext: Vec<u8>,

    /// Initialization vector (if applicable)
    pub iv: Option<Vec<u8>>,

    /// Authentication tag (for AEAD)
    pub auth_tag: Option<Vec<u8>>,

    /// Encryption metadata
    pub metadata: EncryptionMetadata,
}

/// Encryption metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Algorithm used
    pub algorithm: String,

    /// Key identifier
    pub key_id: String,

    /// Timestamp
    pub encrypted_at: SystemTime,
}

/// Decryption result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionResult {
    /// Decrypted plaintext
    pub plaintext: Vec<u8>,

    /// Decryption metadata
    pub metadata: DecryptionMetadata,
}

/// Decryption metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionMetadata {
    /// Key used
    pub key_id: String,

    /// Timestamp
    pub decrypted_at: SystemTime,
}

/// Signature result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    /// Signature bytes
    pub signature: Vec<u8>,

    /// Algorithm used
    pub algorithm: SignatureAlgorithm,

    /// Key identifier
    pub key_id: String,

    /// Timestamp
    pub signed_at: SystemTime,
}

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    Valid,
    Invalid,
    KeyNotFound,
    AlgorithmMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_request_creation() {
        let request = PermissionRequest {
            requester_id: "user123".to_string(),
            target: ExternalTarget::CloudProvider {
                provider: CloudProvider::AWS,
                regions: vec!["us-east-1".to_string()],
                services: vec![],
            },
            scope: PermissionScope {
                operations: vec!["read".to_string()],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: vec![],
            },
            validity_duration: std::time::Duration::from_secs(3600),
            delegation_info: None,
        };

        assert_eq!(request.requester_id, "user123");
    }

    #[test]
    fn test_signature_algorithms() {
        assert_ne!(SignatureAlgorithm::EcdsaP256, SignatureAlgorithm::Ed25519);
        assert_eq!(SignatureAlgorithm::EcdsaP256, SignatureAlgorithm::EcdsaP256);
    }

    #[test]
    fn test_verification_result() {
        assert_eq!(VerificationResult::Valid, VerificationResult::Valid);
        assert_ne!(VerificationResult::Valid, VerificationResult::Invalid);
    }

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert!(limits.max_cpu.is_none());
        assert!(limits.max_memory_gb.is_none());
    }
}
