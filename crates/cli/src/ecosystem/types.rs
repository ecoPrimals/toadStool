//! Ecosystem Integration - Type Definitions

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::PathBuf;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub service_type: EcosystemService,
    pub address: SocketAddr,
    pub version: String,
    pub capabilities: Vec<String>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EcosystemService {
    Songbird, // Service discovery and coordination
    BearDog,  // Cryptographic security
    NestGate, // Distributed storage
    Unknown(String),
}

impl EcosystemService {
    pub(super) fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "songbird" => EcosystemService::Songbird,
            "beardog" => EcosystemService::BearDog,
            "nestgate" => EcosystemService::NestGate,
            _ => EcosystemService::Unknown(s.to_string()),
        }
    }

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
    Advertised, // Advertised via Songbird
    Verified,   // Cryptographically verified
    Sovereign,  // Full sovereign verification
}

#[derive(Debug, Clone)]
pub(super) struct ServiceConnection {
    pub(super) endpoint: ServiceEndpoint,
    pub(super) status: ConnectionStatus,
    pub(super) last_heartbeat: chrono::DateTime<chrono::Utc>,
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
    pub valid_until: chrono::DateTime<chrono::Utc>,
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
pub struct DiscoveredService {
    pub service_type: ServiceType,
    pub address: SocketAddr,
    pub trust_level: TrustLevel,
    pub capabilities: HashMap<String, String>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ServiceType {
    Songbird,
    BearDog,
    NestGate,
    ToadStool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSignature {
    pub algorithm: String,
    pub signature: String,
    pub public_key: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedServiceResponse {
    pub service_id: String,
    pub service_type: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: ServiceSignature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoVerificationContext {
    pub trusted_public_keys: HashMap<String, String>,
    pub revoked_keys: Vec<String>,
    pub verification_timestamp: chrono::DateTime<chrono::Utc>,
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
            verification_timestamp: chrono::Utc::now(),
            max_age_minutes: 5, // 5 minute max age for responses
        }
    }
}

impl CryptoVerificationContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_trusted_key(mut self, service: String, public_key: String) -> Self {
        self.trusted_public_keys.insert(service, public_key);
        self
    }

    pub fn verify_ed25519_signature(
        &self,
        message: &[u8],
        signature_bytes: &[u8],
        public_key_bytes: &[u8],
    ) -> Result<bool> {
        let public_key = UnparsedPublicKey::new(&ED25519, public_key_bytes);
        match public_key.verify(message, signature_bytes) {
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
        let public_key_base64 = if let Some(key) = self.trusted_public_keys.get(service_type) {
            key
        } else {
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
        let response_age = chrono::Utc::now()
            .signed_duration_since(response.timestamp)
            .num_minutes();

        if response_age > i64::from(self.max_age_minutes) {
            warn!("Service response is too old: {} minutes", response_age);
            return Ok(false);
        }

        // Decode public key and signature
        let public_key_bytes = BASE64
            .decode(public_key_base64)
            .map_err(|e| anyhow::anyhow!("Failed to decode public key: {e}"))?;

        let signature_bytes = BASE64
            .decode(&response.signature.signature)
            .map_err(|e| anyhow::anyhow!("Failed to decode signature: {e}"))?;

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
        let timestamp = response.timestamp.to_rfc3339();
        data.insert("timestamp", &timestamp);
        data.insert("nonce", &response.signature.nonce);

        let capabilities_json = serde_json::to_string(&response.capabilities)
            .map_err(|e| anyhow::anyhow!("Failed to serialize capabilities: {e}"))?;
        data.insert("capabilities", &capabilities_json);

        let canonical_json = serde_json::to_string(&data)
            .map_err(|e| anyhow::anyhow!("Failed to create canonical message: {e}"))?;

        Ok(canonical_json.into_bytes())
    }
}

// Private helper types (used internally by EcosystemIntegrator)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SongbirdRegistration {
    pub(super) service_name: String,
    pub(super) service_type: String,
    pub(super) address: SocketAddr,
    pub(super) capabilities: Vec<String>,
    pub(super) metadata: HashMap<String, String>,
    pub(super) auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub(super) struct SongbirdHeartbeat {
    pub(super) service_id: String,
    pub(super) status: String,
    pub(super) timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SongbirdResponse {
    pub(super) service_id: String,
    pub(super) registry_url: String,
    pub(super) heartbeat_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct EcosystemStatus {
    pub(super) endpoints: HashMap<String, ServiceEndpoint>,
    pub(super) active_connections: usize,
    pub(super) total_discovered: usize,
    pub(super) credential_status: bool,
}
