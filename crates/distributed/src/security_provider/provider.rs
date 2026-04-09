// SPDX-License-Identifier: AGPL-3.0-or-later
//! SecurityProvider trait - the core abstraction
//!
//! This trait defines what ANY security provider must be able to do.
//! Security, HSM, KMS, local keyring - all implement this same trait.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use toadstool::error::ToadStoolResult;

use super::types::{
    DecryptionResult, EncryptionMetadata, EncryptionResult, PermissionRequest, ProviderMetadata,
    SecurityPermission, SignatureAlgorithm, SignatureResult, VerificationResult,
};

#[cfg(any(test, feature = "test-mocks"))]
use super::types::{DecryptionMetadata, SecurityProof};

/// Security Provider - generic interface for security operations
///
/// ANY primal or service can implement this trait to provide security capabilities.
/// The Universal Adapter discovers WHO implements it at runtime.
///
/// ## Implementations
///
/// - `DistributedSecurityProvider`: Default bundled [`SecurityProvider`] implementation
/// - `LocalKeyringProvider`: Local OS keyring (future)
/// - `HSMProvider`: Hardware Security Module (future)
/// - `CloudKMSProvider`: Cloud Key Management Service (future)
/// - `MockSecurityProvider`: Test-only (`cfg(test)` or `feature = "test-mocks"`)
///
/// ## Deep Debt Compliance
///
/// - ✅ No hardcoding: Generic trait, works with ANY implementation
/// - ✅ Capability-based: Consumers request capabilities, not specific providers
/// - ✅ Runtime discovery: Universal Adapter finds best provider
/// - ✅ Self-knowledge: Each provider knows only itself
/// - ✅ Testable: Easy to mock for testing
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait SecurityProvider: Send + Sync {
    /// Get provider capabilities
    ///
    /// Returns what security features this provider supports.
    /// Used by Universal Adapter for best-match selection.
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>>;

    /// Get provider metadata
    ///
    /// Returns information about this provider (type, version, etc.).
    /// Note: Does NOT return primal name! Returns generic metadata.
    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata>;

    /// Encrypt data
    ///
    /// Encrypts the given plaintext using this provider's encryption capabilities.
    ///
    /// # Arguments
    ///
    /// * `data` - Plaintext to encrypt
    /// * `options` - Optional encryption options (algorithm, key ID, etc.)
    ///
    /// # Returns
    ///
    /// Encrypted data with metadata
    async fn encrypt(
        &self,
        data: &[u8],
        options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult>;

    /// Decrypt data
    ///
    /// Decrypts the given ciphertext using this provider's decryption capabilities.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - Encrypted data
    /// * `metadata` - Encryption metadata (algorithm, key ID, etc.)
    ///
    /// # Returns
    ///
    /// Decrypted plaintext
    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult>;

    /// Sign data
    ///
    /// Creates a cryptographic signature for the given data.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to sign
    /// * `options` - Optional signing options (algorithm, key ID, etc.)
    ///
    /// # Returns
    ///
    /// Signature with metadata
    async fn sign(
        &self,
        data: &[u8],
        options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult>;

    /// Verify signature
    ///
    /// Verifies a cryptographic signature.
    ///
    /// # Arguments
    ///
    /// * `data` - Original data that was signed
    /// * `signature` - Signature to verify
    /// * `public_key_id` - Public key identifier
    ///
    /// # Returns
    ///
    /// Verification result
    async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult>;

    /// Create a permission
    ///
    /// Issues a cryptographic permission for external integration access.
    ///
    /// # Arguments
    ///
    /// * `request` - Permission request details
    ///
    /// # Returns
    ///
    /// Signed permission with cryptographic proof
    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission>;

    /// Validate a permission
    ///
    /// Validates a permission's cryptographic proof and expiry.
    ///
    /// # Arguments
    ///
    /// * `permission` - Permission to validate
    ///
    /// # Returns
    ///
    /// Validation result
    async fn validate_permission(
        &self,
        permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult>;

    /// Revoke a permission
    ///
    /// Revokes a previously issued permission.
    ///
    /// # Arguments
    ///
    /// * `permission_id` - ID of permission to revoke
    /// * `reason` - Reason for revocation
    ///
    /// # Returns
    ///
    /// Revocation confirmation
    async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()>;

    /// Health check
    ///
    /// Checks if this provider is healthy and operational.
    ///
    /// # Returns
    ///
    /// Provider health status
    async fn health_check(&self) -> ToadStoolResult<ProviderHealth>;
}

/// Security capabilities a provider can offer
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityCapability {
    /// Symmetric encryption (AES, etc.)
    SymmetricEncryption,

    /// Asymmetric encryption (RSA, etc.)
    AsymmetricEncryption,

    /// Digital signatures (ECDSA, Ed25519, etc.)
    DigitalSignatures,

    /// Key management
    KeyManagement,

    /// Permission issuance
    PermissionIssuance,

    /// Certificate authority
    CertificateAuthority,

    /// Hardware security module
    HardwareSecurityModule,

    /// Quantum-resistant crypto
    QuantumResistant,

    /// Audit logging
    AuditLogging,
}

/// Encryption options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionOptions {
    /// Encryption algorithm to use
    pub algorithm: Option<String>,

    /// Key ID to use
    pub key_id: Option<String>,

    /// Additional authenticated data (for AEAD)
    pub aad: Option<Vec<u8>>,
}

/// Signing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningOptions {
    /// Signature algorithm to use
    pub algorithm: Option<SignatureAlgorithm>,

    /// Key ID to use
    pub key_id: Option<String>,
}

/// Permission validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionValidationResult {
    /// Permission is valid
    Valid,

    /// Permission signature is invalid
    InvalidSignature,

    /// Permission has expired
    Expired,

    /// Permission has been revoked
    Revoked,

    /// Permission not found
    NotFound,
}

/// Provider health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderHealth {
    /// Provider is healthy and operational
    Healthy,

    /// Provider is degraded but functional
    Degraded,

    /// Provider is unhealthy (should not use)
    Unhealthy,
}

/// Mock security provider for testing
#[cfg(any(test, feature = "test-mocks"))]
pub struct MockSecurityProvider {
    capabilities: Vec<SecurityCapability>,
}

#[cfg(any(test, feature = "test-mocks"))]
impl Default for MockSecurityProvider {
    fn default() -> Self {
        Self {
            capabilities: vec![
                SecurityCapability::SymmetricEncryption,
                SecurityCapability::DigitalSignatures,
            ],
        }
    }
}

#[cfg(any(test, feature = "test-mocks"))]
impl MockSecurityProvider {
    /// Create a new mock security provider with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(any(test, feature = "test-mocks"))]
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl SecurityProvider for MockSecurityProvider {
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>> {
        Ok(self.capabilities.clone())
    }

    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata> {
        Ok(ProviderMetadata {
            provider_id: uuid::Uuid::new_v4().to_string(),
            provider_type: "mock".to_string(),
            provider_version: "1.0.0".to_string(),
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn encrypt(
        &self,
        data: &[u8],
        _options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult> {
        // Mock encryption: just reverse bytes
        let mut ciphertext = data.to_vec();
        ciphertext.reverse();

        Ok(EncryptionResult {
            ciphertext,
            iv: None,
            auth_tag: None,
            metadata: EncryptionMetadata {
                algorithm: "mock".to_string(),
                key_id: "mock-key".to_string(),
                encrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        _metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        // Mock decryption: reverse bytes back
        let mut plaintext = ciphertext.to_vec();
        plaintext.reverse();

        Ok(DecryptionResult {
            plaintext,
            metadata: DecryptionMetadata {
                key_id: "mock-key".to_string(),
                decrypted_at: std::time::SystemTime::now(),
            },
        })
    }

    async fn sign(
        &self,
        _data: &[u8],
        _options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        Ok(SignatureResult {
            signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: "mock-key".to_string(),
            signed_at: std::time::SystemTime::now(),
        })
    }

    async fn verify(
        &self,
        _data: &[u8],
        _signature: &[u8],
        _public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        Ok(VerificationResult::Valid)
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        let now = std::time::SystemTime::now();

        Ok(SecurityPermission {
            permission_id: uuid::Uuid::new_v4(),
            holder_id: request.requester_id,
            target: request.target,
            scope: request.scope,
            valid_from: now,
            valid_until: now + request.validity_duration,
            proof: SecurityProof {
                signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
                algorithm: SignatureAlgorithm::Ed25519,
                public_key_id: "mock-key".to_string(),
                signed_at: now,
            },
            provider_metadata: ProviderMetadata {
                provider_id: uuid::Uuid::new_v4().to_string(),
                provider_type: "mock".to_string(),
                provider_version: "1.0.0".to_string(),
                metadata: std::collections::HashMap::new(),
            },
        })
    }

    async fn validate_permission(
        &self,
        _permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        Ok(PermissionValidationResult::Valid)
    }

    async fn revoke_permission(
        &self,
        _permission_id: &uuid::Uuid,
        _reason: &str,
    ) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        Ok(ProviderHealth::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_capabilities() {
        let provider = MockSecurityProvider::new();
        let caps = provider.capabilities().await.unwrap();

        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
    }

    #[tokio::test]
    async fn test_mock_provider_encrypt_decrypt() {
        let provider = MockSecurityProvider::new();
        let data = b"test data";

        let encrypted = provider.encrypt(data, None).await.unwrap();
        let decrypted = provider
            .decrypt(&encrypted.ciphertext, &encrypted.metadata)
            .await
            .unwrap();

        assert_eq!(decrypted.plaintext, data);
    }

    #[tokio::test]
    async fn test_mock_provider_sign() {
        let provider = MockSecurityProvider::new();
        let data = b"test data";

        let signature = provider.sign(data, None).await.unwrap();
        assert!(!signature.signature.is_empty());
    }

    #[tokio::test]
    async fn test_mock_provider_health() {
        let provider = MockSecurityProvider::new();
        let health = provider.health_check().await.unwrap();

        assert_eq!(health, ProviderHealth::Healthy);
    }

    #[tokio::test]
    async fn test_permission_validation_result() {
        assert_eq!(
            PermissionValidationResult::Valid,
            PermissionValidationResult::Valid
        );
        assert_ne!(
            PermissionValidationResult::Valid,
            PermissionValidationResult::Expired
        );
    }
}
