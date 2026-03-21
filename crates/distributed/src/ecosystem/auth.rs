// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Authentication manager
#[allow(
    dead_code,
    reason = "Phase 2+: distributed auth token/credential management"
)]
pub struct AuthenticationManager {
    tokens: Arc<RwLock<HashMap<String, AuthToken>>>,
    credentials: Arc<RwLock<HashMap<String, Credentials>>>,
}

/// Auth token for service authentication with expiration.
#[derive(Debug, Clone)]
pub struct AuthToken {
    /// Bearer or session token value.
    pub token: String,
    /// Expiration timestamp for token invalidation.
    pub expires_at: std::time::SystemTime,
}

/// Username/password credentials for basic auth.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// Username for authentication.
    pub username: String,
    /// Password (stored securely in production).
    pub password: String,
}

impl AuthenticationManager {
    /// Creates a new authentication manager with empty token and credential stores.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            credentials: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for AuthenticationManager {
    fn default() -> Self {
        Self::new()
    }
}
