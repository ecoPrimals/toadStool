// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for the security service client.

use toadstool_common::capability_provider::CapabilityError;

/// Errors for security service client
#[derive(Debug, thiserror::Error)]
pub enum SecurityClientError {
    /// No security provider was discovered.
    #[error("No security provider found")]
    NoProvider,

    /// Encryption operation failed with the given message.
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption operation failed with the given message.
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Signing operation failed with the given message.
    #[error("Signature failed: {0}")]
    SignatureFailed(String),

    /// Signature verification failed with the given message.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Key management operation failed with the given message.
    #[error("Key management failed: {0}")]
    KeyManagementFailed(String),

    /// Token validation failed with the given message.
    #[error("Token validation failed: {0}")]
    ValidationFailed(String),

    /// Underlying capability discovery or RPC error.
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type alias for [`SecurityClient`](super::SecurityClient) operations.
pub type Result<T> = std::result::Result<T, SecurityClientError>;
