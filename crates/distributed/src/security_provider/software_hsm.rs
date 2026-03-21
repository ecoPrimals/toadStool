// SPDX-License-Identifier: AGPL-3.0-only
//! Software HSM security provider.
//!
//! A pure-Rust in-process security provider using:
//! - **AES-256-GCM** for authenticated symmetric encryption (nonce prepended to ciphertext)
//! - **ed25519-dalek** for signing and verification
//! - **In-memory key store** protected by `RwLock`
//!
//! Key material is ephemeral — lost on restart. Suitable for development, CI,
//! and environments where a hardware security provider (BearDog) is unavailable.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use async_trait::async_trait;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use tokio::sync::RwLock;
use tracing::debug;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::provider::*;
use super::types::*;

/// In-memory key store for the software HSM.
#[derive(Default)]
struct KeyStore {
    /// AES-256-GCM symmetric keys (32 bytes each)
    symmetric: HashMap<String, Vec<u8>>,
    /// ed25519 signing key pairs
    signing: HashMap<String, SigningKey>,
}

impl KeyStore {
    fn get_or_create_symmetric(&mut self, key_id: &str) -> &[u8] {
        self.symmetric.entry(key_id.to_string()).or_insert_with(|| {
            let mut key = vec![0u8; 32];
            OsRng.fill_bytes(&mut key);
            debug!(key_id, "SoftwareHSM: generated new AES-256-GCM key");
            key
        })
    }

    fn get_or_create_signing(&mut self, key_id: &str) -> &SigningKey {
        self.signing.entry(key_id.to_string()).or_insert_with(|| {
            let signing_key = SigningKey::generate(&mut OsRng);
            debug!(key_id, "SoftwareHSM: generated new ed25519 key pair");
            signing_key
        })
    }

    fn get_symmetric(&self, key_id: &str) -> Option<&[u8]> {
        self.symmetric.get(key_id).map(Vec::as_slice)
    }

    fn get_signing(&self, key_id: &str) -> Option<&SigningKey> {
        self.signing.get(key_id)
    }
}

/// Software HSM security provider — in-process, ephemeral keys.
pub struct SoftwareHsmProvider {
    keys: Arc<RwLock<KeyStore>>,
    /// Revoked permission IDs — stored in-memory only.
    revoked: Arc<RwLock<Vec<uuid::Uuid>>>,
}

impl SoftwareHsmProvider {
    /// Create a new software HSM provider with a fresh key store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(KeyStore::default())),
            revoked: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for SoftwareHsmProvider {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl SecurityProvider for SoftwareHsmProvider {
    async fn capabilities(&self) -> ToadStoolResult<Vec<SecurityCapability>> {
        Ok(vec![
            SecurityCapability::SymmetricEncryption,
            SecurityCapability::DigitalSignatures,
            SecurityCapability::KeyManagement,
            SecurityCapability::PermissionIssuance,
        ])
    }

    async fn metadata(&self) -> ToadStoolResult<ProviderMetadata> {
        Ok(ProviderMetadata {
            provider_id: "software-hsm-local".to_string(),
            provider_type: "SoftwareHSM".to_string(),
            provider_version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("algorithm_sym".to_string(), "AES-256-GCM".to_string());
                m.insert("algorithm_sign".to_string(), "ed25519".to_string());
                m.insert("key_persistence".to_string(), "ephemeral".to_string());
                m
            },
        })
    }

    async fn encrypt(
        &self,
        data: &[u8],
        options: Option<EncryptionOptions>,
    ) -> ToadStoolResult<EncryptionResult> {
        let key_id = options
            .and_then(|o| o.key_id)
            .unwrap_or_else(|| "default".to_string());

        let mut store = self.keys.write().await;
        let raw_key = store.get_or_create_symmetric(&key_id).to_vec();
        drop(store);

        let key = Key::<Aes256Gcm>::from_slice(&raw_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // AES-GCM output includes the 16-byte auth tag appended to ciphertext.
        let ct_with_tag = cipher
            .encrypt(&nonce, data)
            .map_err(|e| ToadStoolError::security(format!("AES-256-GCM encrypt: {e}")))?;

        // Canonical wire format: nonce (12 B) || ciphertext || tag (16 B)
        let mut wire = nonce.to_vec();
        wire.extend_from_slice(&ct_with_tag);

        let tag_start = ct_with_tag.len().saturating_sub(16);
        let tag = &ct_with_tag[tag_start..];

        Ok(EncryptionResult {
            ciphertext: wire, // full wire payload (nonce + ct + tag)
            iv: Some(nonce.to_vec()),
            auth_tag: Some(tag.to_vec()),
            metadata: EncryptionMetadata {
                algorithm: "AES-256-GCM".to_string(),
                key_id,
                encrypted_at: SystemTime::now(),
            },
        })
    }

    async fn decrypt(
        &self,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> ToadStoolResult<DecryptionResult> {
        // Expect wire format: nonce (12 B) || ciphertext+tag
        if ciphertext.len() < 12 + 16 {
            return Err(ToadStoolError::security("Ciphertext too short".to_string()));
        }
        let (nonce_bytes, ct_and_tag) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let store = self.keys.read().await;
        let raw_key = store
            .get_symmetric(&metadata.key_id)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!(
                    "Key '{key_id}' not found",
                    key_id = metadata.key_id
                ))
            })?
            .to_vec();
        drop(store);

        let key = Key::<Aes256Gcm>::from_slice(&raw_key);
        let cipher = Aes256Gcm::new(key);

        let plaintext = cipher
            .decrypt(nonce, ct_and_tag)
            .map_err(|e| ToadStoolError::security(format!("AES-256-GCM decrypt: {e}")))?;

        Ok(DecryptionResult {
            plaintext,
            metadata: DecryptionMetadata {
                key_id: metadata.key_id.clone(),
                decrypted_at: SystemTime::now(),
            },
        })
    }

    async fn sign(
        &self,
        data: &[u8],
        options: Option<SigningOptions>,
    ) -> ToadStoolResult<SignatureResult> {
        let key_id = options
            .and_then(|o| o.key_id)
            .unwrap_or_else(|| "default".to_string());

        let mut store = self.keys.write().await;
        let signing_key = store.get_or_create_signing(&key_id);
        let signature: Signature = signing_key.sign(data);
        let key_id_clone = key_id.clone();
        drop(store);

        Ok(SignatureResult {
            signature: signature.to_bytes().to_vec(),
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: key_id_clone,
            signed_at: SystemTime::now(),
        })
    }

    async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<VerificationResult> {
        let store = self.keys.read().await;
        let signing_key = store
            .get_signing(public_key_id)
            .ok_or_else(|| ToadStoolError::not_found(format!("Key '{public_key_id}' not found")))?;
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        drop(store);

        let sig_bytes: [u8; 64] = signature.try_into().map_err(|_| {
            ToadStoolError::security("Invalid ed25519 signature length".to_string())
        })?;
        let sig = Signature::from_bytes(&sig_bytes);

        match verifying_key.verify(data, &sig) {
            Ok(()) => Ok(VerificationResult::Valid),
            Err(_) => Ok(VerificationResult::Invalid),
        }
    }

    async fn create_permission(
        &self,
        request: PermissionRequest,
    ) -> ToadStoolResult<SecurityPermission> {
        let payload =
            serde_json::to_vec(&request).map_err(|e| ToadStoolError::security(e.to_string()))?;
        let sig_result = self.sign(&payload, None).await?;
        let now = SystemTime::now();
        let valid_until = now + request.validity_duration;

        Ok(SecurityPermission {
            permission_id: uuid::Uuid::new_v4(),
            holder_id: request.requester_id,
            target: request.target,
            scope: request.scope,
            valid_from: now,
            valid_until,
            proof: SecurityProof {
                signature: sig_result.signature,
                algorithm: SignatureAlgorithm::Ed25519,
                public_key_id: sig_result.key_id,
                signed_at: now,
            },
            provider_metadata: self.metadata().await?,
        })
    }

    async fn validate_permission(
        &self,
        permission: &SecurityPermission,
    ) -> ToadStoolResult<PermissionValidationResult> {
        // Check revocation list
        {
            let revoked = self.revoked.read().await;
            if revoked.contains(&permission.permission_id) {
                return Ok(PermissionValidationResult::Revoked);
            }
        }

        // Check expiry
        if SystemTime::now() > permission.valid_until {
            return Ok(PermissionValidationResult::Expired);
        }

        // Reconstruct signed payload
        let request = PermissionRequest {
            requester_id: permission.holder_id.clone(),
            target: permission.target.clone(),
            scope: permission.scope.clone(),
            validity_duration: permission
                .valid_until
                .duration_since(permission.valid_from)
                .unwrap_or_default(),
            delegation_info: None,
        };
        let payload =
            serde_json::to_vec(&request).map_err(|e| ToadStoolError::security(e.to_string()))?;

        match self
            .verify(
                &payload,
                &permission.proof.signature,
                &permission.proof.public_key_id,
            )
            .await?
        {
            VerificationResult::Valid => Ok(PermissionValidationResult::Valid),
            _ => Ok(PermissionValidationResult::InvalidSignature),
        }
    }

    async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        debug!(?permission_id, reason, "SoftwareHSM: revoking permission");
        let mut revoked = self.revoked.write().await;
        if !revoked.contains(permission_id) {
            revoked.push(*permission_id);
            drop(revoked);
        }
        Ok(())
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        Ok(ProviderHealth::Healthy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security_provider::provider::{EncryptionOptions, SigningOptions};
    use crate::security_provider::types::{
        ExternalTarget, PermissionRequest, PermissionScope, ResourceLimits,
    };
    use std::time::Duration;

    fn make_permission_request() -> PermissionRequest {
        PermissionRequest {
            requester_id: "test-requester".to_string(),
            target: ExternalTarget::ExternalTool {
                tool_name: "test-tool".to_string(),
                api_endpoints: vec!["http://localhost".to_string()],
                feature_set: vec!["read".to_string()],
            },
            scope: PermissionScope {
                operations: vec!["read".to_string()],
                resource_limits: ResourceLimits::default(),
                geo_restrictions: vec![],
            },
            validity_duration: Duration::from_secs(3600),
            delegation_info: None,
        }
    }

    #[tokio::test]
    async fn test_software_hsm_new() {
        let provider = SoftwareHsmProvider::new();
        let caps = provider.capabilities().await.unwrap();
        assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
        assert!(caps.contains(&SecurityCapability::DigitalSignatures));
    }

    #[tokio::test]
    async fn test_software_hsm_default() {
        let provider = SoftwareHsmProvider::default();
        assert!(provider.capabilities().await.is_ok());
    }

    #[tokio::test]
    async fn test_software_hsm_metadata() {
        let provider = SoftwareHsmProvider::new();
        let meta = provider.metadata().await.unwrap();
        assert_eq!(meta.provider_type, "SoftwareHSM");
        assert_eq!(meta.provider_id, "software-hsm-local");
        assert!(meta.metadata.contains_key("algorithm_sym"));
        assert_eq!(meta.metadata.get("algorithm_sym").unwrap(), "AES-256-GCM");
    }

    #[tokio::test]
    async fn test_software_hsm_encrypt_decrypt_roundtrip() {
        let provider = SoftwareHsmProvider::new();
        let data = b"secret message";

        let encrypted = provider.encrypt(data, None).await.unwrap();
        assert!(!encrypted.ciphertext.is_empty());
        assert!(encrypted.iv.is_some());
        assert!(encrypted.auth_tag.is_some());
        assert_eq!(encrypted.metadata.algorithm, "AES-256-GCM");
        assert_eq!(encrypted.metadata.key_id, "default");

        let decrypted = provider
            .decrypt(&encrypted.ciphertext, &encrypted.metadata)
            .await
            .unwrap();
        assert_eq!(decrypted.plaintext, data);
    }

    #[tokio::test]
    async fn test_software_hsm_encrypt_with_custom_key_id() {
        let provider = SoftwareHsmProvider::new();
        let data = b"custom key data";
        let options = Some(EncryptionOptions {
            algorithm: None,
            key_id: Some("my-key".to_string()),
            aad: None,
        });

        let encrypted = provider.encrypt(data, options).await.unwrap();
        assert_eq!(encrypted.metadata.key_id, "my-key");

        let decrypted = provider
            .decrypt(&encrypted.ciphertext, &encrypted.metadata)
            .await
            .unwrap();
        assert_eq!(decrypted.plaintext, data);
    }

    #[tokio::test]
    async fn test_software_hsm_decrypt_ciphertext_too_short() {
        let provider = SoftwareHsmProvider::new();
        let short_ct = vec![0u8; 10];
        let metadata = EncryptionMetadata {
            algorithm: "AES-256-GCM".to_string(),
            key_id: "default".to_string(),
            encrypted_at: SystemTime::now(),
        };

        let result = provider.decrypt(&short_ct, &metadata).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_software_hsm_decrypt_key_not_found() {
        let provider = SoftwareHsmProvider::new();
        let data = b"data";
        let encrypted = provider.encrypt(data, None).await.unwrap();

        let metadata = EncryptionMetadata {
            algorithm: "AES-256-GCM".to_string(),
            key_id: "nonexistent-key".to_string(),
            encrypted_at: SystemTime::now(),
        };

        let result = provider.decrypt(&encrypted.ciphertext, &metadata).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_software_hsm_sign_verify() {
        let provider = SoftwareHsmProvider::new();
        let data = b"data to sign";

        let sig_result = provider.sign(data, None).await.unwrap();
        assert_eq!(sig_result.algorithm, SignatureAlgorithm::Ed25519);
        assert_eq!(sig_result.key_id, "default");
        assert_eq!(sig_result.signature.len(), 64);

        let verify_result = provider
            .verify(data, &sig_result.signature, &sig_result.key_id)
            .await
            .unwrap();
        assert_eq!(verify_result, VerificationResult::Valid);
    }

    #[tokio::test]
    async fn test_software_hsm_sign_with_custom_key() {
        let provider = SoftwareHsmProvider::new();
        let options = Some(SigningOptions {
            algorithm: None,
            key_id: Some("sign-key".to_string()),
        });

        let sig = provider.sign(b"data", options).await.unwrap();
        assert_eq!(sig.key_id, "sign-key");
    }

    #[tokio::test]
    async fn test_software_hsm_verify_invalid_signature_length() {
        let provider = SoftwareHsmProvider::new();
        let short_sig = vec![0u8; 32];

        let result = provider.verify(b"data", &short_sig, "default").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_software_hsm_verify_invalid_signature() {
        let provider = SoftwareHsmProvider::new();
        let _ = provider.sign(b"data", None).await.unwrap();
        let wrong_sig = vec![0u8; 64];

        let result = provider
            .verify(b"data", &wrong_sig, "default")
            .await
            .unwrap();
        assert_eq!(result, VerificationResult::Invalid);
    }

    #[tokio::test]
    async fn test_software_hsm_verify_key_not_found() {
        let provider = SoftwareHsmProvider::new();
        let sig = vec![0u8; 64];

        let result = provider.verify(b"data", &sig, "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_software_hsm_create_permission() {
        let provider = SoftwareHsmProvider::new();
        let request = make_permission_request();

        let permission = provider.create_permission(request).await.unwrap();
        assert!(!permission.proof.signature.is_empty());
        assert_eq!(permission.holder_id, "test-requester");
        assert_eq!(permission.proof.algorithm, SignatureAlgorithm::Ed25519);
    }

    #[tokio::test]
    async fn test_software_hsm_validate_permission() {
        let provider = SoftwareHsmProvider::new();
        let request = make_permission_request();
        let permission = provider.create_permission(request).await.unwrap();

        let result = provider.validate_permission(&permission).await.unwrap();
        assert_eq!(result, PermissionValidationResult::Valid);
    }

    #[tokio::test]
    async fn test_software_hsm_validate_revoked_permission() {
        let provider = SoftwareHsmProvider::new();
        let request = make_permission_request();
        let permission = provider.create_permission(request).await.unwrap();

        provider
            .revoke_permission(&permission.permission_id, "test")
            .await
            .unwrap();

        let result = provider.validate_permission(&permission).await.unwrap();
        assert_eq!(result, PermissionValidationResult::Revoked);
    }

    #[tokio::test]
    async fn test_software_hsm_revoke_permission() {
        let provider = SoftwareHsmProvider::new();
        let perm_id = uuid::Uuid::new_v4();

        let result = provider.revoke_permission(&perm_id, "reason").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_software_hsm_health_check() {
        let provider = SoftwareHsmProvider::new();
        let health = provider.health_check().await.unwrap();
        assert_eq!(health, ProviderHealth::Healthy);
    }
}
