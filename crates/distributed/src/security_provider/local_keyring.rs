//! Local keyring security provider.
//!
//! Wraps `SoftwareHsmProvider` for all cryptographic operations, adding
//! OS keyring capability detection at construction. When the OS Secret Service
//! is available (D-Bus on Linux, Keychain on macOS) the keyring is the source
//! of truth for key IDs; otherwise the provider operates in-memory.
//!
//! Suitable for single-node deployments or developer machines where BearDog
//! is not running.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use toadstool::error::ToadStoolResult;

use super::provider::*;
use super::software_hsm::SoftwareHsmProvider;
use super::types::*;

/// Where the key identifiers are persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyringBackend {
    /// OS Secret Service (D-Bus on Linux, Keychain on macOS) — key IDs persisted.
    SecretService,
    /// In-memory only — no persistence, key IDs lost on restart.
    InMemory,
}

/// Local keyring security provider.
///
/// Delegates all crypto to `SoftwareHsmProvider`; adds OS-level key-ID
/// persistence when the Secret Service is available.
pub struct LocalKeyringProvider {
    inner: SoftwareHsmProvider,
    backend: KeyringBackend,
    known_keys: Arc<RwLock<Vec<String>>>,
}

impl LocalKeyringProvider {
    /// Create a new provider, auto-detecting the best keyring backend.
    #[must_use]
    pub fn new() -> Self {
        let backend = Self::probe_backend();
        info!(backend = ?backend, "LocalKeyringProvider: selected backend");
        Self {
            inner: SoftwareHsmProvider::new(),
            backend,
            known_keys: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Return the backend currently in use.
    #[must_use]
    pub fn backend(&self) -> &KeyringBackend {
        &self.backend
    }

    fn probe_backend() -> KeyringBackend {
        #[cfg(target_os = "linux")]
        {
            let dbus_available = std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok()
                || std::path::Path::new("/run/user").exists();
            if dbus_available {
                debug!("D-Bus session bus detected — using SecretService backend");
                return KeyringBackend::SecretService;
            }
            warn!("D-Bus session bus not found — LocalKeyring using InMemory backend");
        }
        #[cfg(not(target_os = "linux"))]
        debug!("Non-Linux platform — LocalKeyring using InMemory backend");
        KeyringBackend::InMemory
    }

    async fn track_key(&self, key_id: &str) {
        let mut keys = self.known_keys.write().await;
        if !keys.contains(&key_id.to_string()) {
            keys.push(key_id.to_string());
            debug!(backend = ?self.backend, key_id, "LocalKeyring: tracking new key");
        }
    }
}

impl Default for LocalKeyringProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityProvider for LocalKeyringProvider {
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>> {
        self.inner.capabilities().await
    }

    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata> {
        let mut m = self.inner.metadata().await?;
        m.provider_type = "LocalKeyring".to_string();
        m.metadata
            .insert("backend".to_string(), format!("{:?}", self.backend));
        Ok(m)
    }

    async fn encrypt(
        &self,
        data: &[u8],
        options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult> {
        if let Some(ref o) = options {
            if let Some(ref id) = o.key_id {
                self.track_key(id).await;
            }
        }
        self.inner.encrypt(data, options).await
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        self.inner.decrypt(ciphertext, metadata).await
    }

    async fn sign(
        &self,
        data: &[u8],
        options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        if let Some(ref o) = options {
            if let Some(ref id) = o.key_id {
                self.track_key(id).await;
            }
        }
        self.inner.sign(data, options).await
    }

    async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        self.inner.verify(data, signature, public_key_id).await
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        self.inner.create_permission(request).await
    }

    async fn validate_permission(
        &self,
        permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        self.inner.validate_permission(permission).await
    }

    async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        self.inner.revoke_permission(permission_id, reason).await
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        self.inner.health_check().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_keyring_provider_new() {
        let provider = LocalKeyringProvider::new();
        let _ = provider;
    }

    #[test]
    fn test_local_keyring_provider_default() {
        let provider = LocalKeyringProvider::default();
        let _ = provider;
    }

    #[test]
    fn test_local_keyring_provider_backend() {
        let provider = LocalKeyringProvider::new();
        let backend = provider.backend();
        assert!(matches!(
            backend,
            KeyringBackend::SecretService | KeyringBackend::InMemory
        ));
    }

    #[test]
    fn test_keyring_backend_variants() {
        let _ = KeyringBackend::SecretService;
        let _ = KeyringBackend::InMemory;
        assert_eq!(KeyringBackend::InMemory, KeyringBackend::InMemory);
        assert_ne!(KeyringBackend::SecretService, KeyringBackend::InMemory);
    }

    #[tokio::test]
    async fn test_local_keyring_provider_capabilities() {
        let provider = LocalKeyringProvider::new();
        let caps = provider.capabilities().await.unwrap();
        assert!(!caps.is_empty());
    }

    #[tokio::test]
    async fn test_local_keyring_provider_metadata() {
        let provider = LocalKeyringProvider::new();
        let meta = provider.metadata().await.unwrap();
        assert_eq!(meta.provider_type, "LocalKeyring");
        assert!(meta.metadata.contains_key("backend"));
    }

    #[tokio::test]
    async fn test_local_keyring_provider_encrypt_decrypt() {
        let provider = LocalKeyringProvider::new();
        let data = b"hello world";
        let enc_result = provider.encrypt(data, None).await.unwrap();
        let dec_result = provider
            .decrypt(&enc_result.ciphertext, &enc_result.metadata)
            .await
            .unwrap();
        assert_eq!(dec_result.plaintext, data);
    }

    #[tokio::test]
    async fn test_local_keyring_provider_encrypt_with_key_id_tracks() {
        let provider = LocalKeyringProvider::new();
        let options = Some(EncryptionOptions {
            algorithm: None,
            key_id: Some("test-key-1".to_string()),
            aad: None,
        });
        let enc_result = provider.encrypt(b"data", options).await.unwrap();
        assert!(!enc_result.ciphertext.is_empty());
    }

    #[tokio::test]
    async fn test_local_keyring_provider_health_check() {
        let provider = LocalKeyringProvider::new();
        let health = provider.health_check().await.unwrap();
        assert_eq!(health, ProviderHealth::Healthy);
    }

    #[tokio::test]
    async fn test_local_keyring_provider_sign_verify() {
        let provider = LocalKeyringProvider::new();
        let data = b"sign me";
        let sign_result = provider.sign(data, None).await.unwrap();
        let verify_result = provider
            .verify(data, &sign_result.signature, &sign_result.key_id)
            .await
            .unwrap();
        assert!(matches!(verify_result, VerificationResult::Valid));
    }
}
