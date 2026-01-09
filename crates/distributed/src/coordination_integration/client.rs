//! Coordination service client - Capability-based discovery
//!
//! **Design Philosophy (Infant Discovery)**:
//! - Async-first: Non-blocking operations
//! - Resilient: Retry logic, circuit breaker patterns
//! - Observable: Metrics and health checks
//! - Zero hardcoding: Endpoints discovered at runtime by capability
//! - Multi-vendor: Works with ANY coordination service (Songbird, Consul, etcd, K8s, etc.)

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use super::types::{
    CoordinationRequest, CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, NodeInfo,
    ServiceRegistration,
};
use super::{CoordinationConfig, ServiceLocation};

/// Coordination service discovery - Finds coordination providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CoordinationDiscovery {
    config: CoordinationConfig,
    discovery: ServiceDiscovery,
    discovered_services: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl CoordinationDiscovery {
    /// Create new discovery instance
    pub async fn new(config: CoordinationConfig) -> ToadStoolResult<Self> {
        // Use Auto discovery method - tries all strategies
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        Ok(Self {
            config,
            discovery,
            discovered_services: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Discover coordination services by capability
    ///
    /// **Design**: Multi-strategy discovery (mDNS, registry, environment)
    pub async fn discover(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Discover by each required capability
        for cap in &self.config.required_capabilities {
            let capability = Capability::Coordination(cap.clone());

            if let Ok(service) = self.discovery.find_service_by_capability(capability).await {
                services.push(service);
            }
        }

        // Remove duplicates (same service ID)
        services.dedup_by(|a, b| a.id == b.id);

        // Filter by location preference
        let filtered = self.filter_by_location(&services);

        // Cache discovered services
        let mut cache = self.discovered_services.write().await;
        *cache = filtered.clone();

        Ok(filtered)
    }

    /// Discover by specific capability
    pub async fn discover_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<Option<DiscoveredService>> {
        self.discovery
            .find_service_by_capability(capability)
            .await
            .map(Some)
            .or_else(|_| Ok(None))
    }

    /// Filter services by location preference
    fn filter_by_location(&self, services: &[DiscoveredService]) -> Vec<DiscoveredService> {
        match self.config.preferred_location {
            ServiceLocation::Local => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        e.address.starts_with("127.") || e.address.starts_with("localhost")
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Network => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        !e.address.starts_with("127.") && !e.address.starts_with("localhost")
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Any => services.to_vec(),
        }
    }

    /// Get cached services
    pub async fn get_cached(&self) -> Vec<DiscoveredService> {
        self.discovered_services.read().await.clone()
    }
}

/// Coordination service client - Makes requests to discovered services
///
/// **Design**: Works with ANY coordination provider's HTTP API
pub struct CoordinationClient {
    http_client: reqwest::Client,
    service_endpoint: ServiceEndpoint,
    timeout: Duration,
}

impl CoordinationClient {
    /// Create client for a discovered service
    pub fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        let endpoint = service.endpoints.first().ok_or_else(|| {
            ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: service.name.clone(),
                reason: "No endpoints available".to_string(),
            })
        })?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::ConnectionFailed {
                    endpoint: "http_client".to_string(),
                    reason: e.to_string(),
                })
            })?;

        Ok(Self {
            http_client,
            service_endpoint: endpoint.clone(),
            timeout: Duration::from_secs(30),
        })
    }

    /// Create client with custom timeout
    pub fn with_timeout(service: &DiscoveredService, timeout: Duration) -> ToadStoolResult<Self> {
        let mut client = Self::new(service)?;
        client.timeout = timeout;
        Ok(client)
    }

    /// Register a service with the coordination provider
    pub async fn register_service(
        &self,
        registration: ServiceRegistration,
    ) -> ToadStoolResult<CoordinationResponse> {
        let url = format!("{}/api/v1/services/register", self.service_endpoint.url());

        let response = self
            .http_client
            .post(&url)
            .json(&registration)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        if !response.status().is_success() {
            return Err(ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service registration failed: {}", response.status()),
            }));
        }

        response.json::<CoordinationResponse>().await.map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: e.to_string(),
            })
        })
    }

    /// Discover services by capability
    pub async fn discover_services(&self, capability: &str) -> ToadStoolResult<Vec<NodeInfo>> {
        let url = format!(
            "{}/api/v1/services/discover?capability={}",
            self.service_endpoint.url(),
            capability
        );

        let response = self
            .http_client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        if !response.status().is_success() {
            return Err(ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service discovery failed: {}", response.status()),
            }));
        }

        response.json::<Vec<NodeInfo>>().await.map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: e.to_string(),
            })
        })
    }

    /// Report health status
    pub async fn report_health(
        &self,
        health: HealthCheckRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        let url = format!("{}/api/v1/health/report", self.service_endpoint.url());

        let response = self
            .http_client
            .post(&url)
            .json(&health)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        if !response.status().is_success() {
            return Err(ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Health report failed: {}", response.status()),
            }));
        }

        response.json::<CoordinationResponse>().await.map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: e.to_string(),
            })
        })
    }

    /// Get load balancing advice
    pub async fn get_load_balancing(
        &self,
        request: LoadBalancingRequest,
    ) -> ToadStoolResult<Vec<NodeInfo>> {
        let url = format!(
            "{}/api/v1/loadbalancing/advice",
            self.service_endpoint.url()
        );

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        if !response.status().is_success() {
            return Err(ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Load balancing request failed: {}", response.status()),
            }));
        }

        response.json::<Vec<NodeInfo>>().await.map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: e.to_string(),
            })
        })
    }

    /// Health check
    pub async fn health_check(&self) -> ToadStoolResult<bool> {
        let url = format!("{}/health", self.service_endpoint.url());

        let response = self
            .http_client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        Ok(response.status().is_success())
    }

    /// Execute generic coordination request
    pub async fn execute(
        &self,
        request: CoordinationRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        let url = format!("{}/api/v1/coordination", self.service_endpoint.url());

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        if !response.status().is_success() {
            return Err(ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Coordination request failed: {}", response.status()),
            }));
        }

        response.json::<CoordinationResponse>().await.map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: e.to_string(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::primal_identity::{CoordinationCapability, ServiceEndpoint};

    #[tokio::test]
    async fn test_coordination_discovery_creation() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await;

        assert!(discovery.is_ok());
    }

    #[tokio::test]
    async fn test_location_filtering() {
        let config = CoordinationConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CoordinationDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-coord".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http("127.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_health_check: None,
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote-coord".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http("10.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_health_check: None,
                healthy: true,
            },
        ];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "local");
    }

    #[test]
    fn test_service_location_types() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
    }
}
