// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::must_use_candidate,
    clippy::match_same_arms,
    clippy::doc_markdown
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
//! - **Coordination**: Network Coordination and Service Mesh
//! - **`Security`**: Security and Authentication
//! - **`Storage`**: Storage and Data Management
//! - **Intelligence**: AI Agents and Model Control Protocol
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

#[cfg(any(test, feature = "test-mocks"))]
pub mod mock_primal;

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

    use super::mock_primal::MockPrimal;
    use super::*;

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
