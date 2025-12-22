//! Universal Compute Platform configuration and management

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool_config::defaults;

use crate::{execution::RuntimeEngine, execution::RuntimeType, ToadStoolResult};

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
    pub fn get_config(&self) -> &UniversalPlatformConfig {
        &self.config
    }

    /// Check if recursive hosting is enabled
    #[must_use]
    pub fn is_recursive_hosting_enabled(&self) -> bool {
        self.config.recursive_hosting
    }

    /// Check if `BiomeOS` integration is enabled
    #[must_use]
    pub fn is_biomeos_integration_enabled(&self) -> bool {
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
