// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use super::super::capability_types::HealthStatus;
use super::ProviderRegistry;

impl ProviderRegistry {
    /// Create a new provider registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Update provider health status
    pub fn update_health(&mut self, provider_id: &str, health: HealthStatus) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.info.health = health;
            provider.last_health_check = Some(Instant::now());
        }
    }

    /// Record a successful request to a provider
    pub fn record_success(&mut self, provider_id: &str) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.request_count += 1;
        }
    }

    /// Record a failed request to a provider
    pub fn record_failure(&mut self, provider_id: &str) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.request_count += 1;
            provider.failure_count += 1;
        }
    }

    /// Clear all providers
    pub fn clear(&mut self) {
        self.providers.clear();
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
