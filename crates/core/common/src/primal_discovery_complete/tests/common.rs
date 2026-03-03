// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers for primal discovery complete tests

use super::super::*;
use async_trait::async_trait;

pub(super) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Mock DiscoveryClient for testing mDNS code paths without network
pub(super) struct MockDiscoveryClient {
    pub services: std::sync::RwLock<Option<Vec<DiscoveredService>>>,
    pub error: std::sync::RwLock<Option<String>>,
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl crate::runtime_discovery::DiscoveryClient for MockDiscoveryClient {
    async fn discover_by_capability(
        &self,
        _capability: &Capability,
    ) -> ToadStoolResult<Vec<DiscoveredService>> {
        if let Ok(guard) = self.error.read() {
            if let Some(ref msg) = *guard {
                return Err(ToadStoolError::runtime(msg.clone()));
            }
        }
        if let Ok(guard) = self.services.read() {
            if let Some(ref svcs) = *guard {
                return Ok(svcs.clone());
            }
        }
        Ok(vec![])
    }

    async fn discover_all(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        Ok(vec![])
    }

    async fn register_service(&self, _service: &DiscoveredService) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn deregister_service(&self, _service_id: &str) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn health_check(&self, _service_id: &str) -> ToadStoolResult<bool> {
        Ok(true)
    }
}
