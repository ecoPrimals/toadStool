// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test double for [`crate::PrimalIntegration`] — only built for `cfg(test)` or `test-mocks`.

use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    BiomeManifest, HealthStatus, PrimalConfig, PrimalIntegration, PrimalMessage, PrimalMetrics,
    ServiceRegistration, StartupResult, StartupStatus,
};
use toadstool::{ToadStoolError, ToadStoolResult};

/// Minimal mock Primal for unit tests and integration tests that enable `test-mocks`.
pub struct MockPrimal {
    /// Logical service name returned in registrations and startup results.
    pub name: String,
    /// When true, `initialize_from_manifest` fails (error-path coverage).
    pub should_fail: bool,
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
