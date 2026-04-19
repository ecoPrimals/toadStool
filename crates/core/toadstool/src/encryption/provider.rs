// SPDX-License-Identifier: AGPL-3.0-or-later
//! Crypto provider interface and registry
//!
//! **Design Philosophy**:
//! - Trait-based: Any primal can provide crypto services
//! - Discovery: Runtime registration and lookup
//! - No hardcoding: Providers announce capabilities, consumers discover

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{ToadStoolError, ToadStoolResult};

use super::{CryptoCapability, EncryptedPayload, EncryptionKey, EncryptionMetadata, SecurityLevel};

/// Trait for crypto service providers
///
/// **Design**: Any primal implementing this can provide crypto
/// (security service, external HSM, cloud KMS, etc.)
pub trait CryptoProvider: Send + Sync {
    /// Get provider identifier (discovered at runtime)
    fn provider_id(&self) -> &str;

    /// Get provider capabilities
    fn capabilities(&self) -> &CryptoCapability;

    /// Encrypt data
    fn encrypt<'a>(
        &'a self,
        data: &'a [u8],
        key: &'a EncryptionKey,
    ) -> impl Future<Output = ToadStoolResult<(EncryptedPayload, EncryptionMetadata)>> + Send + 'a;

    /// Decrypt data
    fn decrypt<'a>(
        &'a self,
        encrypted: &'a EncryptedPayload,
        key: &'a EncryptionKey,
        metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a;

    /// Generate new encryption key
    fn generate_key(
        &self,
        security_level: SecurityLevel,
    ) -> impl Future<Output = ToadStoolResult<EncryptionKey>> + Send + '_;

    /// Get existing key by ID
    fn get_key<'a>(
        &'a self,
        key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<EncryptionKey>> + Send + 'a;

    /// Check if provider is healthy and reachable
    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_;
}

/// Placeholder crypto provider used as the default type parameter when no
/// vendor implementation is registered. All methods return
/// [`ToadStoolError::configuration`] with capability-based guidance.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCryptoProvider;

const NOOP_MSG: &str =
    "no crypto provider registered; register a provider via crypto.provider.register capability";

impl CryptoProvider for NoopCryptoProvider {
    fn provider_id(&self) -> &'static str {
        "noop"
    }

    fn capabilities(&self) -> &CryptoCapability {
        static CAP: std::sync::OnceLock<CryptoCapability> = std::sync::OnceLock::new();
        CAP.get_or_init(|| CryptoCapability {
            algorithms: vec![],
            security_level: SecurityLevel::Standard,
            hardware_backed: false,
        })
    }

    fn encrypt<'a>(
        &'a self,
        _data: &'a [u8],
        _key: &'a EncryptionKey,
    ) -> impl Future<Output = ToadStoolResult<(EncryptedPayload, EncryptionMetadata)>> + Send + 'a
    {
        async { Err(ToadStoolError::configuration(NOOP_MSG)) }
    }

    fn decrypt<'a>(
        &'a self,
        _encrypted: &'a EncryptedPayload,
        _key: &'a EncryptionKey,
        _metadata: &'a EncryptionMetadata,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a {
        async { Err(ToadStoolError::configuration(NOOP_MSG)) }
    }

    fn generate_key(
        &self,
        _security_level: SecurityLevel,
    ) -> impl Future<Output = ToadStoolResult<EncryptionKey>> + Send + '_ {
        async { Err(ToadStoolError::configuration(NOOP_MSG)) }
    }

    fn get_key<'a>(
        &'a self,
        _key_id: &'a str,
    ) -> impl Future<Output = ToadStoolResult<EncryptionKey>> + Send + 'a {
        async { Err(ToadStoolError::configuration(NOOP_MSG)) }
    }

    fn health_check(&self) -> impl Future<Output = ToadStoolResult<ProviderHealth>> + Send + '_ {
        async { Ok(ProviderHealth::unhealthy(NOOP_MSG)) }
    }
}

/// Provider health status
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    /// Is provider reachable?
    pub available: bool,

    /// Response latency (milliseconds)
    pub latency_ms: u64,

    /// Current load (0.0 - 1.0)
    pub load: f32,

    /// Error message if unavailable
    pub error: Option<String>,
}

impl ProviderHealth {
    /// Creates a healthy provider status with latency.
    pub const fn healthy(latency_ms: u64) -> Self {
        Self {
            available: true,
            latency_ms,
            load: 0.0,
            error: None,
        }
    }

    /// Creates an unhealthy provider status with error message.
    pub fn unhealthy(error: impl Into<String>) -> Self {
        Self {
            available: false,
            latency_ms: 0,
            load: 0.0,
            error: Some(error.into()),
        }
    }
}

/// Registry of available crypto providers
///
/// **Design**: Central discovery point, no hardcoded providers
pub struct CryptoProviderRegistry<P: CryptoProvider> {
    providers: Arc<RwLock<HashMap<String, Arc<P>>>>,
}

impl<P: CryptoProvider> CryptoProviderRegistry<P> {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a crypto provider
    ///
    /// **Design**: Providers announce themselves, no pre-configuration
    ///
    /// # Errors
    ///
    /// Returns error if the provider ID is already registered.
    pub async fn register(&self, provider: Arc<P>) -> ToadStoolResult<()> {
        let provider_id = provider.provider_id().to_string();
        let mut providers = self.providers.write().await;

        // Check if already registered
        if providers.contains_key(&provider_id) {
            return Err(ToadStoolError::configuration(format!(
                "Provider {provider_id} already registered"
            )));
        }

        providers.insert(provider_id, provider);
        drop(providers);
        Ok(())
    }

    /// Unregister a provider
    ///
    /// # Errors
    ///
    /// Returns error if `provider_id` is not registered.
    pub async fn unregister(&self, provider_id: &str) -> ToadStoolResult<()> {
        self.providers
            .write()
            .await
            .remove(provider_id)
            .ok_or_else(|| {
                ToadStoolError::not_found(format!("Provider {provider_id} not found"))
            })?;
        Ok(())
    }

    /// Find provider matching capability
    ///
    /// **Design**: Capability-based lookup, returns best match
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future failures.
    pub async fn find_provider(
        &self,
        capability: &CryptoCapability,
    ) -> ToadStoolResult<Option<Arc<P>>> {
        // Find all matching providers
        let mut matches: Vec<(u32, Arc<P>)> = self
            .providers
            .read()
            .await
            .values()
            .filter(|p| p.capabilities().matches(capability))
            .map(|p| {
                let score = p.capabilities().match_score(capability);
                (score, Arc::clone(p))
            })
            .collect();

        if matches.is_empty() {
            return Ok(None);
        }

        // Sort by score (best first)
        matches.sort_by(|a, b| b.0.cmp(&a.0));

        // Return best match
        Ok(Some(matches[0].1.clone()))
    }

    /// Find all providers matching capability
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`; the `Result` type is reserved for future failures.
    pub async fn find_all_providers(
        &self,
        capability: &CryptoCapability,
    ) -> ToadStoolResult<Vec<Arc<P>>> {
        let matches: Vec<Arc<P>> = self
            .providers
            .read()
            .await
            .values()
            .filter(|p| p.capabilities().matches(capability))
            .map(Arc::clone)
            .collect();

        Ok(matches)
    }

    /// Get provider by ID
    ///
    /// # Errors
    ///
    /// Returns error if `provider_id` is not registered.
    pub async fn get_provider(&self, provider_id: &str) -> ToadStoolResult<Arc<P>> {
        self.providers
            .read()
            .await
            .get(provider_id)
            .map(Arc::clone)
            .ok_or_else(|| ToadStoolError::not_found(format!("Provider {provider_id} not found")))
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Check health of all providers
    pub async fn health_check_all(&self) -> HashMap<String, ProviderHealth> {
        let to_check: Vec<(String, Arc<P>)> = self
            .providers
            .read()
            .await
            .iter()
            .map(|(id, p)| (id.clone(), Arc::clone(p)))
            .collect();

        let mut health_map = HashMap::new();
        for (id, provider) in to_check {
            let health = provider
                .health_check()
                .await
                .unwrap_or_else(|e| ProviderHealth::unhealthy(e.to_string()));
            health_map.insert(id, health);
        }

        health_map
    }
}

impl<P: CryptoProvider> Default for CryptoProviderRegistry<P> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "provider_tests.rs"]
mod tests;
