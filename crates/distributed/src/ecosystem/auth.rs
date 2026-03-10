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

/// Auth token
#[derive(Debug, Clone)]
pub struct AuthToken {
    pub token: String,
    pub expires_at: std::time::SystemTime,
}

/// Credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl AuthenticationManager {
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
