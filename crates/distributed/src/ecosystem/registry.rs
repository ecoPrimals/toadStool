use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Service registry
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, RegisteredService>>>,
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
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
