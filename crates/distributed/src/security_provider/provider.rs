// SPDX-License-Identifier: AGPL-3.0-or-later
//! SecurityProvider trait - the core abstraction
//!
//! This trait defines what ANY security provider must be able to do.
//! Security, HSM, KMS, local keyring - all implement this same trait.

use serde::{Deserialize, Serialize};
use std::future::Future;
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
pub trait SecurityProvider: Send + Sync {
    /// Get provider capabilities
    ///
    /// Returns what security features this provider supports.
    /// Used by Universal Adapter for best-match selection.
    fn capabilities(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<SecurityCapability>>> + Send + '_;

    /// Get provider metadata
    ///
    /// Returns information about this provider (type, version, etc.).
    /// Note: Does NOT return primal name! Returns generic metadata.
    fn metadata(&self) -> impl Future<Output = ToadStoolResult<ProviderMetadata>> + Send + '_;

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
    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<EncryptionOptions>,
    ) -> impl Future<Output = ToadStoolResult<EncryptionResult>> + Send + 'a;

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
    fn decrypt<'a>(
        &'a self,
        ciphertext: &'a [u8],
        metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<DecryptionResult>> + Send + 'a;

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
    fn sign<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<SigningOptions>,
    ) -> impl Future<Output = ToadStoolResult<SignatureResult>> + Send + 'a;

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
    fn verify<'a>(
        &'a self,
        data: &'a [u8],
        signature: &'a [u8],
        public_key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VerificationResult>> + Send + 'a;

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
    fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> impl Future<Output = ToadStoolResult<SecurityPermission>> + Send + '_;

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
    fn validate_permission<'a>(
        &'a self,
        permission: &'a SecurityPermission,
    ) -> impl Future<Output = ToadStoolResult<PermissionValidationResult>> + Send + 'a;

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
    fn revoke_permission<'a>(
        &'a self,
        permission_id: &'a uuid::Uuid,
        reason: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Health check
    ///
    /// Checks if this provider is healthy and operational.
    ///
    /// # Returns
    ///
    /// Provider health status
    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_;
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
impl SecurityProvider for MockSecurityProvider {
    fn capabilities(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<SecurityCapability>>> + Send + '_ {
        let caps = self.capabilities.clone();
        async move { Ok(caps) }
    }

    fn metadata(&self) -> impl Future<Output = ToadStoolResult<ProviderMetadata>> + Send + '_ {
        async {
            Ok(ProviderMetadata {
                provider_id: uuid::Uuid::new_v4().to_string(),
                provider_type: "mock".to_string(),
                provider_version: "1.0.0".to_string(),
                metadata: std::collections::HashMap::new(),
            })
        }
    }

    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        _options: Option<EncryptionOptions>,
    ) -> impl Future<Output = ToadStoolResult<EncryptionResult>> + Send + 'a {
        async move {
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
    }

    fn decrypt<'a>(
        &'a self,
        ciphertext: &'a [u8],
        _metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<DecryptionResult>> + Send + 'a {
        async move {
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
    }

    fn sign<'a>(
        &'a self,
        _data: &'a [u8],
        _options: Option<SigningOptions>,
    ) -> impl Future<Output = ToadStoolResult<SignatureResult>> + Send + 'a {
        async {
            Ok(SignatureResult {
                signature: vec![0xDE, 0xAD, 0xBE, 0xEF],
                algorithm: SignatureAlgorithm::Ed25519,
                key_id: "mock-key".to_string(),
                signed_at: std::time::SystemTime::now(),
            })
        }
    }

    fn verify<'a>(
        &'a self,
        _data: &'a [u8],
        _signature: &'a [u8],
        _public_key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VerificationResult>> + Send + 'a {
        async { Ok(VerificationResult::Valid) }
    }

    fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> impl Future<Output = ToadStoolResult<SecurityPermission>> + Send + '_ {
        async move {
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
    }

    fn validate_permission<'a>(
        &'a self,
        _permission: &'a SecurityPermission,
    ) -> impl Future<Output = ToadStoolResult<PermissionValidationResult>> + Send + 'a {
        async { Ok(PermissionValidationResult::Valid) }
    }

    fn revoke_permission<'a>(
        &'a self,
        _permission_id: &'a uuid::Uuid,
        _reason: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async { Ok(()) }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_ {
        async { Ok(ProviderHealth::Healthy) }
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
