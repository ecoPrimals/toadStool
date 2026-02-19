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
        self.symmetric.get(key_id).map(|v| v.as_slice())
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
                ToadStoolError::not_found(format!("Key '{}' not found", metadata.key_id))
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
        }
        Ok(())
    }

    async fn health_check(&self) -> ToadStoolResult<ProviderHealth> {
        Ok(ProviderHealth::Healthy)
    }
}
