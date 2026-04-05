// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// Cloud provider for permission targeting.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum CloudProvider {
    /// Amazon Web Services.
    AWS,
    /// Microsoft Azure.
    Azure,
    /// Google Cloud Platform.
    GCP,
    /// DigitalOcean.
    DigitalOcean,
    /// Linode.
    Linode,
    /// Vultr.
    Vultr,
    /// Hetzner.
    Hetzner,
    /// OVH.
    OVH,
    /// Scaleway.
    Scaleway,
}

/// Container platform for permission targeting.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContainerPlatform {
    /// Docker.
    Docker,
    /// Kubernetes.
    Kubernetes,
    /// Nomad.
    Nomad,
    /// OpenShift.
    OpenShift,
    /// Docker Swarm.
    DockerSwarm,
    /// Podman.
    Podman,
}

/// Quantum provider for permission targeting.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantumProvider {
    /// IBM Quantum.
    IBM,
    /// Google Quantum.
    Google,
    /// IonQ.
    IonQ,
    /// Rigetti.
    Rigetti,
    /// AWS Braket.
    AWSBraket,
    /// Azure Quantum.
    AzureQuantum,
}

/// HPC scheduler for permission targeting.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum HPCScheduler {
    /// SLURM.
    SLURM,
    /// PBS.
    PBS,
    /// SGE.
    SGE,
    /// LSF.
    LSF,
    /// Custom scheduler.
    Custom,
}

/// Service tier for enterprise services.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServiceTier {
    /// Basic tier.
    Basic,
    /// Professional tier.
    Professional,
    /// Enterprise tier.
    Enterprise,
    /// Premium tier.
    Premium,
}

/// External target that needs permission (unified with crypto_lock)
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExternalTarget {
    /// Cloud provider APIs
    CloudProvider {
        /// Cloud vendor.
        provider: CloudProvider,
        /// Allowed region identifiers.
        regions: Vec<String>,
        /// Service names or API families.
        services: Vec<String>,
    },
    /// Container orchestration platforms
    ContainerPlatform {
        /// Orchestrator product.
        platform: ContainerPlatform,
        /// Cluster names in scope.
        clusters: Vec<String>,
        /// Kubernetes-style namespaces or equivalents.
        namespaces: Vec<String>,
    },
    /// External tools and services
    ExternalTool {
        /// Logical tool or integration name.
        tool_name: String,
        /// HTTP or RPC endpoints.
        api_endpoints: Vec<String>,
        /// Feature flags or modules enabled.
        feature_set: Vec<String>,
    },
    /// Quantum computing platforms
    QuantumProvider {
        /// Quantum cloud vendor.
        provider: QuantumProvider,
        /// Named quantum backends.
        backends: Vec<String>,
        /// Optional maximum qubit count.
        qubit_limits: Option<u32>,
    },
    /// HPC and supercomputing clusters
    HPCCluster {
        /// Cluster display name.
        cluster_name: String,
        /// Job scheduler in use.
        scheduler: HPCScheduler,
        /// Scheduler partition or queue names.
        partitions: Vec<String>,
    },
    /// Enterprise and commercial services
    EnterpriseService {
        /// Product or service name.
        service_name: String,
        /// Commercial tier.
        tier: ServiceTier,
        /// Enabled product features.
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

    /// Valid time range (inclusive start)
    pub valid_from: SystemTime,
    /// Valid time range (inclusive end)
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
    /// ECDSA over P-256.
    EcdsaP256,
    /// ECDSA over P-384.
    EcdsaP384,
    /// Ed25519.
    Ed25519,
    /// RSA-4096.
    Rsa4096,
    /// Provider-specific or custom algorithm.
    Custom,
}

/// Provider metadata (identifies WHO issued permission)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    /// Provider ID (NOT primal name! UUID or similar)
    pub provider_id: String,

    /// Provider type (security, hsm, kms, local-keyring, etc.)
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
    /// Signature verifies successfully.
    Valid,
    /// Signature does not verify.
    Invalid,
    /// Public key or key id was not found.
    KeyNotFound,
    /// Algorithm does not match the key or request.
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
