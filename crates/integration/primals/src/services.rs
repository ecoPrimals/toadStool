use crate::error::PrimalResult;
use serde::{Deserialize, Serialize};

/// Service manager for primal services
pub struct ServiceManager {
    services: std::collections::HashMap<String, ServiceInfo>,
}

/// Information about a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub service_id: String,
    pub service_type: String,
    pub status: ServiceStatus,
    pub endpoint: String,
}

/// Status of a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Running,
    Stopped,
    Error(String),
    Unknown,
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: std::collections::HashMap::new(),
        }
    }

    pub async fn start_service(
        &mut self,
        service_id: String,
        service_type: String,
    ) -> PrimalResult<()> {
        // Use environment-aware configuration for service endpoints
        let port: u16 = std::env::var("TOADSTOOL_SONGBIRD_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| {
                let config = toadstool_config::env_config::EnvironmentConfig::from_env();
                config.network.songbird_port
            });
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let host = &config.network.bind_address;
        
        let service_info = ServiceInfo {
            service_id: service_id.clone(),
            service_type,
            status: ServiceStatus::Running,
            endpoint: format!("http://{host}:{port}/{service_id}"),
        };
        self.services.insert(service_id, service_info);
        Ok(())
    }

    pub async fn stop_service(&mut self, service_id: &str) -> PrimalResult<()> {
        if let Some(service) = self.services.get_mut(service_id) {
            service.status = ServiceStatus::Stopped;
        }
        Ok(())
    }

    pub fn get_service(&self, service_id: &str) -> Option<&ServiceInfo> {
        self.services.get(service_id)
    }
}
