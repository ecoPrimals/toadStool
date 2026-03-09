// SPDX-License-Identifier: AGPL-3.0-only
//! BearDog integration types
//!
//! **Design Philosophy**:
//! - Protocol-agnostic: Support HTTP, mDNS, or future protocols
//! - Version-agnostic: Handle API evolution gracefully
//! - Type-safe: Strong types, no stringly-typed

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

/// BearDog service endpoint (discovered at runtime)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BearDogEndpoint {
    /// Service identifier
    pub service_id: String,

    /// Protocol (http, https, grpc, etc.)
    pub protocol: String,

    /// Address (discovered via mDNS or other discovery)
    pub address: SocketAddr,

    /// API version
    pub api_version: String,

    /// Service capabilities
    pub capabilities: Vec<BearDogCapability>,

    /// Health status
    pub healthy: bool,

    /// Response latency (milliseconds)
    pub latency_ms: Option<u64>,
}

/// BearDog capabilities (what BearDog can do)
///
/// **Design**: Capability-based, extensible
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BearDogCapability {
    /// Encryption/decryption
    Encryption { algorithms: Vec<String> },
    /// Key management
    KeyManagement,
    /// Genetic entropy
    GeneticEntropy,
    /// Hardware security module
    HardwareSecurity,
    /// Secure storage
    SecureStorage,
    /// Custom capability
    Custom(String),
}

/// Encryption request to BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Operation (encrypt/decrypt)
    pub operation: EncryptionOperation,

    /// Data to encrypt/decrypt
    pub data: Vec<u8>,

    /// Key identifier (if using existing key)
    pub key_id: Option<String>,

    /// Algorithm preference
    pub algorithm: Option<String>,

    /// Security level required
    pub security_level: SecurityLevel,
}

/// Encryption response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Result data
    pub data: Vec<u8>,

    /// Key used
    pub key_id: String,

    /// Algorithm used
    pub algorithm: String,

    /// Metadata (nonce, tag, etc.)
    pub metadata: serde_json::Value,
}

/// Encryption operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EncryptionOperation {
    Encrypt,
    Decrypt,
}

/// Security level for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SecurityLevel {
    Standard,
    Enhanced,
    HardwareSecured,
}

/// Key management request to BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Operation type
    pub operation: KeyOperation,

    /// Key identifier (for get/delete)
    pub key_id: Option<String>,

    /// Security level (for generate)
    pub security_level: Option<SecurityLevel>,
}

/// Key management response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Operation result
    pub result: KeyOperationResult,
}

/// Key operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum KeyOperation {
    /// Generate new key
    Generate,
    /// Get existing key
    Get,
    /// Delete key
    Delete,
    /// List keys
    List,
}

/// Key operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyOperationResult {
    /// Key generated
    Generated { key_id: String, algorithm: String },
    /// Key retrieved
    Retrieved {
        key_id: String,
        key_material: Vec<u8>,
        algorithm: String,
    },
    /// Key deleted
    Deleted { key_id: String },
    /// Keys listed
    Listed { keys: Vec<String> },
    /// Operation failed
    Error { message: String },
}

/// Signature request to BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Data to sign
    pub data: Vec<u8>,

    /// Key identifier (if using existing key)
    pub key_id: Option<String>,

    /// Algorithm preference
    pub algorithm: Option<String>,
}

/// Signature response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Signature data
    pub signature: Vec<u8>,

    /// Key used
    pub key_id: String,

    /// Algorithm used
    pub algorithm: String,
}

/// Verification request to BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    /// Request identifier
    pub request_id: Uuid,

    /// Original data
    pub data: Vec<u8>,

    /// Signature to verify
    pub signature: Vec<u8>,

    /// Public key identifier
    pub public_key_id: String,
}

/// Verification response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Whether signature is valid
    pub valid: bool,

    /// Verification details (optional)
    pub details: Option<String>,
}

/// Permission response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Permission identifier
    pub permission_id: Uuid,

    /// Cryptographic proof
    pub proof: Vec<u8>,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Validation response from BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    /// Request identifier
    pub request_id: Uuid,

    /// Whether permission is valid
    pub valid: bool,

    /// Validation details (optional)
    pub details: Option<String>,
}

/// Revocation request to BearDog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationRequest {
    /// Reason for revocation
    pub reason: String,
}
