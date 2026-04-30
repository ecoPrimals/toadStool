// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concrete security provider dispatch — replaces `dyn SecurityProvider` with an enum.

use std::future::Future;

use toadstool::error::ToadStoolResult;

use super::provider::{
    EncryptionOptions, PermissionValidationResult, ProviderHealth, SecurityCapability,
    SecurityProvider, SigningOptions,
};
use super::security_impl::DistributedSecurityProvider;
use super::tcp_provider::TcpSecurityProvider;
use super::types::{
    DecryptionResult, EncryptionMetadata, EncryptionResult, PermissionRequest, ProviderMetadata,
    SecurityPermission, SignatureResult, VerificationResult,
};
use super::unix_socket_provider::UnixSocketSecurityProvider;

#[cfg(feature = "dev-crypto")]
use super::local_keyring::LocalKeyringProvider;
#[cfg(feature = "dev-crypto")]
use super::software_hsm::SoftwareHsmProvider;

/// Production and test security providers behind a single concrete type.
pub enum SecurityProviderDispatch {
    /// Unix domain socket JSON-RPC transport.
    UnixSocket(UnixSocketSecurityProvider),
    /// TCP JSON-RPC transport.
    Tcp(TcpSecurityProvider),
    /// In-process Security primal adapter.
    Distributed(DistributedSecurityProvider),
    /// Dev/CI software HSM (`dev-crypto` feature).
    #[cfg(feature = "dev-crypto")]
    SoftwareHsm(SoftwareHsmProvider),
    /// Dev/CI local keyring (`dev-crypto` feature).
    #[cfg(feature = "dev-crypto")]
    LocalKeyring(LocalKeyringProvider),
    /// Test mock (`test-mocks` / unit tests).
    #[cfg(any(test, feature = "test-mocks"))]
    Mock(super::provider::MockSecurityProvider),
}

impl SecurityProvider for SecurityProviderDispatch {
    fn capabilities(
        &self,
    ) -> impl Future<Output = ToadStoolResult<Vec<SecurityCapability>>> + Send + '_ {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.capabilities().await,
                SecurityProviderDispatch::Tcp(p) => p.capabilities().await,
                SecurityProviderDispatch::Distributed(p) => p.capabilities().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.capabilities().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.capabilities().await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.capabilities().await,
            }
        }
    }

    fn metadata(&self) -> impl Future<Output = ToadStoolResult<ProviderMetadata>> + Send + '_ {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.metadata().await,
                SecurityProviderDispatch::Tcp(p) => p.metadata().await,
                SecurityProviderDispatch::Distributed(p) => p.metadata().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.metadata().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.metadata().await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.metadata().await,
            }
        }
    }

    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<EncryptionOptions>,
    ) -> impl Future<Output = ToadStoolResult<EncryptionResult>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.encrypt(data, options).await,
                SecurityProviderDispatch::Tcp(p) => p.encrypt(data, options).await,
                SecurityProviderDispatch::Distributed(p) => p.encrypt(data, options).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.encrypt(data, options).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.encrypt(data, options).await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.encrypt(data, options).await,
            }
        }
    }

    fn decrypt<'a>(
        &'a self,
        ciphertext: &'a [u8],
        metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<DecryptionResult>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.decrypt(ciphertext, metadata).await,
                SecurityProviderDispatch::Tcp(p) => p.decrypt(ciphertext, metadata).await,
                SecurityProviderDispatch::Distributed(p) => p.decrypt(ciphertext, metadata).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.decrypt(ciphertext, metadata).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.decrypt(ciphertext, metadata).await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.decrypt(ciphertext, metadata).await,
            }
        }
    }

    fn sign<'a>(
        &'a self,
        data: &'a [u8],
        options: Option<SigningOptions>,
    ) -> impl Future<Output = ToadStoolResult<SignatureResult>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.sign(data, options).await,
                SecurityProviderDispatch::Tcp(p) => p.sign(data, options).await,
                SecurityProviderDispatch::Distributed(p) => p.sign(data, options).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.sign(data, options).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.sign(data, options).await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.sign(data, options).await,
            }
        }
    }

    fn verify<'a>(
        &'a self,
        data: &'a [u8],
        signature: &'a [u8],
        public_key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VerificationResult>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => {
                    p.verify(data, signature, public_key_id).await
                }
                SecurityProviderDispatch::Tcp(p) => p.verify(data, signature, public_key_id).await,
                SecurityProviderDispatch::Distributed(p) => {
                    p.verify(data, signature, public_key_id).await
                }
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => {
                    p.verify(data, signature, public_key_id).await
                }
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => {
                    p.verify(data, signature, public_key_id).await
                }
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.verify(data, signature, public_key_id).await,
            }
        }
    }

    fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> impl Future<Output = ToadStoolResult<SecurityPermission>> + Send + '_ {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.create_permission(request).await,
                SecurityProviderDispatch::Tcp(p) => p.create_permission(request).await,
                SecurityProviderDispatch::Distributed(p) => p.create_permission(request).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.create_permission(request).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.create_permission(request).await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.create_permission(request).await,
            }
        }
    }

    fn validate_permission<'a>(
        &'a self,
        permission: &'a SecurityPermission,
    ) -> impl Future<Output = ToadStoolResult<PermissionValidationResult>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.validate_permission(permission).await,
                SecurityProviderDispatch::Tcp(p) => p.validate_permission(permission).await,
                SecurityProviderDispatch::Distributed(p) => p.validate_permission(permission).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.validate_permission(permission).await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => {
                    p.validate_permission(permission).await
                }
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.validate_permission(permission).await,
            }
        }
    }

    fn revoke_permission<'a>(
        &'a self,
        permission_id: &'a uuid::Uuid,
        reason: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
                SecurityProviderDispatch::Tcp(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
                SecurityProviderDispatch::Distributed(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => {
                    p.revoke_permission(permission_id, reason).await
                }
            }
        }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_ {
        async move {
            match self {
                SecurityProviderDispatch::UnixSocket(p) => p.health_check().await,
                SecurityProviderDispatch::Tcp(p) => p.health_check().await,
                SecurityProviderDispatch::Distributed(p) => p.health_check().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::SoftwareHsm(p) => p.health_check().await,
                #[cfg(feature = "dev-crypto")]
                SecurityProviderDispatch::LocalKeyring(p) => p.health_check().await,
                #[cfg(any(test, feature = "test-mocks"))]
                SecurityProviderDispatch::Mock(p) => p.health_check().await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::provider::MockSecurityProvider;
    use super::*;

    fn mock_dispatch() -> SecurityProviderDispatch {
        SecurityProviderDispatch::Mock(MockSecurityProvider::new())
    }

    #[tokio::test]
    async fn mock_capabilities_returns_defaults() {
        let d = mock_dispatch();
        let caps = d.capabilities().await.unwrap();
        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
    }

    #[tokio::test]
    async fn mock_metadata_returns_valid() {
        let d = mock_dispatch();
        let meta = d.metadata().await.unwrap();
        assert_eq!(meta.provider_type, "mock");
        assert_eq!(meta.provider_version, "1.0.0");
    }

    #[tokio::test]
    async fn mock_encrypt_decrypt_roundtrip() {
        let d = mock_dispatch();
        let plaintext = b"hello world";
        let encrypted = d.encrypt(plaintext, None).await.unwrap();
        assert_ne!(encrypted.ciphertext, plaintext);

        let decrypted = d
            .decrypt(&encrypted.ciphertext, &encrypted.metadata)
            .await
            .unwrap();
        assert_eq!(decrypted.plaintext, plaintext);
    }

    #[tokio::test]
    async fn mock_sign_verify_roundtrip() {
        let d = mock_dispatch();
        let data = b"test data";
        let sig = d.sign(data, None).await.unwrap();
        assert!(!sig.signature.is_empty());

        let verify = d.verify(data, &sig.signature, &sig.key_id).await.unwrap();
        assert!(matches!(verify, VerificationResult::Valid));
    }

    #[tokio::test]
    async fn mock_create_validate_permission() {
        use super::super::types::{ExternalTarget, PermissionScope, ResourceLimits};
        let d = mock_dispatch();
        let request = PermissionRequest {
            requester_id: "test-subject".to_string(),
            target: ExternalTarget::ExternalTool {
                tool_name: "test-tool".to_string(),
                api_endpoints: vec!["http://localhost".to_string()],
                feature_set: vec!["read".to_string()],
            },
            scope: PermissionScope {
                operations: vec!["read".to_string()],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: Vec::new(),
            },
            validity_duration: std::time::Duration::from_secs(3600),
            delegation_info: None,
        };
        let perm = d.create_permission(request).await.unwrap();
        assert!(!perm.permission_id.is_nil());

        let result = d.validate_permission(&perm).await.unwrap();
        assert_eq!(result, PermissionValidationResult::Valid);
    }

    #[tokio::test]
    async fn mock_revoke_permission() {
        let d = mock_dispatch();
        let id = uuid::Uuid::new_v4();
        d.revoke_permission(&id, "test reason").await.unwrap();
    }

    #[tokio::test]
    async fn mock_health_check() {
        let d = mock_dispatch();
        let health = d.health_check().await.unwrap();
        assert_eq!(health, ProviderHealth::Healthy);
    }
}
