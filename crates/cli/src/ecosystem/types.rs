// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ecosystem Integration - Type Definitions
//!
//! # Zero-Copy Optimization (Phase 2.3)
//! Service discovery types use `Arc<str>` for frequently-cloned string fields.

use crate::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, warn};
use uuid::Uuid;

/// Ecosystem service discovery and integration (main struct)
pub struct EcosystemIntegrator {
    /// Known ecosystem endpoints
    pub(super) endpoints: HashMap<String, ServiceEndpoint>,
    /// Active connections
    pub(super) connections: HashMap<String, ServiceConnection>,
    /// Authentication credentials
    pub(super) credentials: Option<EcosystemCredentials>,
}

#[derive(Debug, Clone)]
#[allow(deprecated)] // ServiceEndpoint still uses EcosystemService for backward compatibility
pub struct ServiceEndpoint {
    pub service_type: EcosystemService,
    pub address: SocketAddr,
    /// **Zero-Copy**: Uses `Arc<str>` for cheap clones in registry lookups
    pub version: Arc<str>,
    pub capabilities: Vec<String>,
    pub trust_level: TrustLevel,
}

// Custom Serialize implementation for ServiceEndpoint
impl Serialize for ServiceEndpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ServiceEndpoint", 5)?;
        state.serialize_field("service_type", &self.service_type)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("version", self.version.as_ref())?;
        state.serialize_field("capabilities", &self.capabilities)?;
        state.serialize_field("trust_level", &self.trust_level)?;
        state.end()
    }
}

// Custom Deserialize implementation for ServiceEndpoint
impl<'de> Deserialize<'de> for ServiceEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[allow(deprecated)] // EcosystemService is deprecated but still used for backward compat
        struct ServiceEndpointHelper {
            service_type: EcosystemService,
            address: SocketAddr,
            version: String,
            capabilities: Vec<String>,
            trust_level: TrustLevel,
        }

        let helper = ServiceEndpointHelper::deserialize(deserializer)?;
        Ok(ServiceEndpoint {
            service_type: helper.service_type,
            address: helper.address,
            version: Arc::from(helper.version.as_str()),
            capabilities: helper.capabilities,
            trust_level: helper.trust_level,
        })
    }
}

/// ⚠️ DEPRECATED: Hardcoded service names enum
///
/// **Use `ServiceType` instead.**
///
/// This enum hardcodes service names, violating the infant discovery principle.
/// Services should be identified by capabilities, not by hardcoded names.
///
/// # Migration Guide
/// ```rust,ignore
/// // ❌ OLD: Hardcoded enum
/// let service = EcosystemService::BearDog;
/// match service {
///     EcosystemService::BearDog => handle_crypto(),
///     _ => {}
/// }
///
/// // ✅ NEW: Capability-based ServiceType
/// let service_type = ServiceType::from_capability_list(capabilities);
/// if service_type.provides_crypto() {
///     handle_crypto();
/// }
/// ```
///
/// See: `crate::ecosystem::service_type::ServiceType` for the replacement.
#[deprecated(
    since = "0.1.0",
    note = "Use ServiceType instead. This enum hardcodes service names, violating infant discovery principles."
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemService {
    Songbird, // Service discovery and coordination
    BearDog,  // Cryptographic security
    NestGate, // Distributed storage
    Unknown(String),
}

#[allow(deprecated)] // Implementation of deprecated EcosystemService
impl EcosystemService {
    // Removed parse() - unused helper. Services are constructed directly.

    /// ⚠️ DEPRECATED: Use `ServiceType::display_name()` instead
    #[deprecated(since = "0.1.0", note = "Use ServiceType::display_name() instead")]
    pub(super) fn name(&self) -> &str {
        match self {
            EcosystemService::Songbird => "songbird",
            EcosystemService::BearDog => "beardog",
            EcosystemService::NestGate => "nestgate",
            EcosystemService::Unknown(name) => name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Discovered, // Found via network scan
    Advertised, // Advertised via mDNS/service mesh
    Configured, // Explicitly configured (env var/config file)
    Verified,   // Cryptographically verified
    Sovereign,  // Full sovereign verification
}

#[derive(Debug, Clone)]
pub(super) struct ServiceConnection {
    pub(super) endpoint: ServiceEndpoint,
    pub(super) status: ConnectionStatus,
    pub(super) last_heartbeat: std::time::SystemTime,
    pub(super) _auth_token: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum ConnectionStatus {
    _Connecting,
    Connected,
    _Disconnected,
    _Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EcosystemCredentials {
    pub(super) identity: String,
    pub(super) private_key: String,
    pub(super) public_key: String,
    pub(super) certificates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub services: Vec<ServiceEndpoint>,
    pub scan_duration: Duration,
    pub total_discovered: usize,
    pub verified_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogPermission {
    pub permission_id: Uuid,
    pub granted_to: String,
    pub capabilities: Vec<String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub valid_until: std::time::SystemTime,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateMount {
    pub dataset_name: String,
    pub mount_point: PathBuf,
    pub endpoint: String,
    pub zfs_dataset: Option<String>,
    pub access_mode: String, // read, write, admin
    pub encryption_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(deprecated)] // Using ServiceType during migration period
pub struct DiscoveredService {
    pub service_type: ServiceType,
    pub address: SocketAddr,
    pub trust_level: TrustLevel,
    pub capabilities: HashMap<String, String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_seen: std::time::SystemTime,
}

#[deprecated(
    since = "0.2.0",
    note = "Use capability-based service identification. See service_type.rs"
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceType {
    Songbird,
    BearDog,
    NestGate,
    ToadStool,
    /// Generic service identified by capabilities
    Generic,
}

#[allow(deprecated)]
impl ServiceType {
    /// Map to capability name (for migration)
    pub fn to_capability(&self) -> &'static str {
        match self {
            ServiceType::Songbird => "orchestration",
            ServiceType::BearDog => "pki",
            ServiceType::NestGate => "storage",
            ServiceType::ToadStool => "compute:execution",
            ServiceType::Generic => "generic",
        }
    }

    /// Create from capability (for migration)
    pub fn from_capability(capability: &str) -> Self {
        match capability {
            "orchestration" => ServiceType::Songbird,
            "pki" => ServiceType::BearDog,
            "storage" => ServiceType::NestGate,
            "compute:execution" | "compute" => ServiceType::ToadStool,
            _ => ServiceType::Generic,
        }
    }

    /// Create from service name (for migration/backward compatibility)
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "songbird" | "orchestration" => ServiceType::Songbird,
            "beardog" | "pki" => ServiceType::BearDog,
            "nestgate" | "storage" => ServiceType::NestGate,
            "toadstool" | "compute" => ServiceType::ToadStool,
            _ => ServiceType::Generic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSignature {
    pub algorithm: String,
    pub signature: String,
    pub public_key: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedServiceResponse {
    pub service_id: String,
    pub service_type: String,
    pub status: String,
    pub capabilities: Vec<String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    pub signature: ServiceSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoVerificationContext {
    pub trusted_public_keys: HashMap<String, String>,
    pub revoked_keys: Vec<String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub verification_timestamp: std::time::SystemTime,
    pub max_age_minutes: u32,
}

impl Default for CryptoVerificationContext {
    fn default() -> Self {
        // Load trusted public keys from environment or configuration
        let mut trusted_keys = HashMap::new();

        // Production keys should be loaded from secure configuration
        if let Ok(songbird_key) = std::env::var("SONGBIRD_PUBLIC_KEY") {
            trusted_keys.insert("songbird".to_string(), songbird_key);
        }
        if let Ok(beardog_key) = std::env::var("BEARDOG_PUBLIC_KEY") {
            trusted_keys.insert("beardog".to_string(), beardog_key);
        }
        if let Ok(nestgate_key) = std::env::var("NESTGATE_PUBLIC_KEY") {
            trusted_keys.insert("nestgate".to_string(), nestgate_key);
        }

        Self {
            trusted_public_keys: trusted_keys,
            revoked_keys: Vec::new(),
            verification_timestamp: std::time::SystemTime::now(),
            max_age_minutes: 5, // 5 minute max age for responses
        }
    }
}

impl CryptoVerificationContext {
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

    pub fn verify_ed25519_signature(
        &self,
        message: &[u8],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
    ) -> Result<bool> {
        // Parse public key from bytes (RustCrypto ed25519-dalek - pure Rust!)
        let public_key = VerifyingKey::from_bytes(public_key_bytes.try_into().map_err(|_| {
            crate::CliError::Other("Invalid public key length (expected 32 bytes)".to_string())
        })?)?;

        // Parse signature from bytes
        let signature = Signature::from_bytes(signature_bytes.try_into().map_err(|_| {
            crate::CliError::Other("Invalid signature length (expected 64 bytes)".to_string())
        })?);

        // Verify signature using RustCrypto (pure Rust, no C/assembly!)
        match public_key.verify(message, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

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

    // RFC 8032 Ed25519 test vector - empty message
    fn test_public_key() -> [u8; 32] {
        hex::decode("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
            .unwrap()
            .try_into()
            .unwrap()
    }
    fn test_signature() -> [u8; 64] {
        hex::decode("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")
            .unwrap()
            .try_into()
            .unwrap()
    }

    #[test]
    fn test_verify_ed25519_signature_valid() {
        let context = CryptoVerificationContext::new();
        let result = context.verify_ed25519_signature(b"", &test_signature(), &test_public_key());
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_verify_ed25519_signature_invalid() {
        let context = CryptoVerificationContext::new();
        let result = context.verify_ed25519_signature(
            b"wrong message",
            &test_signature(),
            &test_public_key(),
        );
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_verify_ed25519_signature_invalid_key_length() {
        let context = CryptoVerificationContext::new();
        let result = context.verify_ed25519_signature(
            b"msg", &[0u8; 64], &[0u8; 16], // Wrong length - should be 32
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid public key"));
    }

    #[test]
    fn test_verify_ed25519_signature_invalid_signature_length() {
        let context = CryptoVerificationContext::new();
        let result = context.verify_ed25519_signature(
            b"msg", &[0u8; 32], // Wrong length - should be 64
            &[0u8; 32],
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid signature"));
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
        assert_eq!(result.unwrap(), false);
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
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_endpoint_deserialize_roundtrip() {
        let endpoint = ServiceEndpoint {
            service_type: EcosystemService::Songbird,
            address: "127.0.0.1:8080".parse().unwrap(),
            version: Arc::from("1.0"),
            capabilities: vec!["discovery".to_string()],
            trust_level: TrustLevel::Verified,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let restored: ServiceEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version.as_ref(), "1.0");
        assert!(matches!(restored.service_type, EcosystemService::Songbird));
    }
}

// Private helper types (used internally by EcosystemIntegrator)
// SongbirdRegistration, SongbirdHeartbeat, SongbirdResponse: REMOVED
// These types were part of the deprecated hardcoded Songbird integration.
// New code uses capability-based adapters in ecosystem::adapters instead.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EcosystemStatus {
    pub(super) endpoints: HashMap<String, ServiceEndpoint>,
    pub(super) active_connections: usize,
    pub(super) total_discovered: usize,
    pub(super) credential_status: bool,
}
