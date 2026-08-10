// SPDX-License-Identifier: AGPL-3.0-or-later
//! Crypto adapter - capability-based cryptographic operations
//!
//! This adapter replaces primal-specific crypto integration with a generic
//! adapter that works with any service providing crypto capabilities.
//!
//! # Migration
//! ```rust,ignore
//! // ❌ OLD: Hardcoded service module
//! use crate::ecosystem::services::legacy_crypto;
//! let verified = legacy_crypto::verify_ed25519_signature(key, msg, sig).await?;
//!
//! // ✅ NEW: Capability-based (adapters/crypto.rs)
//! use crate::ecosystem::adapters::CryptoAdapter;
//! use crate::ecosystem::capabilities::StandardCapability;
//! let verified = crypto.verify_signature(
//!     StandardCapability::CryptoSignatureEd25519,
//!     key, msg, sig
//! ).await?;
//! ```

use crate::{CliContextExt, Result};
use serde_json::json;
use std::sync::Arc;

use super::universal::{Request, UniversalServiceAdapter};
use crate::ecosystem::capabilities::{CapabilityId, StandardCapability};

/// Crypto adapter - provides cryptographic operations via capability discovery
///
/// This adapter discovers and invokes crypto services without knowing their identity.
/// The backing implementation may be a local PKI service, AWS KMS, Vault, HSM, etc.
pub struct CryptoAdapter {
    /// Universal service adapter for invoking capabilities
    universal: Arc<UniversalServiceAdapter>,
}

impl CryptoAdapter {
    /// Create a new crypto adapter
    pub const fn new(universal: Arc<UniversalServiceAdapter>) -> Self {
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
            .ok_or_else(|| crate::CliError::Other("Invalid response format".to_string()))?;

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
            .ok_or_else(|| crate::CliError::Other("Missing public_key in response".to_string()))?;

        let private_key = data
            .get("private_key")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| crate::CliError::Other("Missing private_key in response".to_string()))?;

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
            .ok_or_else(|| crate::CliError::Other("Missing ciphertext in response".to_string()))?;

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
            .ok_or_else(|| crate::CliError::Other("Missing plaintext in response".to_string()))?;

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
            .ok_or_else(|| crate::CliError::Other("Missing bytes in response".to_string()))?;

        Ok(bytes)
    }

    /// Install cryptographic permissions from a file
    ///
    /// This method replaces legacy hardcoded permission installers with a
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
        use std::fs;
        use tracing::info;

        info!(
            "🔐 Installing crypto permissions from: {}",
            permissions_path.display()
        );

        // Read permissions file
        let permissions_data = fs::read_to_string(permissions_path).context(format!(
            "Failed to read permissions file: {}",
            permissions_path.display()
        ))?;

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
            .ok_or_else(|| {
                crate::CliError::Other("Invalid validation response format".to_string())
            })?;

        if !is_valid {
            let reason = validate_data
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown validation error");
            return Err(crate::CliError::Other(format!(
                "Permission validation failed: {reason}"
            )));
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
    /// Public key bytes (e.g. 32 for Ed25519)
    pub public_key: Vec<u8>,
    /// Private key bytes (keep secret)
    pub private_key: Vec<u8>,
}

mod base64 {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, ::base64::DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
#[path = "crypto_tests.rs"]
mod crypto_tests;
