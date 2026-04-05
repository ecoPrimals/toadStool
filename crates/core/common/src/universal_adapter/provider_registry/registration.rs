// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Instant;

use super::super::capability_types::CapabilityInfo;
use super::{ProviderRegistry, RegisteredProvider};
use crate::ToadStoolResult;

impl ProviderRegistry {
    /// Register a capability provider
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn register(&mut self, info: CapabilityInfo) -> ToadStoolResult<()> {
        let provider_id = info.provider_id.clone();

        let registered = RegisteredProvider {
            info,
            registered_at: Instant::now(),
            last_health_check: None,
            request_count: 0,
            failure_count: 0,
        };

        self.providers.insert(provider_id, registered);
        Ok(())
    }

    /// Unregister a provider
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn unregister(&mut self, provider_id: &str) -> ToadStoolResult<()> {
        self.providers.remove(provider_id);
        Ok(())
    }
}
