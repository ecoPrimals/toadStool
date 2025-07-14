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
        let service_info = ServiceInfo {
            service_id: service_id.clone(),
            service_type,
            status: ServiceStatus::Running,
            endpoint: format!("http://localhost:8080/{service_id}"),
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
