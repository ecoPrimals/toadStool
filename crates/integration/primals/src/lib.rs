// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate
)]

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

use async_trait::async_trait;
use toadstool::ToadStoolResult;

mod health;
mod integration_manifest;
mod manager;
mod messaging;
mod primal_types;
mod service;

// Re-exports for backward compatibility - all public types accessible from crate root
pub use health::{HealthCheck, HealthCheckStatus, HealthStatus};
pub use integration_manifest::{BiomeManifest, BiomeMetadata};
pub use manager::{
    BootstrapResult, PrimalBootstrapResult, PrimalIntegrationConfig, PrimalIntegrationManager,
};
pub use messaging::{PrimalMessage, PrimalMessageType, PrimalMetrics};
pub use primal_types::{GpuAllocation, PrimalConfig, PrimalResources, PrimalType};
pub use service::{ServiceEndpoint, ServiceRegistration, StartupResult, StartupStatus};

/// Universal trait for Primal integration
///
/// This is the canonical definition of the `PrimalIntegration` trait.
/// All Primals in the ecoPrimals ecosystem should implement this trait.
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use uuid::Uuid;

    use super::*;
    use toadstool::ToadStoolError;

    struct MockPrimal {
        name: String,
        should_fail: bool,
    }

    // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
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
                duration: std::time::Duration::from_millis(100),
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
                last_check: std::time::SystemTime::now(),
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
                timestamp: std::time::SystemTime::now(),
            })
        }

        async fn handle_primal_message(
            &self,
            message: &PrimalMessage,
        ) -> ToadStoolResult<PrimalMessage> {
            Ok(message.clone())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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
                        primal_type: PrimalType::Compute,
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
