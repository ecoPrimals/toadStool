// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Service registry
pub struct ServiceRegistry {
    _services: Arc<RwLock<HashMap<String, RegisteredService>>>,
}

/// A service registered in the discovery registry with health metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::{RegisteredService, ServiceRegistry};
    use std::time::{Duration, SystemTime};

    #[test]
    fn service_registry_new_and_default() {
        let a = ServiceRegistry::new();
        let b = ServiceRegistry::default();
        let _ = (a, b);
    }

    #[test]
    fn registered_service_clone_debug_serde_roundtrip() {
        let svc = RegisteredService {
            name: "api".to_string(),
            endpoint: "https://api.example/v1".to_string(),
            health_check_url: "https://api.example/health".to_string(),
            last_seen: SystemTime::UNIX_EPOCH + Duration::from_secs(100),
        };
        let c = svc.clone();
        assert_eq!(svc, c);
        let json = serde_json::to_string(&svc).expect("serialize RegisteredService");
        let back: RegisteredService = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, svc);
        let dbg = format!("{svc:?}");
        assert!(dbg.contains("RegisteredService"));
    }
}
