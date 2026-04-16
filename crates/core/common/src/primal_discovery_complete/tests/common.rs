// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers for primal discovery complete tests

use std::future::Future;
use std::pin::Pin;

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
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }

    fn discover_all<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<DiscoveredService>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![]) })
    }

    fn register_service<'a>(
        &'a self,
        _service: &'a DiscoveredService,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn deregister_service<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn health_check<'a>(
        &'a self,
        _service_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>> {
        Box::pin(async move { Ok(true) })
    }
}
