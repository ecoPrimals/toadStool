//! # Generic Orchestrator Integration
//!
//! This module provides capability-based orchestrator discovery and integration,
//! following the Infant Discovery principle: **"Each primal knows only itself."**
//!
//! Instead of hardcoding knowledge of specific orchestrators (like Songbird),
//! ToadStool discovers orchestration services via capabilities at runtime.
//!
//! ## Core Principle
//!
//! ```ignore
//! // ❌ BAD - Hardcoded orchestrator
//! let songbird = SongbirdClient::new();
//!
//! // ✅ GOOD - Capability discovery
//! let discovery = DiscoveryEngine::new();
//! let orchestrator = OrchestratorClient::discover(&discovery).await?;
//! ```

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};

use toadstool_common::infant_discovery::{CapabilityDiscovery, DiscoveredService, DiscoveryError};

/// Generic orchestrator client - works with ANY orchestrator implementation
#[derive(Debug, Clone)]
pub struct OrchestratorClient {
    /// Base endpoint URL
    endpoint: String,
    /// HTTP client for requests
    http_client: Client,
    /// Optional authentication token
    auth_token: Option<String>,
    /// Request timeout
    timeout: Duration,
}

impl OrchestratorClient {
    /// Discover and connect to an orchestrator via capability
    ///
    /// This uses the infant discovery system to find a service providing
    /// "orchestration" capability, without hardcoding orchestrator names.
    #[instrument(skip(discovery))]
    pub async fn discover(discovery: &dyn CapabilityDiscovery) -> Result<Self, OrchestratorError> {
        info!("Discovering orchestration capability");

        // Discover "orchestration" capability (could be Songbird, or any other orchestrator)
        let service = discovery
            .discover("orchestration")
            .await
            .map_err(|e| OrchestratorError::DiscoveryFailed(e.to_string()))?;

        debug!("Found orchestrator at: {}", service.endpoint);

        // Get auth token from environment if available
        let auth_token = std::env::var("TOADSTOOL_ORCHESTRATOR_AUTH_TOKEN").ok();

        // Get timeout from environment or use default
        let timeout_secs = std::env::var("TOADSTOOL_ORCHESTRATOR_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Self {
            endpoint: service.endpoint,
            http_client: Client::new(),
            auth_token,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    /// Create client with explicit endpoint (for testing or manual configuration)
    pub fn with_endpoint(endpoint: String) -> Self {
        let auth_token = std::env::var("TOADSTOOL_ORCHESTRATOR_AUTH_TOKEN").ok();
        let timeout_secs = std::env::var("TOADSTOOL_ORCHESTRATOR_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Self {
            endpoint,
            http_client: Client::new(),
            auth_token,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Register this service with the orchestrator
    #[instrument(skip(self, registration))]
    pub async fn register(
        &self,
        registration: ServiceRegistration,
    ) -> Result<RegistrationResponse, OrchestratorError> {
        info!("Registering service with orchestrator");

        let mut request = self
            .http_client
            .post(format!("{}/api/v1/register", self.endpoint))
            .json(&registration)
            .timeout(self.timeout);

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| OrchestratorError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OrchestratorError::RegistrationFailed(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let registration_response = response
            .json()
            .await
            .map_err(|e| OrchestratorError::InvalidResponse(e.to_string()))?;

        info!("Successfully registered with orchestrator");
        Ok(registration_response)
    }

    /// Update service health status
    #[instrument(skip(self))]
    pub async fn report_health(&self, health: HealthReport) -> Result<(), OrchestratorError> {
        debug!("Reporting health to orchestrator");

        let mut request = self
            .http_client
            .post(format!("{}/api/v1/health", self.endpoint))
            .json(&health)
            .timeout(self.timeout);

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| OrchestratorError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            warn!("Health report failed: {}", response.status());
        }

        Ok(())
    }

    /// Discover other services via orchestrator
    #[instrument(skip(self))]
    pub async fn discover_service(
        &self,
        capability: &str,
    ) -> Result<Vec<DiscoveredService>, OrchestratorError> {
        debug!("Discovering service with capability: {}", capability);

        let mut request = self
            .http_client
            .get(format!("{}/api/v1/discovery", self.endpoint))
            .query(&[("capability", capability)])
            .timeout(self.timeout);

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| OrchestratorError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OrchestratorError::DiscoveryFailed(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let services = response
            .json()
            .await
            .map_err(|e| OrchestratorError::InvalidResponse(e.to_string()))?;

        Ok(services)
    }

    /// Deregister service from orchestrator
    #[instrument(skip(self))]
    pub async fn deregister(&self, service_id: &str) -> Result<(), OrchestratorError> {
        info!("Deregistering service from orchestrator");

        let mut request = self
            .http_client
            .delete(format!("{}/api/v1/register/{}", self.endpoint, service_id))
            .timeout(self.timeout);

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| OrchestratorError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            warn!("Deregistration failed: {}", response.status());
        }

        Ok(())
    }
}

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Unique service identifier
    pub service_id: String,
    /// Service type identifier  
    pub service_type: String,
    /// Service version
    pub version: String,
    /// Instance identifier
    pub instance_id: String,
    /// Service capabilities
    pub capabilities: Vec<String>,
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Service tags for discovery
    pub tags: Vec<String>,
}

/// Service endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint type (http, grpc, websocket, etc.)
    pub endpoint_type: String,
    /// Endpoint URL
    pub url: String,
    /// Endpoint capabilities
    pub capabilities: Vec<String>,
    /// Protocol version
    pub protocol_version: String,
}

/// Registration response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    /// Assigned service ID
    pub service_id: String,
    /// Registration timestamp
    pub registered_at: DateTime<Utc>,
    /// Assigned endpoints
    pub assigned_endpoints: Vec<String>,
}

/// Health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Service ID
    pub service_id: String,
    /// Health status
    pub status: HealthStatus,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional metrics
    pub metrics: Option<HashMap<String, serde_json::Value>>,
}

/// Health status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Orchestrator client errors
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

impl From<DiscoveryError> for OrchestratorError {
    fn from(err: DiscoveryError) -> Self {
        OrchestratorError::DiscoveryFailed(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_registration_construction() {
        let registration = ServiceRegistration {
            service_id: "toadstool-test".to_string(),
            service_type: "compute".to_string(),
            version: "0.1.0".to_string(),
            instance_id: "instance-123".to_string(),
            capabilities: vec!["compute".to_string(), "native".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            tags: vec!["compute".to_string()],
        };

        assert_eq!(registration.service_id, "toadstool-test");
        assert_eq!(registration.capabilities.len(), 2);
    }

    #[test]
    fn test_health_status_variants() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    }
}
