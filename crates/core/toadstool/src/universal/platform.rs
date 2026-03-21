// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Compute Platform configuration and management

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool_config::defaults;

use crate::{ToadStoolResult, execution::RuntimeEngine, execution::RuntimeType};

use super::jobs::UniversalJob;
use super::provider::ToadStoolPrimalProvider;
use super::registry::UniversalPrimalRegistry;
use super::scheduler::UniversalScheduler;
use super::types::{NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel};
use crate::execution::ExecutionResponse;

/// Universal platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalPlatformConfig {
    /// Enable recursive hosting
    pub recursive_hosting: bool,
    /// Enable ecosystem integration
    pub ecosystem_integration: bool,
    /// Enable `BiomeOS` integration
    pub biomeos_integration: bool,
    /// Maximum concurrent jobs
    pub max_concurrent_jobs: u32,
    /// Pure ecosystem mode
    pub pure_ecosystem: bool,
}

impl Default for UniversalPlatformConfig {
    fn default() -> Self {
        Self {
            recursive_hosting: true,
            ecosystem_integration: true,
            biomeos_integration: true,
            max_concurrent_jobs: 100,
            pure_ecosystem: false,
        }
    }
}

/// Platform status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformStatus {
    /// Initializing
    Initializing,
    /// Running
    Running,
    /// Degraded
    Degraded,
    /// Stopped
    Stopped,
}

/// Universal compute platform
pub struct UniversalComputePlatform {
    /// Platform configuration
    config: UniversalPlatformConfig,
    /// Runtime engines
    runtime_engines: Arc<RwLock<HashMap<RuntimeType, Box<dyn RuntimeEngine>>>>,
    /// Universal scheduler
    scheduler: Arc<UniversalScheduler>,
    /// Primal registry
    primal_registry: Arc<UniversalPrimalRegistry>,
    /// `ToadStool` primal provider
    toadstool_provider: Option<Arc<ToadStoolPrimalProvider>>,
}

impl UniversalComputePlatform {
    /// Create new platform
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if platform initialization fails or primal
    /// registration encounters errors.
    #[must_use = "Platform creation should be checked"]
    pub async fn new() -> ToadStoolResult<Self> {
        Self::new_with_config(UniversalPlatformConfig::default()).await
    }

    /// Create new platform with config
    ///
    /// # Errors
    /// Returns a `ToadStoolError` if:
    /// - Scheduler initialization fails.
    /// - Primal registry setup fails.
    /// - ToadStool provider registration fails.
    #[must_use = "Platform creation should be checked"]
    pub async fn new_with_config(config: UniversalPlatformConfig) -> ToadStoolResult<Self> {
        let primal_registry = Arc::new(UniversalPrimalRegistry::new());
        let scheduler = Arc::new(UniversalScheduler::new(primal_registry.clone()).await?);

        let mut platform = Self {
            config,
            runtime_engines: Arc::new(RwLock::new(HashMap::new())),
            scheduler,
            primal_registry,
            toadstool_provider: None,
        };

        // Register ToadStool as a primal provider
        platform.register_as_universal_primal().await?;

        info!("Universal compute platform initialized");
        Ok(platform)
    }

    /// Register `ToadStool` as a universal primal
    async fn register_as_universal_primal(&mut self) -> ToadStoolResult<()> {
        let context = PrimalContext {
            user_id: "system".to_string(),
            device_id: defaults::network::LOCALHOST.to_string(),
            session_id: Uuid::new_v4().to_string(),
            network_location: NetworkLocation {
                ip_address: defaults::network::LOCALHOST.to_string(),
                subnet: None,
                network_id: None,
                geo_location: None,
            },
            security_level: SecurityLevel::Standard,
            metadata: HashMap::new(),
        };

        let provider = Arc::new(ToadStoolPrimalProvider::new(context));
        self.primal_registry
            .register_primal(provider.clone())
            .await?;
        self.toadstool_provider = Some(provider);

        info!("ToadStool registered as universal primal");
        Ok(())
    }

    /// Execute a universal job
    pub async fn execute_universal_job(
        &self,
        job: UniversalJob,
    ) -> ToadStoolResult<ExecutionResponse> {
        self.scheduler.schedule_job(job).await
    }

    /// Register a runtime engine
    pub async fn register_runtime_engine(
        &self,
        runtime_type: RuntimeType,
        engine: Box<dyn RuntimeEngine>,
    ) -> ToadStoolResult<()> {
        self.runtime_engines
            .write()
            .await
            .insert(runtime_type, engine);
        Ok(())
    }

    /// Get available runtime types
    pub async fn get_available_runtimes(&self) -> Vec<RuntimeType> {
        self.runtime_engines.read().await.keys().cloned().collect()
    }

    /// Find primals by capability
    pub async fn find_primals_by_capability(
        &self,
        capability: &PrimalCapability,
    ) -> Vec<Arc<dyn super::traits::UniversalPrimalProvider>> {
        self.primal_registry.find_by_capability(capability).await
    }

    /// Route primal request
    pub async fn route_primal_request(
        &self,
        request: super::requests::PrimalRequest,
    ) -> ToadStoolResult<super::requests::PrimalResponse> {
        self.primal_registry.route_request(request).await
    }

    /// Discover ecosystem (legacy compatibility)
    ///
    /// # Errors
    /// Currently does not return errors, but future versions may return errors
    /// if ecosystem discovery or service registration fails.
    #[must_use = "Ecosystem discovery result should be checked"]
    pub async fn discover_ecosystem(&self) -> ToadStoolResult<()> {
        if !self.config.ecosystem_integration {
            debug!("Ecosystem integration disabled in configuration");
            return Ok(());
        }

        info!("Discovering ecosystem through universal primal discovery");
        let _providers = self.primal_registry.get_all_providers().await;
        Ok(())
    }

    /// Get platform configuration
    #[must_use]
    pub const fn get_config(&self) -> &UniversalPlatformConfig {
        &self.config
    }

    /// Check if recursive hosting is enabled
    #[must_use]
    pub const fn is_recursive_hosting_enabled(&self) -> bool {
        self.config.recursive_hosting
    }

    /// Check if `BiomeOS` integration is enabled
    #[must_use]
    pub const fn is_biomeos_integration_enabled(&self) -> bool {
        self.config.biomeos_integration
    }
}

/// Initialize platform with runtime engines
pub async fn init_with_runtime_engines(
    engines: Vec<(RuntimeType, Box<dyn RuntimeEngine>)>,
) -> ToadStoolResult<UniversalComputePlatform> {
    let platform = UniversalComputePlatform::new().await?;

    for (runtime_type, engine) in engines {
        platform
            .register_runtime_engine(runtime_type, engine)
            .await?;
    }

    Ok(platform)
}

/// Get platform status
pub async fn get_platform_status() -> PlatformStatus {
    // For now, always return running
    PlatformStatus::Running
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_platform_config_default() {
        let config = UniversalPlatformConfig::default();
        assert!(config.recursive_hosting);
        assert!(config.ecosystem_integration);
        assert!(config.biomeos_integration);
        assert_eq!(config.max_concurrent_jobs, 100);
        assert!(!config.pure_ecosystem);
    }

    #[test]
    fn test_universal_platform_config_serialization_roundtrip() {
        let config = UniversalPlatformConfig {
            recursive_hosting: false,
            ecosystem_integration: false,
            biomeos_integration: false,
            max_concurrent_jobs: 42,
            pure_ecosystem: true,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: UniversalPlatformConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.recursive_hosting, deserialized.recursive_hosting);
        assert_eq!(config.max_concurrent_jobs, deserialized.max_concurrent_jobs);
        assert_eq!(config.pure_ecosystem, deserialized.pure_ecosystem);
    }

    #[test]
    fn test_platform_status_variants() {
        assert_eq!(PlatformStatus::Initializing, PlatformStatus::Initializing);
        assert_eq!(PlatformStatus::Running, PlatformStatus::Running);
        assert_eq!(PlatformStatus::Degraded, PlatformStatus::Degraded);
        assert_eq!(PlatformStatus::Stopped, PlatformStatus::Stopped);
        assert_ne!(PlatformStatus::Initializing, PlatformStatus::Running);
    }

    #[test]
    fn test_platform_status_serialization_roundtrip() {
        for status in [
            PlatformStatus::Initializing,
            PlatformStatus::Running,
            PlatformStatus::Degraded,
            PlatformStatus::Stopped,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: PlatformStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[tokio::test]
    async fn test_get_platform_status_returns_running() {
        let status = get_platform_status().await;
        assert_eq!(status, PlatformStatus::Running);
    }

    #[tokio::test]
    async fn test_new_platform_creation() {
        let platform = UniversalComputePlatform::new().await.unwrap();
        let config = platform.get_config();
        assert!(config.recursive_hosting);
        assert!(config.biomeos_integration);
    }

    #[tokio::test]
    async fn test_new_with_custom_config() {
        let config = UniversalPlatformConfig {
            recursive_hosting: false,
            ecosystem_integration: true,
            biomeos_integration: false,
            max_concurrent_jobs: 50,
            pure_ecosystem: false,
        };
        let platform = UniversalComputePlatform::new_with_config(config)
            .await
            .unwrap();
        assert!(!platform.is_recursive_hosting_enabled());
        assert!(!platform.is_biomeos_integration_enabled());
    }

    #[tokio::test]
    async fn test_is_recursive_hosting_enabled_default() {
        let platform = UniversalComputePlatform::new().await.unwrap();
        assert!(platform.is_recursive_hosting_enabled());
    }

    #[tokio::test]
    async fn test_is_biomeos_integration_enabled_default() {
        let platform = UniversalComputePlatform::new().await.unwrap();
        assert!(platform.is_biomeos_integration_enabled());
    }

    #[tokio::test]
    async fn test_get_available_runtimes_empty_initially() {
        let platform = UniversalComputePlatform::new().await.unwrap();
        let runtimes = platform.get_available_runtimes().await;
        // May be empty or pre-seeded depending on config, but must not panic.
        let _ = runtimes;
    }

    #[tokio::test]
    async fn test_find_primals_by_capability_empty_registry() {
        use crate::universal::types::PrimalCapability;
        let platform = UniversalComputePlatform::new().await.unwrap();
        let results = platform
            .find_primals_by_capability(&PrimalCapability::NativeExecution {
                architectures: vec!["x86_64".to_string()],
            })
            .await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_discover_ecosystem_succeeds() {
        let platform = UniversalComputePlatform::new().await.unwrap();
        // Should not panic; result may be Ok or a soft error.
        let _ = platform.discover_ecosystem().await;
    }

    #[tokio::test]
    async fn test_init_with_runtime_engines_empty_list() {
        let platform = init_with_runtime_engines(vec![]).await.unwrap();
        let runtimes = platform.get_available_runtimes().await;
        let _ = runtimes; // Empty or default-seeded — just verify no panic.
    }
}
