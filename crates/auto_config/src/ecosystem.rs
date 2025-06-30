//! Ecosystem service discovery for auto-configuration

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug};

use toadstool::error::ToadStoolResult;

/// Ecosystem service discovery
pub struct EcosystemDiscoverer {
    discovered_services: HashMap<String, ServiceInfo>,
    connection_health: HashMap<String, HealthStatus>,
}

impl Default for EcosystemDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

impl EcosystemDiscoverer {
    /// Create new ecosystem discoverer
    pub fn new() -> Self {
        Self {
            discovered_services: HashMap::new(),
            connection_health: HashMap::new(),
        }
    }
    
    /// Automatically discover all ecosystem services
    pub async fn discover_services(&mut self) -> ToadStoolResult<EcosystemMap> {
        info!("🌐 Discovering ecosystem services...");
        
        let mut ecosystem = EcosystemMap::new();
        
        // 1. Try to discover Songbird (service discovery hub)
        if let Ok(songbird) = self.discover_songbird().await {
            info!("🎼 Found Songbird service discovery hub");
            ecosystem.add_service("songbird".to_string(), songbird);
            
            // 2. Use Songbird to discover other services
            if let Ok(services) = self.discover_via_songbird(&ecosystem.discovered_services["songbird"]).await {
                for (name, service) in services {
                    ecosystem.add_service(name, service);
                }
            }
        } else {
            info!("🔍 Songbird not found, trying direct discovery");
            // 3. Fallback to direct discovery
            ecosystem.extend(self.discover_direct().await?);
        }
        
        // 4. Test connections and optimize
        ecosystem.test_all_connections().await?;
        
        info!("✅ Ecosystem discovery complete: found {} services", 
              ecosystem.discovered_services.len());
        
        Ok(ecosystem)
    }
    
    /// Discover Songbird service discovery hub
    async fn discover_songbird(&self) -> ToadStoolResult<ServiceInfo> {
        info!("🎼 Looking for Songbird service discovery...");
        
        // Try common Songbird locations
        let common_endpoints = vec![
            "http://localhost:8080",
            "http://songbird:8080", 
            "http://songbird.local:8080",
            "http://127.0.0.1:8080",
            "http://0.0.0.0:8080",
        ];
        
        for endpoint in common_endpoints {
            debug!("Trying Songbird endpoint: {}", endpoint);
            if let Ok(service) = self.test_songbird_endpoint(endpoint).await {
                info!("🎼 Found Songbird at: {}", endpoint);
                return Ok(service);
            }
        }
        
        // Try environment variables
        if let Ok(endpoint) = std::env::var("SONGBIRD_ENDPOINT") {
            if let Ok(service) = self.test_songbird_endpoint(&endpoint).await {
                info!("🎼 Found Songbird via environment: {}", endpoint);
                return Ok(service);
            }
        }
        
        Err(toadstool::error::ToadStoolError::not_found("Songbird service not found"))
    }
    
    /// Test a specific Songbird endpoint
    async fn test_songbird_endpoint(&self, endpoint: &str) -> ToadStoolResult<ServiceInfo> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| toadstool::error::ToadStoolError::network(format!("HTTP client error: {}", e)))?;
        
        // Try to get service info from Songbird
        let url = format!("{}/api/v1/info", endpoint);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(info) = response.json::<SongbirdInfo>().await {
                    return Ok(ServiceInfo {
                        name: "songbird".to_string(),
                        service_type: ServiceType::ServiceDiscovery,
                        endpoint: endpoint.to_string(),
                        version: info.version,
                        capabilities: info.capabilities,
                        health_status: HealthStatus::Healthy,
                        last_seen: chrono::Utc::now(),
                    });
                }
            },
            _ => {
                // Try a simple health check
                let health_url = format!("{}/health", endpoint);
                if let Ok(response) = client.get(&health_url).send().await {
                    if response.status().is_success() {
                        return Ok(ServiceInfo {
                            name: "songbird".to_string(),
                            service_type: ServiceType::ServiceDiscovery,
                            endpoint: endpoint.to_string(),
                            version: "unknown".to_string(),
                            capabilities: vec!["service_discovery".to_string()],
                            health_status: HealthStatus::Healthy,
                            last_seen: chrono::Utc::now(),
                        });
                    }
                }
            }
        }
        
        Err(toadstool::error::ToadStoolError::not_found("Songbird not accessible"))
    }
    
    /// Discover services via Songbird
    async fn discover_via_songbird(&self, songbird: &ServiceInfo) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        info!("🔍 Discovering services via Songbird...");
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| toadstool::error::ToadStoolError::network(format!("HTTP client error: {}", e)))?;
        
        let url = format!("{}/api/v1/services", songbird.endpoint);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                if let Ok(services) = response.json::<Vec<SongbirdServiceInfo>>().await {
                    let mut discovered = HashMap::new();
                    for service in services {
                        let service_info = ServiceInfo {
                            name: service.name.clone(),
                            service_type: ServiceType::from_string(&service.service_type),
                            endpoint: service.endpoint,
                            version: service.version,
                            capabilities: service.capabilities,
                            health_status: HealthStatus::from_string(&service.status),
                            last_seen: chrono::Utc::now(),
                        };
                        discovered.insert(service.name, service_info);
                    }
                    info!("🌐 Discovered {} services via Songbird", discovered.len());
                    return Ok(discovered);
                }
            },
            Ok(response) => {
                warn!("Songbird services endpoint returned: {}", response.status());
            },
            Err(e) => {
                warn!("Failed to query Songbird services: {}", e);
            }
        }
        
        Ok(HashMap::new())
    }
    
    /// Direct service discovery (without Songbird)
    async fn discover_direct(&self) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        info!("🔍 Attempting direct service discovery...");
        
        let mut services = HashMap::new();
        
        // Try to discover NestGate (storage service)
        if let Ok(nestgate) = self.discover_nestgate().await {
            services.insert("nestgate".to_string(), nestgate);
        }
        
        info!("📡 Direct discovery found {} services", services.len());
        Ok(services)
    }
    
    /// Discover NestGate storage service
    async fn discover_nestgate(&self) -> ToadStoolResult<ServiceInfo> {
        let common_endpoints = vec![
            "http://localhost:9000",  // MinIO default
            "http://nestgate:9000",
            "http://nestgate.local:9000",
        ];
        
        for endpoint in common_endpoints {
            if let Ok(service) = self.test_nestgate_endpoint(endpoint).await {
                info!("🏠 Found NestGate at: {}", endpoint);
                return Ok(service);
            }
        }
        
        Err(toadstool::error::ToadStoolError::not_found("NestGate service not found"))
    }
    
    /// Test a specific NestGate endpoint
    async fn test_nestgate_endpoint(&self, endpoint: &str) -> ToadStoolResult<ServiceInfo> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| toadstool::error::ToadStoolError::network(format!("HTTP client error: {}", e)))?;
        
        // Try MinIO health check
        let url = format!("{}/minio/health/live", endpoint);
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                return Ok(ServiceInfo {
                    name: "nestgate".to_string(),
                    service_type: ServiceType::Storage,
                    endpoint: endpoint.to_string(),
                    version: "unknown".to_string(),
                    capabilities: vec!["object_storage".to_string(), "s3_compatible".to_string()],
                    health_status: HealthStatus::Healthy,
                    last_seen: chrono::Utc::now(),
                });
            },
            _ => {}
        }
        
        Err(toadstool::error::ToadStoolError::not_found("NestGate not accessible"))
    }
}

/// Map of discovered ecosystem services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemMap {
    pub discovered_services: HashMap<String, ServiceInfo>,
}

impl Default for EcosystemMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EcosystemMap {
    pub fn new() -> Self {
        Self {
            discovered_services: HashMap::new(),
        }
    }
    
    pub fn add_service(&mut self, name: String, service: ServiceInfo) {
        self.discovered_services.insert(name, service);
    }
    
    pub fn extend(&mut self, services: HashMap<String, ServiceInfo>) {
        self.discovered_services.extend(services);
    }
    
    pub fn get_service(&self, name: &str) -> Option<&ServiceInfo> {
        self.discovered_services.get(name)
    }
    
    /// Test all service connections and update health status
    pub async fn test_all_connections(&mut self) -> ToadStoolResult<()> {
        info!("🔍 Testing connections to all discovered services...");
        
        // Collect service names to avoid borrow checker issues
        let service_names: Vec<String> = self.discovered_services.keys().cloned().collect();
        
        for name in service_names {
            if let Some(service) = self.discovered_services.get(&name).cloned() {
                match self.test_service_connection(&service).await {
                    Ok(health) => {
                        if let Some(service_mut) = self.discovered_services.get_mut(&name) {
                            service_mut.health_status = health;
                            debug!("Service {} is {:?}", name, health);
                        }
                    },
                    Err(e) => {
                        if let Some(service_mut) = self.discovered_services.get_mut(&name) {
                            service_mut.health_status = HealthStatus::Unhealthy;
                            warn!("Service {} health check failed: {}", name, e);
                        }
                    }
                }
            }
        }
        
        info!("✅ Service health checks complete");
        Ok(())
    }
    
    /// Test connection to a specific service
    async fn test_service_connection(&self, service: &ServiceInfo) -> ToadStoolResult<HealthStatus> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| toadstool::error::ToadStoolError::network(format!("HTTP client error: {}", e)))?;
        
        // Try common health check endpoints
        let health_endpoints = vec!["/health", "/api/health", "/status", "/ping"];
        
        for path in health_endpoints {
            let url = format!("{}{}", service.endpoint, path);
            if let Ok(response) = client.get(&url).send().await {
                if response.status().is_success() {
                    return Ok(HealthStatus::Healthy);
                }
            }
        }
        
        Ok(HealthStatus::Unhealthy)
    }
}

/// Information about a discovered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: ServiceType,
    pub endpoint: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub health_status: HealthStatus,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Type of ecosystem service
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServiceType {
    ServiceDiscovery,
    Storage,
    Compute,
    Monitoring,
    Security,
    Unknown,
}

impl ServiceType {
    fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "service_discovery" | "discovery" => ServiceType::ServiceDiscovery,
            "storage" | "object_storage" => ServiceType::Storage,
            "compute" | "execution" => ServiceType::Compute,
            "monitoring" | "metrics" => ServiceType::Monitoring,
            "security" | "auth" => ServiceType::Security,
            _ => ServiceType::Unknown,
        }
    }
}

/// Health status of a service
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "healthy" | "up" | "ok" => HealthStatus::Healthy,
            "degraded" | "warning" => HealthStatus::Degraded,
            "unhealthy" | "down" | "error" => HealthStatus::Unhealthy,
            _ => HealthStatus::Unknown,
        }
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Songbird service information response
#[derive(Debug, Deserialize)]
struct SongbirdInfo {
    version: String,
    capabilities: Vec<String>,
}

/// Songbird service list response
#[derive(Debug, Deserialize)]
struct SongbirdServiceInfo {
    name: String,
    service_type: String,
    endpoint: String,
    version: String,
    capabilities: Vec<String>,
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_ecosystem_discovery() {
        let mut discoverer = EcosystemDiscoverer::new();
        let ecosystem = discoverer.discover_services().await.unwrap();
        
        // Should not fail even if no services are found
        assert!(ecosystem.discovered_services.len() >= 0);
    }
    
    #[test]
    fn test_service_type_from_string() {
        assert!(matches!(ServiceType::from_string("storage"), ServiceType::Storage));
        assert!(matches!(ServiceType::from_string("compute"), ServiceType::Compute));
        assert!(matches!(ServiceType::from_string("unknown"), ServiceType::Unknown));
    }
} 