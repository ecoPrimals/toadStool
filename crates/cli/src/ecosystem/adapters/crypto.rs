//! Crypto adapter - capability-based cryptographic operations
//!
//! This adapter replaces the hardcoded BearDog integration with a generic
//! crypto service adapter that works with ANY service providing crypto capabilities.
//!
//! # Migration from BearDog
//! ```rust,ignore
//! // ❌ OLD: Hardcoded BearDog (services/beardog.rs)
//! use crate::ecosystem::services::beardog;
//! let verified = beardog::verify_ed25519_signature(key, msg, sig).await?;
//!
//! // ✅ NEW: Capability-based (adapters/crypto.rs)
//! use crate::ecosystem::adapters::CryptoAdapter;
//! use crate::ecosystem::capabilities::StandardCapability;
//! let verified = crypto.verify_signature(
//!     StandardCapability::CryptoSignatureEd25519,
//!     key, msg, sig
//! ).await?;
//! ```

use anyhow::{Context, Result};
use serde_json::json;
use std::sync::Arc;

use super::universal::{Request, UniversalServiceAdapter};
use crate::ecosystem::capabilities::{CapabilityId, StandardCapability};

/// Crypto adapter - provides cryptographic operations via capability discovery
///
/// This adapter discovers and invokes crypto services without knowing their identity.
/// Services could be BearDog, AWS KMS, HashiCorp Vault, hardware HSM, or custom implementations.
pub struct CryptoAdapter {
    /// Universal service adapter for invoking capabilities
    universal: Arc<UniversalServiceAdapter>,
}

impl CryptoAdapter {
    /// Create a new crypto adapter
    pub fn new(universal: Arc<UniversalServiceAdapter>) -> Self {
        Self { universal }
    }

    /// Verify a digital signature
    ///
    /// Discovers a service providing the specified signature algorithm
    /// and verifies the signature.
    ///
    /// # Arguments
    /// * `algorithm` - Signature algorithm (e.g., Ed25519, ECDSA, RSA)
    /// * `public_key` - Public key bytes
    /// * `message` - Message that was signed
    /// * `signature` - Signature to verify
    ///
    /// # Example
    /// ```ignore
    /// // Forward-looking example - API under development
    /// use toadstool_cli::ecosystem::adapters::CryptoAdapter;
    /// use toadstool_cli::ecosystem::capabilities::StandardCapability;
    ///
    /// # async fn example(crypto: CryptoAdapter, public_key: &[u8], message: &[u8], signature: &[u8]) -> anyhow::Result<()> {
    /// let verified = crypto.verify_signature(
    ///     StandardCapability::CryptoSignatureEd25519,
    ///     public_key,
    ///     message,
    ///     signature,
    /// ).await?;
    ///
    /// println!("Signature valid: {}", verified);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn verify_signature(
        &self,
        algorithm: impl Into<CapabilityId>,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        let capability = algorithm.into();

        let request = Request::new(
            "verify",
            json!({
                "public_key": base64::encode(public_key),
                "message": base64::encode(message),
                "signature": base64::encode(signature),
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to verify signature")?;

        let data = response.data()?;
        let verified = data
            .get("verified")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

        Ok(verified)
    }

    /// Generate a cryptographic key pair
    ///
    /// # Example
    /// ```ignore
    /// # async fn example(crypto: CryptoAdapter) -> anyhow::Result<()> {
    /// use toadstool_cli::ecosystem::capabilities::StandardCapability;
    ///
    /// let keypair = crypto.generate_keypair(
    ///     StandardCapability::CryptoSignatureEd25519
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_keypair(&self, algorithm: impl Into<CapabilityId>) -> Result<KeyPair> {
        let capability = algorithm.into();

        // Use key generation capability
        let key_gen_capability = StandardCapability::CryptoKeyGeneration.id();

        let request = Request::new(
            "generate",
            json!({
                "algorithm": capability.as_str(),
            }),
        );

        let response = self
            .universal
            .invoke(key_gen_capability, request)
            .await
            .context("Failed to generate keypair")?;

        let data = response.data()?;

        let public_key = data
            .get("public_key")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Missing public_key in response"))?;

        let private_key = data
            .get("private_key")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Missing private_key in response"))?;

        Ok(KeyPair {
            public_key,
            private_key,
        })
    }

    /// Encrypt data
    pub async fn encrypt(
        &self,
        algorithm: impl Into<CapabilityId>,
        key: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>> {
        let capability = algorithm.into();

        let request = Request::new(
            "encrypt",
            json!({
                "key": base64::encode(key),
                "plaintext": base64::encode(plaintext),
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to encrypt data")?;

        let data = response.data()?;
        let ciphertext = data
            .get("ciphertext")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Missing ciphertext in response"))?;

        Ok(ciphertext)
    }

    /// Decrypt data
    pub async fn decrypt(
        &self,
        algorithm: impl Into<CapabilityId>,
        key: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>> {
        let capability = algorithm.into();

        let request = Request::new(
            "decrypt",
            json!({
                "key": base64::encode(key),
                "ciphertext": base64::encode(ciphertext),
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to decrypt data")?;

        let data = response.data()?;
        let plaintext = data
            .get("plaintext")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Missing plaintext in response"))?;

        Ok(plaintext)
    }

    /// Generate secure random bytes
    pub async fn random_bytes(&self, length: usize) -> Result<Vec<u8>> {
        let capability = StandardCapability::CryptoRandom.id();

        let request = Request::new(
            "generate",
            json!({
                "length": length,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to generate random bytes")?;

        let data = response.data()?;
        let bytes = data
            .get("bytes")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("Missing bytes in response"))?;

        Ok(bytes)
    }

    /// Install cryptographic permissions from a file
    ///
    /// This method replaces the hardcoded `beardog::install_permissions()` with a
    /// capability-based approach. It reads a permissions file, validates the permissions,
    /// and optionally installs them using any crypto service that provides permission
    /// management capabilities.
    ///
    /// # Arguments
    /// * `permissions_path` - Path to the permissions JSON file
    /// * `validate_only` - If true, only validate without installing
    ///
    /// # Returns
    /// * `Ok(())` - Permissions validated and optionally installed
    /// * `Err(_)` - Permission validation or installation failed
    ///
    /// # Example
    /// ```ignore
    /// use std::path::PathBuf;
    /// use toadstool_cli::ecosystem::adapters::CryptoAdapter;
    ///
    /// # async fn example(crypto: CryptoAdapter) -> anyhow::Result<()> {
    /// let path = PathBuf::from("/path/to/permissions.json");
    ///
    /// // Validate and install
    /// crypto.install_permissions(&path, false).await?;
    ///
    /// // Or just validate
    /// crypto.install_permissions(&path, true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn install_permissions(
        &self,
        permissions_path: &std::path::Path,
        validate_only: bool,
    ) -> Result<()> {
        use tokio::fs;
        use tracing::info;

        info!(
            "🔐 Installing crypto permissions from: {}",
            permissions_path.display()
        );

        // Read permissions file
        let permissions_data = fs::read_to_string(permissions_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to read permissions file: {}",
                    permissions_path.display()
                )
            })?;

        // Parse permissions as generic JSON (service-agnostic)
        let permissions: serde_json::Value =
            serde_json::from_str(&permissions_data).context("Failed to parse permissions file")?;

        // Use permission validation capability
        let capability = StandardCapability::CryptoPermissionManagement.id();

        // Validate permissions via capability system
        let validate_request = Request::new(
            "validate",
            json!({
                "permissions": permissions,
            }),
        );

        let validate_response = self
            .universal
            .invoke(capability.clone(), validate_request)
            .await
            .context("Failed to validate permissions")?;

        let validate_data = validate_response.data()?;
        let is_valid = validate_data
            .get("valid")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!("Invalid validation response format"))?;

        if !is_valid {
            let reason = validate_data
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown validation error");
            anyhow::bail!("Permission validation failed: {}", reason);
        }

        info!("✅ Crypto permission validation successful");

        // If validate_only, stop here
        if validate_only {
            info!("🔍 Validation-only mode - not installing");
            return Ok(());
        }

        // Install permissions via capability system
        let install_request = Request::new(
            "install",
            json!({
                "permissions": permissions,
            }),
        );

        self.universal
            .invoke(capability, install_request)
            .await
            .context("Failed to install permissions")?;

        info!("✅ Crypto permissions installed successfully");
        Ok(())
    }
}

/// Cryptographic key pair
#[derive(Debug, Clone)]
pub struct KeyPair {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

// Re-export base64 for convenience using base64 0.21 API
mod base64 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, ::base64::DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_roundtrip() {
        let data = b"hello world";
        let encoded = base64::encode(data);
        let decoded = base64::decode(&encoded).unwrap();
        assert_eq!(data, &decoded[..]);
    }
}
