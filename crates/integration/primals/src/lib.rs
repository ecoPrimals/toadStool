//! # Universal Primal Integration Framework
//!
//! This module provides a consistent interface for integrating with all Primals
//! in the ecoPrimals ecosystem. It defines the `PrimalIntegration` trait and
//! common types for universal orchestration from biome.yaml manifests.
//!
//! ## Supported Primals
//!
//! - **`ToadStool`**: Universal Compute Platform
//! - **Songbird**: Network Coordination and Service Mesh
//! - **`BearDog`**: Security and Authentication
//! - **`NestGate`**: Storage and Data Management
//! - **Squirrel**: AI Agents and Model Control Protocol
//! - **biomeOS**: Universal Operating System

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use toadstool::{ToadStoolError, ToadStoolResult};

/// Universal trait for Primal integration
/// 
/// This is the canonical definition of the PrimalIntegration trait.
/// All Primals in the ecoPrimals ecosystem should implement this trait.
#[async_trait]
pub trait PrimalIntegration: Send + Sync {
    /// Initialize the Primal from manifest configuration
    async fn initialize_from_manifest(&self, config: &PrimalConfig) -> ToadStoolResult<()>;

    /// Register with orchestrator via capability discovery
    async fn register_with_orchestrator(
        &self,
        discovery: &dyn toadstool_common::infant_discovery::CapabilityDiscovery,
    ) -> ToadStoolResult<ServiceRegistration>;

    /// Validate dependencies before startup
    async fn validate_dependencies(&self, manifest: &BiomeManifest) -> ToadStoolResult<()>;

    /// Start Primal services
    async fn start_services(&self) -> ToadStoolResult<StartupResult>;

    /// Shutdown Primal services gracefully
    async fn shutdown(&self) -> ToadStoolResult<()>;

    /// Get current health status
    async fn get_health_status(&self) -> ToadStoolResult<HealthStatus>;

    /// Get Primal capabilities
    async fn get_capabilities(&self) -> ToadStoolResult<Vec<String>>;

    /// Handle configuration updates
    async fn update_configuration(&self, config: &PrimalConfig) -> ToadStoolResult<()>;

    /// Get metrics and monitoring data
    async fn get_metrics(&self) -> ToadStoolResult<PrimalMetrics>;

    /// Handle inter-Primal communication
    async fn handle_primal_message(
        &self,
        message: &PrimalMessage,
    ) -> ToadStoolResult<PrimalMessage>;
}

/// Generic configuration for any Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalConfig {
    /// Primal name
    pub name: String,
    /// Primal type
    pub primal_type: PrimalType,
    /// Enable flag
    pub enabled: bool,
    /// Resource allocation
    pub resources: Option<PrimalResources>,
    /// Dependencies on other Primals
    pub dependencies: Vec<String>,
    /// Custom configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Labels for metadata
    pub labels: HashMap<String, String>,
    /// Annotations for additional metadata
    pub annotations: HashMap<String, String>,
}

/// Types of Primals in the ecosystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PrimalType {
    /// `ToadStool` - Universal Compute
    ToadStool,
    /// Songbird - Network Coordination
    Songbird,
    /// `BearDog` - Security
    BearDog,
    /// `NestGate` - Storage
    NestGate,
    /// Squirrel - AI
    Squirrel,
    /// biomeOS - Universal OS
    BiomeOS,
    /// Custom Primal
    Custom(String),
}

/// Resource allocation for a Primal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalResources {
    /// CPU cores allocation
    pub cpu_cores: Option<f64>,
    /// Memory allocation in GB
    pub memory_gb: Option<f64>,
    /// Storage allocation in GB
    pub storage_gb: Option<f64>,
    /// GPU allocation
    pub gpu: Option<GpuAllocation>,
    /// Network bandwidth limit
    pub network_bandwidth: Option<String>,
    /// Custom resource limits
    pub custom_limits: HashMap<String, serde_json::Value>,
}

/// GPU allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Number of GPUs
    pub count: u32,
    /// GPU type preference
    pub gpu_type: Option<String>,
    /// Memory per GPU in GB
    pub memory_gb: Option<f64>,
    /// CUDA compute capability
    pub cuda_capability: Option<String>,
}

/// Service registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistration {
    /// Service ID
    pub service_id: Uuid,
    /// Service name
    pub service_name: String,
    /// Service endpoints
    pub endpoints: Vec<ServiceEndpoint>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
    /// Health check endpoint
    pub health_endpoint: Option<String>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Endpoint name
    pub name: String,
    /// Protocol (http, https, grpc, etc.)
    pub protocol: String,
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Base path
    pub path: Option<String>,
}

/// Startup result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupResult {
    /// Startup duration
    pub duration: Duration,
    /// Services started
    pub services_started: Vec<String>,
    /// Startup logs
    pub logs: Vec<String>,
    /// Startup status
    pub status: StartupStatus,
}

/// Startup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartupStatus {
    /// Startup successful
    Success,
    /// Startup failed
    Failed(String),
    /// Startup partially successful
    Partial(Vec<String>),
}

/// Health status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health
    pub healthy: bool,
    /// Health checks
    pub checks: Vec<HealthCheck>,
    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Individual health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthCheckStatus,
    /// Check message
    pub message: Option<String>,
    /// Check duration
    pub duration: Duration,
}

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthCheckStatus {
    /// Check passed
    Healthy,
    /// Check failed
    Unhealthy,
    /// Check in progress
    Pending,
}

/// Primal metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in GB
    pub memory_usage: f64,
    /// Storage usage in GB
    pub storage_usage: f64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, serde_json::Value>,
    /// Metrics timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Inter-Primal communication message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalMessage {
    /// Message ID
    pub id: Uuid,
    /// Source Primal
    pub from: String,
    /// Destination Primal
    pub to: String,
    /// Message type
    pub message_type: PrimalMessageType,
    /// Message payload
    pub payload: serde_json::Value,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Message headers
    pub headers: HashMap<String, String>,
}

/// Types of inter-Primal messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalMessageType {
    /// Configuration update
    ConfigUpdate,
    /// Resource request
    ResourceRequest,
    /// Resource response
    ResourceResponse,
    /// Health check
    HealthCheck,
    /// Metrics request
    MetricsRequest,
    /// Metrics response
    MetricsResponse,
    /// Service discovery
    ServiceDiscovery,
    /// Authentication token
    AuthToken,
    /// Custom message
    Custom(String),
}

/// Biome manifest reference (simplified for this module)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeManifest {
    /// API version
    pub api_version: String,
    /// Manifest kind
    pub kind: String,
    /// Biome metadata
    pub metadata: BiomeMetadata,
    /// Primal configurations
    pub primals: HashMap<String, PrimalConfig>,
}

/// Biome metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeMetadata {
    /// Biome name
    pub name: String,
    /// Biome version
    pub version: String,
    /// Environment
    pub environment: Option<String>,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Primal Integration Manager
pub struct PrimalIntegrationManager {
    /// Registered Primals
    primals: HashMap<String, Box<dyn PrimalIntegration + Send + Sync>>,
    /// Configuration
    _config: PrimalIntegrationConfig,
}

/// Configuration for Primal Integration Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalIntegrationConfig {
    /// Auto-discovery enabled
    pub auto_discovery: bool,
    /// Discovery timeout
    pub discovery_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for PrimalIntegrationConfig {
    fn default() -> Self {
        Self {
            auto_discovery: true,
            discovery_timeout: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(30),
            max_retry_attempts: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

impl PrimalIntegrationManager {
    /// Create a new Primal Integration Manager
    #[must_use]
    pub fn new(config: PrimalIntegrationConfig) -> Self {
        Self {
            primals: HashMap::new(),
            _config: config,
        }
    }

    /// Register a Primal implementation
    pub fn register_primal(
        &mut self,
        name: String,
        primal: Box<dyn PrimalIntegration + Send + Sync>,
    ) {
        self.primals.insert(name, primal);
    }

    /// Bootstrap all Primals from manifest
    pub async fn bootstrap_from_manifest(
        &self,
        manifest: &BiomeManifest,
    ) -> ToadStoolResult<BootstrapResult> {
        let start_time = std::time::Instant::now();
        let mut results = HashMap::new();

        // Phase 1: Validate all Primal configurations
        for name in manifest.primals.keys() {
            if let Some(primal) = self.primals.get(name) {
                if let Err(e) = primal.validate_dependencies(manifest).await {
                    results.insert(name.clone(), PrimalBootstrapResult::Failed(e.to_string()));
                    continue;
                }
            }
        }

        // Phase 2: Initialize Primals in dependency order
        let startup_order = self.calculate_startup_order(manifest)?;
        for primal_name in &startup_order {
            if let Some(primal) = self.primals.get(primal_name as &str) {
                if let Some(config) = manifest.primals.get(primal_name as &str) {
                    match primal.initialize_from_manifest(config).await {
                        Ok(()) => {
                            results.insert(primal_name.clone(), PrimalBootstrapResult::Success);
                        }
                        Err(e) => {
                            results.insert(
                                primal_name.clone(),
                                PrimalBootstrapResult::Failed(e.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Phase 3: Start services
        for primal_name in &startup_order {
            if let Some(primal) = self.primals.get(primal_name as &str) {
                if results.get(primal_name as &str) == Some(&PrimalBootstrapResult::Success) {
                    match primal.start_services().await {
                        Ok(startup_result) => {
                            if startup_result.status == StartupStatus::Success {
                                results.insert(primal_name.clone(), PrimalBootstrapResult::Running);
                            } else {
                                results.insert(
                                    primal_name.clone(),
                                    PrimalBootstrapResult::Failed(
                                        "Service startup failed".to_string(),
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            results.insert(
                                primal_name.clone(),
                                PrimalBootstrapResult::Failed(e.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Phase 4: Register with Songbird
        // Registration with orchestrator moved to separate phase
        // Each primal will handle its own registration via capability discovery
        for primal_name in &startup_order {
            if results.get(primal_name as &str) == Some(&PrimalBootstrapResult::Running) {
                tracing::info!("Primal {} started successfully", primal_name);
            }
        }

        let successful_primals = results
            .values()
            .filter(|r| matches!(r, PrimalBootstrapResult::Running))
            .count();

        Ok(BootstrapResult {
            duration: start_time.elapsed(),
            results,
            total_primals: manifest.primals.len(),
            successful_primals,
        })
    }

    /// Calculate the startup order based on dependencies
    fn calculate_startup_order(&self, manifest: &BiomeManifest) -> ToadStoolResult<Vec<String>> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        // Topological sort to resolve dependencies
        for primal_name in manifest.primals.keys() {
            if !visited.contains(primal_name) {
                self.visit_primal(
                    primal_name,
                    manifest,
                    &mut visited,
                    &mut visiting,
                    &mut order,
                )?;
            }
        }

        Ok(order)
    }

    /// Visit a Primal during topological sort
    #[allow(clippy::only_used_in_recursion)]
    fn visit_primal(
        &self,
        primal_name: &str,
        manifest: &BiomeManifest,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) -> ToadStoolResult<()> {
        if visiting.contains(primal_name) {
            return Err(crate::ToadStoolError::runtime(format!(
                "Circular dependency detected: {primal_name}"
            )));
        }

        if visited.contains(primal_name) {
            return Ok(());
        }

        visiting.insert(primal_name.to_string());

        if let Some(config) = manifest.primals.get(primal_name) {
            for dependency in &config.dependencies {
                self.visit_primal(dependency, manifest, visited, visiting, order)?;
            }
        }

        visiting.remove(primal_name);
        visited.insert(primal_name.to_string());
        order.push(primal_name.to_string());

        Ok(())
    }

    /// Get health status for all Primals
    pub async fn get_all_health_status(&self) -> HashMap<String, HealthStatus> {
        let mut statuses = HashMap::new();

        for (name, primal) in &self.primals {
            match primal.get_health_status().await {
                Ok(status) => {
                    statuses.insert(name.clone(), status);
                }
                Err(e) => {
                    tracing::error!("Failed to get health status for {}: {}", name, e);
                    statuses.insert(
                        name.clone(),
                        HealthStatus {
                            healthy: false,
                            checks: vec![HealthCheck {
                                name: "system".to_string(),
                                status: HealthCheckStatus::Unhealthy,
                                message: Some(e.to_string()),
                                duration: Duration::from_millis(0),
                            }],
                            last_check: chrono::Utc::now(),
                        },
                    );
                }
            }
        }

        statuses
    }

    /// Shutdown all Primals gracefully
    pub async fn shutdown_all(&self) -> ToadStoolResult<()> {
        for (name, primal) in &self.primals {
            if let Err(e) = primal.shutdown().await {
                tracing::error!("Failed to shutdown {}: {}", name, e);
            }
        }
        Ok(())
    }
}

/// Result of bootstrapping Primals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    /// Total bootstrap duration
    pub duration: Duration,
    /// Individual Primal results
    pub results: HashMap<String, PrimalBootstrapResult>,
    /// Total number of Primals
    pub total_primals: usize,
    /// Number of successfully started Primals
    pub successful_primals: usize,
}

/// Result of bootstrapping a single Primal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimalBootstrapResult {
    /// Not started
    NotStarted,
    /// Successfully initialized
    Success,
    /// Successfully running
    Running,
    /// Failed with error
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockPrimal {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl PrimalIntegration for MockPrimal {
        async fn initialize_from_manifest(&self, _config: &PrimalConfig) -> ToadStoolResult<()> {
            if self.should_fail {
                Err(ToadStoolError::runtime("Mock failure".to_string()))
            } else {
                Ok(())
            }
        }

        async fn register_with_orchestrator(
            &self,
            _discovery: &dyn toadstool_common::infant_discovery::CapabilityDiscovery,
        ) -> ToadStoolResult<ServiceRegistration> {
            // Mock implementation - uses capability discovery to find orchestrator
            Ok(ServiceRegistration {
                service_id: Uuid::new_v4(),
                service_name: self.name.clone(),
                endpoints: vec![],
                metadata: HashMap::new(),
                health_endpoint: None,
            })
        }

        async fn validate_dependencies(&self, _manifest: &BiomeManifest) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn start_services(&self) -> ToadStoolResult<StartupResult> {
            Ok(StartupResult {
                duration: Duration::from_millis(100),
                services_started: vec![self.name.clone()],
                logs: vec![],
                status: StartupStatus::Success,
            })
        }

        async fn shutdown(&self) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn get_health_status(&self) -> ToadStoolResult<HealthStatus> {
            Ok(HealthStatus {
                healthy: true,
                checks: vec![],
                last_check: chrono::Utc::now(),
            })
        }

        async fn get_capabilities(&self) -> ToadStoolResult<Vec<String>> {
            Ok(vec!["test".to_string()])
        }

        async fn update_configuration(&self, _config: &PrimalConfig) -> ToadStoolResult<()> {
            Ok(())
        }

        async fn get_metrics(&self) -> ToadStoolResult<PrimalMetrics> {
            Ok(PrimalMetrics {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                storage_usage: 0.0,
                network_bytes_sent: 0,
                network_bytes_received: 0,
                custom_metrics: HashMap::new(),
                timestamp: chrono::Utc::now(),
            })
        }

        async fn handle_primal_message(
            &self,
            message: &PrimalMessage,
        ) -> ToadStoolResult<PrimalMessage> {
            Ok(message.clone())
        }
    }

    #[tokio::test]
    async fn test_primal_integration_manager() {
        let mut manager = PrimalIntegrationManager::new(PrimalIntegrationConfig::default());

        let mock_primal = MockPrimal {
            name: "test".to_string(),
            should_fail: false,
        };

        manager.register_primal("test".to_string(), Box::new(mock_primal));

        let manifest = BiomeManifest {
            api_version: "biomeOS/v1".to_string(),
            kind: "Biome".to_string(),
            metadata: BiomeMetadata {
                name: "test-biome".to_string(),
                version: "1.0.0".to_string(),
                environment: None,
                labels: HashMap::new(),
            },
            primals: {
                let mut primals = HashMap::new();
                primals.insert(
                    "test".to_string(),
                    PrimalConfig {
                        name: "test".to_string(),
                        primal_type: PrimalType::Custom("test".to_string()),
                        enabled: true,
                        resources: None,
                        dependencies: vec![],
                        config: HashMap::new(),
                        environment: HashMap::new(),
                        labels: HashMap::new(),
                        annotations: HashMap::new(),
                    },
                );
                primals
            },
        };

        let result = manager.bootstrap_from_manifest(&manifest).await.unwrap();
        assert_eq!(result.successful_primals, 1);
        assert_eq!(result.total_primals, 1);
    }
}
