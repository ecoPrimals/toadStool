// SPDX-License-Identifier: AGPL-3.0-only
//! Token types and validation

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationToken {
    /// Unique token identifier.
    pub id: String,
    /// Token type (e.g. JWT, bearer).
    pub token_type: String,
    /// Opaque token string.
    pub token: String,
    /// Public key for signature verification.
    pub public_key: String,
    /// Token expiration time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub expires_at: SystemTime,
    /// Token issuance time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub issued_at: SystemTime,
    /// Issuing primal or service.
    pub issuer: String,
    /// Intended audience (primal names).
    pub audience: Vec<String>,
    /// Authorization scopes.
    pub scope: Vec<String>,
    /// Additional JWT-style claims.
    pub claims: HashMap<String, serde_json::Value>,
}

/// Token request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    /// Primal requesting the token.
    pub requesting_primal: String,
    /// Requested scopes.
    pub scope: Vec<String>,
    /// Intended audience.
    pub audience: Vec<String>,
    /// Request timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    /// Primal requesting refresh.
    pub requesting_primal: String,
    /// Refresh request timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

/// Token verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationRequest {
    /// Primal name to verify token for.
    pub primal_name: String,
    /// Verification request timestamp.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    /// Ed25519 signature over verification payload.
    pub signature: String,
}

/// Token verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationResponse {
    /// Verification outcome.
    pub status: TokenVerificationStatus,
    /// Token expiry if valid.
    #[serde(default, with = "toadstool_common::system_time_serde::opt")]
    pub expires_at: Option<SystemTime>,
    /// Optional error or status details.
    pub details: Option<String>,
}

/// Token verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenVerificationStatus {
    /// Token is valid and not expired.
    Valid,
    /// Token has expired.
    Expired,
    /// Token signature or format invalid.
    Invalid,
    /// Token not found.
    NotFound,
    /// Verification error with message.
    Error(String),
}
