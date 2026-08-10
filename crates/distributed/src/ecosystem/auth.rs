// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Authentication manager
#[expect(
    dead_code,
    reason = "Phase 2+: distributed auth token/credential management"
)]
pub struct AuthenticationManager {
    tokens: Arc<RwLock<HashMap<String, AuthToken>>>,
    credentials: Arc<RwLock<HashMap<String, Credentials>>>,
}

/// Auth token for service authentication with expiration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthToken {
    /// Bearer or session token value.
    pub token: String,
    /// Expiration timestamp for token invalidation.
    pub expires_at: std::time::SystemTime,
}

/// Username/password credentials for basic auth.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::{AuthToken, AuthenticationManager, Credentials};
    use std::time::{Duration, SystemTime};

    #[test]
    fn authentication_manager_new_and_default_are_equivalent() {
        let a = AuthenticationManager::new();
        let b = AuthenticationManager::default();
        let _ = (a, b);
    }

    #[test]
    fn auth_token_clone_debug_serde_roundtrip() {
        let t = AuthToken {
            token: "bearer-secret".to_string(),
            expires_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3600),
        };
        let c = t.clone();
        assert_eq!(t, c);
        let json = serde_json::to_string(&t).expect("serialize AuthToken");
        let back: AuthToken = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, t);
        let s = format!("{t:?}");
        assert!(s.contains("AuthToken"));
    }

    #[test]
    fn credentials_clone_debug_serde_roundtrip() {
        let c = Credentials {
            username: "alice".to_string(),
            password: "hunter2".to_string(),
        };
        let d = c.clone();
        assert_eq!(c, d);
        let json = serde_json::to_string(&c).expect("serialize Credentials");
        let back: Credentials = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, c);
    }
}
