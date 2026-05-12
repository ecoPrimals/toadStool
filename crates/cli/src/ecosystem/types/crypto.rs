// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security permissions, signed responses, and verification context.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tracing::{error, warn};
use uuid::Uuid;

use crate::Result;

/// Crypto / security permission grant (capability-based)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPermission {
    /// Unique permission ID
    pub permission_id: Uuid,
    /// Identity the permission is granted to
    pub granted_to: String,
    /// Allowed capability names
    pub capabilities: Vec<String>,
    /// Expiration time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub valid_until: std::time::SystemTime,
    /// Cryptographic signature
    pub signature: String,
}

/// Compatibility alias for [`SecurityPermission`] (prefer the canonical name in new code).
pub type BearDogPermission = SecurityPermission;

/// Cryptographic signature on a service response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSignature {
    /// Algorithm (e.g. ed25519)
    pub algorithm: String,
    /// Base64-encoded signature
    pub signature: String,
    /// Signer's public key
    pub public_key: String,
    /// When the signature was created
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Nonce for replay protection
    pub nonce: String,
}

/// Service response with cryptographic signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedServiceResponse {
    /// Service instance ID
    pub service_id: String,
    /// Capability type
    pub service_type: String,
    /// Status string (e.g. ok, busy)
    pub status: String,
    /// Capability names
    pub capabilities: Vec<String>,
    /// Response timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Signature for verification
    pub signature: ServiceSignature,
}

/// Context for verifying service signatures (trusted keys, revocation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoVerificationContext {
    /// Map of service type to trusted public key (base64)
    pub trusted_public_keys: HashMap<String, String>,
    /// Revoked public keys (base64)
    pub revoked_keys: Vec<String>,
    /// When verification context was built
    #[serde(with = "toadstool_common::system_time_serde")]
    pub verification_timestamp: std::time::SystemTime,
    /// Max age of responses to accept (minutes)
    pub max_age_minutes: u32,
}

impl Default for CryptoVerificationContext {
    fn default() -> Self {
        use toadstool_common::interned_strings::capabilities;

        let mut trusted_keys = HashMap::new();

        // Capability-based env vars first, legacy env names for backward compat
        if let Ok(key) = std::env::var("CRYPTO_PROVIDER_PUBLIC_KEY") {
            trusted_keys.insert(capabilities::CRYPTO.to_string(), key);
        } else if let Ok(key) = std::env::var("BEARDOG_PUBLIC_KEY") {
            warn!("BEARDOG_PUBLIC_KEY is deprecated — use CRYPTO_PROVIDER_PUBLIC_KEY");
            trusted_keys.insert(capabilities::CRYPTO.to_string(), key);
        }
        if let Ok(key) = std::env::var("STORAGE_PROVIDER_PUBLIC_KEY") {
            trusted_keys.insert(capabilities::STORAGE.to_string(), key);
        } else if let Ok(key) = std::env::var("NESTGATE_PUBLIC_KEY") {
            warn!("NESTGATE_PUBLIC_KEY is deprecated — use STORAGE_PROVIDER_PUBLIC_KEY");
            trusted_keys.insert(capabilities::STORAGE.to_string(), key);
        }
        if let Ok(key) = std::env::var("DISCOVERY_PROVIDER_PUBLIC_KEY") {
            trusted_keys.insert(capabilities::COORDINATION.to_string(), key);
        } else if let Ok(key) = std::env::var("SONGBIRD_PUBLIC_KEY") {
            warn!("SONGBIRD_PUBLIC_KEY is deprecated — use DISCOVERY_PROVIDER_PUBLIC_KEY");
            trusted_keys.insert(capabilities::COORDINATION.to_string(), key);
        }

        Self {
            trusted_public_keys: trusted_keys,
            revoked_keys: Vec::new(),
            verification_timestamp: std::time::SystemTime::now(),
            max_age_minutes: 5,
        }
    }
}

impl CryptoVerificationContext {
    /// Create a new verification context (loads trusted keys from env)
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Zero-Copy Optimization: Takes `&str` to avoid allocations.
    #[must_use]
    pub fn with_trusted_key(mut self, service: &str, public_key: &str) -> Self {
        self.trusted_public_keys
            .insert(service.to_string(), public_key.to_string());
        self
    }

    /// Verify an Ed25519 signature over a message.
    ///
    /// Delegates to the crypto / security provider via `crypto.verify` JSON-RPC
    /// over the discovered Unix socket. Returns `Ok(false)` when no crypto
    /// provider is available.
    pub fn verify_ed25519_signature(
        &self,
        message: &[u8],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
    ) -> Result<bool> {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("crypto");

        if !socket_path.exists() {
            tracing::debug!(
                "No crypto capability socket at {}, signature verification unavailable",
                socket_path.display()
            );
            return Ok(false);
        }

        let client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        let params = serde_json::json!({
            "data": message,
            "signature": signature_bytes,
            "public_key": BASE64.encode(public_key_bytes),
        });

        let result: std::result::Result<serde_json::Value, _> = std::thread::scope(|s| {
            s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| {
                        crate::CliError::Other(format!("Failed to create runtime: {e}"))
                    })?;
                rt.block_on(client.call("crypto.verify", params))
                    .map_err(|e| crate::CliError::Other(format!("crypto.verify failed: {e}")))
            })
            .join()
            .unwrap_or_else(|_| Err(crate::CliError::Other("verify thread panicked".into())))
        });

        match result {
            Ok(value) => Ok(value
                .get("valid")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)),
            Err(e) => {
                tracing::debug!("crypto.verify RPC failed: {e}, returning false");
                Ok(false)
            }
        }
    }

    /// Verify a signed service response using trusted keys
    pub fn verify_service_signature(
        &self,
        service_type: &str,
        response: &SignedServiceResponse,
    ) -> Result<bool> {
        // Check if we have a trusted public key for this service
        let Some(public_key_base64) = self.trusted_public_keys.get(service_type) else {
            warn!("No trusted public key found for service: {}", service_type);
            return Ok(false);
        };

        // Check if key is revoked
        if self.revoked_keys.contains(public_key_base64) {
            error!(
                "Attempted to use revoked public key for service: {}",
                service_type
            );
            return Ok(false);
        }

        // Check response age
        let response_age = std::time::SystemTime::now()
            .duration_since(response.timestamp)
            .map(|d| d.as_secs() / 60)
            .unwrap_or(0);

        if response_age > u64::from(self.max_age_minutes) {
            warn!("Service response is too old: {} minutes", response_age);
            return Ok(false);
        }

        // Decode public key and signature
        let public_key_bytes = BASE64
            .decode(public_key_base64)
            .map_err(|e| crate::CliError::Other(format!("Failed to decode public key: {e}")))?;

        let signature_bytes = BASE64
            .decode(&response.signature.signature)
            .map_err(|e| crate::CliError::Other(format!("Failed to decode signature: {e}")))?;

        // Create canonical message for verification
        let message = self.create_canonical_message(response)?;

        // Verify signature
        self.verify_ed25519_signature(&message, &signature_bytes, &public_key_bytes)
    }

    fn create_canonical_message(&self, response: &SignedServiceResponse) -> Result<Vec<u8>> {
        // Create deterministic message representation for signature verification
        let mut data = BTreeMap::new();
        data.insert("service_id", &response.service_id);
        data.insert("service_type", &response.service_type);
        data.insert("status", &response.status);
        let timestamp = toadstool_common::system_time_serde::format_rfc3339(response.timestamp);
        data.insert("timestamp", &timestamp);
        data.insert("nonce", &response.signature.nonce);

        let capabilities_json = serde_json::to_string(&response.capabilities).map_err(|e| {
            crate::CliError::Other(format!("Failed to serialize capabilities: {e}"))
        })?;
        data.insert("capabilities", &capabilities_json);

        let canonical_json = serde_json::to_string(&data).map_err(|e| {
            crate::CliError::Other(format!("Failed to create canonical message: {e}"))
        })?;

        Ok(canonical_json.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_ed25519_returns_false_without_crypto_provider() {
        let context = CryptoVerificationContext::new();
        let result = context.verify_ed25519_signature(b"hello", &[0u8; 64], &[0u8; 32]);
        assert!(
            !result.unwrap(),
            "must return false when no crypto provider is reachable"
        );
    }

    #[test]
    fn test_verify_ed25519_accepts_any_input_shapes() {
        let context = CryptoVerificationContext::new();
        let short_key = context.verify_ed25519_signature(b"msg", &[0u8; 64], &[0u8; 16]);
        let short_sig = context.verify_ed25519_signature(b"msg", &[0u8; 32], &[0u8; 32]);
        assert!(
            short_key.is_ok(),
            "length validation is the security provider's responsibility"
        );
        assert!(
            short_sig.is_ok(),
            "length validation is the security provider's responsibility"
        );
    }

    #[test]
    fn test_verify_service_signature_no_trusted_key() {
        let context = CryptoVerificationContext::new();
        let response = SignedServiceResponse {
            service_id: "svc-1".to_string(),
            service_type: "unknown".to_string(),
            status: "ok".to_string(),
            capabilities: vec![],
            timestamp: std::time::SystemTime::now(),
            signature: ServiceSignature {
                algorithm: "ed25519".to_string(),
                signature: "sig".to_string(),
                public_key: "key".to_string(),
                timestamp: std::time::SystemTime::now(),
                nonce: "nonce".to_string(),
            },
        };
        let result = context.verify_service_signature("unknown", &response);
        assert!(!result.unwrap());
    }

    #[test]
    fn test_verify_service_signature_revoked_key() {
        let mut ctx = CryptoVerificationContext::new().with_trusted_key("test", "dGVzdA==");
        ctx.revoked_keys.push("dGVzdA==".to_string());

        let response = SignedServiceResponse {
            service_id: "svc-1".to_string(),
            service_type: "test".to_string(),
            status: "ok".to_string(),
            capabilities: vec![],
            timestamp: std::time::SystemTime::now(),
            signature: ServiceSignature {
                algorithm: "ed25519".to_string(),
                signature: "sig".to_string(),
                public_key: "key".to_string(),
                timestamp: std::time::SystemTime::now(),
                nonce: "nonce".to_string(),
            },
        };
        let result = ctx.verify_service_signature("test", &response);
        assert!(!result.unwrap());
    }
}
