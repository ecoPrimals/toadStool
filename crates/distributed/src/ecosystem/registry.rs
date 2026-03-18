// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service registry
pub struct ServiceRegistry {
    _services: Arc<RwLock<HashMap<String, RegisteredService>>>,
}

/// A service registered in the discovery registry with health metadata.
#[derive(Debug, Clone)]
pub struct RegisteredService {
    /// Service name for lookup.
    pub name: String,
    /// Base endpoint URL for requests.
    pub endpoint: String,
    /// URL for health check probes.
    pub health_check_url: String,
    /// Last time the service was seen (heartbeat).
    pub last_seen: std::time::SystemTime,
}

impl ServiceRegistry {
    /// Creates an empty service registry.
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
