//! Token types and validation

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Authentication token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationToken {
    pub id: String,
    pub token_type: String,
    pub token: String,
    pub public_key: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub issued_at: chrono::DateTime<chrono::Utc>,
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
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Token refresh request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshRequest {
    pub requesting_primal: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Token verification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationRequest {
    pub primal_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: String,
}

/// Token verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVerificationResponse {
    pub status: TokenVerificationStatus,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
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
