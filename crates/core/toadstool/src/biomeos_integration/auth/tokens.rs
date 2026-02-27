//! Token types and validation

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationToken {
    pub id: String,
    pub token_type: String,
    pub token: String,
    pub public_key: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub expires_at: SystemTime,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub issued_at: SystemTime,
    pub issuer: String,
    pub audience: Vec<String>,
    pub scope: Vec<String>,
    pub claims: HashMap<String, serde_json::Value>,
}

/// Token request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    pub requesting_primal: String,
    pub scope: Vec<String>,
    pub audience: Vec<String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    pub requesting_primal: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
}

/// Token verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationRequest {
    pub primal_name: String,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    pub signature: String,
}

/// Token verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationResponse {
    pub status: TokenVerificationStatus,
    #[serde(default, with = "toadstool_common::system_time_serde::opt")]
    pub expires_at: Option<SystemTime>,
    pub details: Option<String>,
}

/// Token verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenVerificationStatus {
    Valid,
    Expired,
    Invalid,
    NotFound,
    Error(String),
}
