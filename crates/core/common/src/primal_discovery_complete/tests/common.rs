// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers for primal discovery complete tests

use super::super::*;

pub(super) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Mock `DiscoveryClient` for testing mDNS code paths without network
pub(super) struct MockDiscoveryClient {
    pub services: std::sync::RwLock<Option<Vec<DiscoveredService>>>,
    pub error: std::sync::RwLock<Option<String>>,
}

impl crate::runtime_discovery::DiscoveryClient for MockDiscoveryClient {
    fn discover_by_capability<'a>(
        &'a self,
        _capability: &'a Capability,
    ) -> impl std::future::Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a
    {
        async move {
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
    }

    fn discover_all<'a>(
        &'a self,
    ) -> impl std::future::Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a
    {
        async move { Ok(vec![]) }
    }

    fn register_service<'a>(
        &'a self,
        _service: &'a DiscoveredService,
    ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move { Ok(()) }
    }

    fn deregister_service<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move { Ok(()) }
    }

    fn health_check<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> impl std::future::Future<Output = ToadStoolResult<bool>> + Send + 'a {
        async move { Ok(true) }
    }
}
