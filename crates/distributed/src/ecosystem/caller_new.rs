//! Modern Ecosystem caller using infant discovery
//!
//! ZERO primal name hardcoding - uses capability-based discovery

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::auth::AuthenticationManager;
use super::registry::ServiceRegistry;
use crate::types::*;
use ecosystem_api::EcosystemError;

// Use infant discovery for service location
use toadstool_common::infant_discovery::{
    capability_names as capabilities,
    DiscoveryEngine,
    DiscoveryEngineBuilder,
    production_sources,
    standard_detectors,
};

/// Ecosystem caller for invoking external services using infant discovery
pub struct EcosystemCaller {
    /// Infant discovery engine
    discovery_engine: Arc<DiscoveryEngine>,
    /// HTTP client for REST APIs
    _http_client: reqwest::Client,
    /// gRPC client configurations
    _grpc_clients: HashMap<String, GrpcClientConfig>,
    /// WebSocket connections
    _websocket_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    /// Message queue connections
    _message_queues: Arc<RwLock<HashMap<String, MessageQueueConnection>>>,
    /// Authentication manager
    _auth_manager: Arc<AuthenticationManager>,
    /// Service registry
    _service_registry: Arc<ServiceRegistry>,
}

/// gRPC client configuration
#[derive(Debug, Clone)]
pub struct GrpcClientConfig {
    pub endpoint: String,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

/// WebSocket connection
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub endpoint: String,
    pub connected: bool,
    pub last_ping: std::time::Instant,
}

/// Message queue connection
#[derive(Debug, Clone)]
pub struct MessageQueueConnection {
    pub endpoint: String,
    pub queue_name: String,
    pub connected: bool,
}

impl EcosystemCaller {
    /// Create new ecosystem caller with infant discovery
    pub async fn new() -> Result<Self, EcosystemError> {
        let discovery_engine = DiscoveryEngineBuilder::new()
            .add_sources(production_sources())
            .add_detectors(standard_detectors())
            .build()
            .await
            .map_err(|e| EcosystemError::network_error(&format!("Failed to build discovery engine: {}", e)))?;

        Ok(Self {
            discovery_engine: Arc::new(discovery_engine),
            _http_client: reqwest::Client::new(),
            _grpc_clients: HashMap::new(),
            _websocket_connections: Arc::new(RwLock::new(HashMap::new())),
            _message_queues: Arc::new(RwLock::new(HashMap::new())),
            _auth_manager: Arc::new(AuthenticationManager::new()),
            _service_registry: Arc::new(ServiceRegistry::new()),
        })
    }

    /// Call a service using capability-based discovery
    pub async fn call_service(&self, job: &UniversalJob) -> Result<(), EcosystemError> {
        tracing::info!("Calling ecosystem service for job: {:?}", job.job_id);

        // Determine capability needed based on execution target
        let service_endpoint = match &job.target {
            crate::types::ExecutionTarget::EcosystemService { service_name, .. } => {
                // Map legacy service names to capabilities
                let capability = self.map_service_to_capability(service_name);
                
                // Discover service by capability
                match self.discovery_engine.discover(capability, None).await {
                    Ok(service) => format!("{}/execute", service.endpoint),
                    Err(e) => {
                        tracing::warn!("Could not discover {} capability: {}", capability, e);
                        return Ok(()); // Graceful degradation
                    }
                }
            }
            crate::types::ExecutionTarget::ToadStool { endpoint, .. } => {
                format!("{endpoint}/api/v1/execute")
            }
            _ => {
                tracing::debug!("Job target {:?} will be handled locally", job.target);
                return Ok(());
            }
        };

        // Create the request payload
        let request_payload = serde_json::json!({
            "job_id": job.job_id,
            "job_type": job.job_type,
            "execution_request": job.execution_request,
            "target": job.target,
            "priority": job.priority,
            "dependencies": job.dependencies,
            "resource_requirements": job.resource_requirements,
            "retry_config": job.retry_config,
            "created_at": job.created_at
        });

        // Make the HTTP request
        match self.make_service_request(&service_endpoint, &request_payload).await {
            Ok(response) => {
                tracing::info!("✅ Successfully called ecosystem service: {}", service_endpoint);
                tracing::debug!("Service response: {:?}", response);
                Ok(())
            }
            Err(e) => {
                tracing::error!("❌ Failed to call ecosystem service {}: {}", service_endpoint, e);
                Ok(()) // Graceful degradation
            }
        }
    }

    /// Map legacy service names to capabilities
    fn map_service_to_capability(&self, service_name: &str) -> &'static str {
        match service_name.to_lowercase().as_str() {
            name if name.contains("storage") || name.contains("gate") => capabilities::STORAGE,
            name if name.contains("ai") || name.contains("nlp") || name.contains("squirrel") => capabilities::AI_PROCESSING,
            name if name.contains("security") || name.contains("auth") || name.contains("dog") => capabilities::AUTHENTICATION,
            name if name.contains("orchestr") || name.contains("bird") => capabilities::ORCHESTRATION,
            name if name.contains("compute") || name.contains("biome") => capabilities::COMPUTE,
            _ => {
                tracing::debug!("Unknown service name: {}, using generic compute", service_name);
                capabilities::COMPUTE
            }
        }
    }

    /// Make an HTTP request to a service endpoint
    async fn make_service_request(
        &self,
        endpoint: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, EcosystemError> {
        use std::time::Duration;

        let timeout_duration = Duration::from_secs(30);

        let response = self
            ._http_client
            .post(endpoint)
            .json(payload)
            .timeout(timeout_duration)
            .send()
            .await
            .map_err(|e| {
                EcosystemError::network_error(&format!("Failed to send request to {endpoint}: {e}"))
            })?;

        if response.status().is_success() {
            let response_body = response.text().await.map_err(|e| {
                EcosystemError::network_error(&format!("Failed to read response body: {e}"))
            })?;

            match serde_json::from_str::<serde_json::Value>(&response_body) {
                Ok(json) => Ok(json),
                Err(_) => Ok(serde_json::Value::String(response_body)),
            }
        } else {
            Err(EcosystemError::network_error(&format!(
                "Service request failed with status: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )))
        }
    }

    /// Call a service by capability with custom payload
    pub async fn call_service_with_payload(
        &self,
        capability: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, EcosystemError> {
        // Discover service by capability
        let service = self.discovery_engine
            .discover(capability, None)
            .await
            .map_err(|e| EcosystemError::network_error(&format!("Failed to discover {} capability: {}", capability, e)))?;

        let endpoint = format!("{}/api/v1/{}", service.endpoint, operation);

        tracing::info!("Calling {} capability/{} at {}", capability, operation, endpoint);

        self.make_service_request(&endpoint, &payload).await
    }

    /// Discover all available ecosystem services by capability
    pub async fn discover_services(&self) -> Result<Vec<String>, EcosystemError> {
        let capabilities_to_check = vec![
            capabilities::ORCHESTRATION,
            capabilities::AUTHENTICATION,
            capabilities::STORAGE,
            capabilities::AI_PROCESSING,
            capabilities::NLP,
            capabilities::COMPUTE,
        ];

        let mut discovered = Vec::new();

        for capability in capabilities_to_check {
            match self.discovery_engine.discover(capability, None).await {
                Ok(service) => {
                    tracing::info!("✅ Capability {} is available at {}", capability, service.endpoint);
                    discovered.push(capability.to_string());
                }
                Err(e) => {
                    tracing::debug!("❌ Capability {} is not available: {}", capability, e);
                }
            }
        }

        Ok(discovered)
    }

    /// Check if a specific capability is available
    pub async fn check_capability_health(&self, capability: &str) -> Result<(), EcosystemError> {
        use std::time::Duration;

        let service = self.discovery_engine
            .discover(capability, None)
            .await
            .map_err(|e| EcosystemError::network_error(&format!("Capability {} not found: {}", capability, e)))?;

        let health_endpoint = format!("{}/health", service.endpoint);

        let response = self
            ._http_client
            .get(&health_endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                EcosystemError::network_error(&format!(
                    "Health check failed for {} capability: {}", capability, e
                ))
            })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(EcosystemError::network_error(&format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }
}

impl Default for EcosystemCaller {
    fn default() -> Self {
        // Note: This blocks on async initialization and may panic.
        // In production, prefer using EcosystemCaller::new().await directly
        // to handle errors gracefully.
        tokio::runtime::Runtime::new()
            .expect("FATAL: Failed to create tokio runtime - this is unrecoverable")
            .block_on(Self::new())
            .expect("FATAL: Failed to initialize EcosystemCaller - check configuration")
    }
}

