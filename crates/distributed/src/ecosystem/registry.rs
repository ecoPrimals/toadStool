// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service registry
pub struct ServiceRegistry {
    _services: Arc<RwLock<HashMap<String, RegisteredService>>>,
}

/// Registered service
#[derive(Debug, Clone)]
pub struct RegisteredService {
    pub name: String,
    pub endpoint: String,
    pub health_check_url: String,
    pub last_seen: std::time::SystemTime,
}

impl ServiceRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _services: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
