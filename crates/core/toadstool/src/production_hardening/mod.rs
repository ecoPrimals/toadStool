// SPDX-License-Identifier: AGPL-3.0-or-later
//! Production hardening — circuit breakers, memory pressure, resource leak detection.

#[cfg(feature = "hardening")]
mod circuit_breaker;
mod memory_pressure;
#[cfg(feature = "runtime")]
mod resource_leak;

#[cfg(feature = "runtime")]
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "runtime")]
use crate::ToadStoolResult;
use serde::{Deserialize, Serialize};
#[cfg(all(feature = "runtime", feature = "hardening"))]
use std::sync::RwLock;
#[cfg(feature = "runtime")]
use uuid::Uuid;

#[cfg(feature = "hardening")]
pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
};
pub use memory_pressure::{
    DefaultMemoryPressureCallback, MemoryPressureCallback, MemoryPressureConfig,
    MemoryPressureDispatch, MemoryPressureHandler, MemoryPressureLevel,
};
#[cfg(feature = "runtime")]
pub use resource_leak::{ResourceAllocation, ResourceLeakDetector};

/// Production hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionHardeningConfig {
    /// Enable circuit breakers for fault tolerance.
    pub enable_circuit_breakers: bool,
    /// Enable resource leak detection.
    pub enable_leak_detection: bool,
    /// Enable memory pressure monitoring.
    pub enable_memory_pressure: bool,
    /// Default circuit breaker settings.
    #[cfg(feature = "hardening")]
    pub default_circuit_config: CircuitBreakerConfig,
    /// Memory pressure thresholds and callbacks.
    pub memory_pressure_config: MemoryPressureConfig,
    /// Duration before idle allocation is considered a leak.
    pub leak_detection_threshold: Duration,
    /// Interval for leak cleanup sweep.
    pub leak_cleanup_interval: Duration,
}

impl Default for ProductionHardeningConfig {
    fn default() -> Self {
        Self {
            enable_circuit_breakers: true,
            enable_leak_detection: true,
            enable_memory_pressure: true,
            #[cfg(feature = "hardening")]
            default_circuit_config: CircuitBreakerConfig::default(),
            memory_pressure_config: MemoryPressureConfig::default(),
            leak_detection_threshold: Duration::from_secs(300),
            leak_cleanup_interval: Duration::from_secs(300),
        }
    }
}

/// Unified production hardening manager.
///
/// Owns a circuit-breaker registry, a resource-leak detector, and a memory-pressure
/// handler. Callers construct once with [`Self::new`], then use the delegation
/// methods to interact with each sub-system.
#[cfg(feature = "runtime")]
pub struct ProductionHardeningManager {
    config: ProductionHardeningConfig,
    #[cfg(feature = "hardening")]
    circuit_breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
    leak_detector: Arc<ResourceLeakDetector>,
    memory_handler: Arc<MemoryPressureHandler>,
}

#[cfg(feature = "runtime")]
impl ProductionHardeningManager {
    /// Creates a new production hardening manager with the given config.
    #[must_use]
    pub fn new(config: ProductionHardeningConfig) -> Self {
        let leak_detector = Arc::new(ResourceLeakDetector::new(
            config.leak_detection_threshold,
            config.leak_cleanup_interval,
        ));
        let memory_handler = Arc::new(MemoryPressureHandler::new(
            config.memory_pressure_config.clone(),
        ));
        Self {
            config,
            #[cfg(feature = "hardening")]
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            leak_detector,
            memory_handler,
        }
    }

    /// Start background tasks: resource-leak cleanup loop and memory-pressure
    /// monitoring. Idempotent — safe to call multiple times.
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        if self.config.enable_leak_detection {
            self.leak_detector.start_cleanup_task().await;
        }
        if self.config.enable_memory_pressure {
            self.memory_handler
                .register_callback(Arc::new(MemoryPressureDispatch::Default(
                    DefaultMemoryPressureCallback,
                )))
                .await;
        }
        Ok(())
    }

    // ── Circuit breaker API ────────────────────────────────────────────────────

    /// Retrieve an existing circuit breaker, or auto-create one using the
    /// default config. Callers always receive a ready-to-use breaker.
    #[cfg(feature = "hardening")]
    pub async fn get_circuit_breaker(&self, service: &str) -> Arc<CircuitBreaker> {
        {
            let breakers = self
                .circuit_breakers
                .read()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(b) = breakers.get(service) {
                return Arc::clone(b);
            }
        }
        let breaker = Arc::new(CircuitBreaker::new(
            service.to_string(),
            self.config.default_circuit_config.clone(),
        ));
        self.circuit_breakers
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(service.to_string(), Arc::clone(&breaker));
        breaker
    }

    /// Look up an existing circuit breaker without creating one.
    #[cfg(feature = "hardening")]
    pub async fn find_circuit_breaker(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        self.circuit_breakers
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(service)
            .cloned()
    }

    // ── Resource leak detection API ────────────────────────────────────────────

    /// Register a new resource allocation for leak tracking.
    pub async fn track_resource(&self, allocation: ResourceAllocation) {
        self.leak_detector.track_allocation(allocation).await;
    }

    /// Update the last-accessed timestamp for a tracked resource.
    pub async fn update_resource_access(&self, resource_id: Uuid) {
        self.leak_detector.update_access(resource_id).await;
    }

    /// Remove a resource from leak tracking (normal deallocation path).
    pub async fn remove_resource(&self, resource_id: Uuid) {
        self.leak_detector.remove_allocation(resource_id).await;
    }

    // ── Memory pressure API ────────────────────────────────────────────────────

    /// Report current memory usage. Triggers pressure callbacks when thresholds
    /// are exceeded.
    pub async fn update_memory_usage(&self, total_memory: u64, used_memory: u64) {
        self.memory_handler
            .update_memory_usage(total_memory, used_memory)
            .await;
    }

    /// Current memory pressure level.
    pub async fn get_memory_pressure_level(&self) -> MemoryPressureLevel {
        self.memory_handler.get_pressure_level().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_production_hardening_config_default() {
        let config = ProductionHardeningConfig::default();
        assert!(config.enable_circuit_breakers);
        assert!(config.enable_leak_detection);
        assert!(config.enable_memory_pressure);
        assert_eq!(config.leak_detection_threshold, Duration::from_secs(300));
        assert_eq!(config.leak_cleanup_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_production_hardening_manager_new() {
        let config = ProductionHardeningConfig::default();
        let _manager = ProductionHardeningManager::new(config);
    }

    #[tokio::test]
    #[cfg(feature = "hardening")]
    async fn test_production_hardening_manager_get_circuit_breaker() {
        let config = ProductionHardeningConfig::default();
        let manager = ProductionHardeningManager::new(config);
        let breaker = manager.get_circuit_breaker("test-service").await;
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    #[cfg(feature = "hardening")]
    async fn test_production_hardening_manager_find_circuit_breaker() {
        let config = ProductionHardeningConfig::default();
        let manager = ProductionHardeningManager::new(config);
        assert!(manager.find_circuit_breaker("nonexistent").await.is_none());

        manager.get_circuit_breaker("my-svc").await;
        assert!(manager.find_circuit_breaker("my-svc").await.is_some());
    }

    #[tokio::test]
    async fn test_production_hardening_manager_track_and_remove_resource() {
        let config = ProductionHardeningConfig::default();
        let manager = ProductionHardeningManager::new(config);
        let id = uuid::Uuid::new_v4();
        let allocation = super::resource_leak::ResourceAllocation {
            id,
            resource_type: "test".to_string(),
            allocated_at: std::time::Instant::now(),
            requirements: crate::resources::ResourceRequirements::default(),
            owner: "test".to_string(),
            last_accessed: std::time::Instant::now(),
        };
        manager.track_resource(allocation).await;
        manager.remove_resource(id).await;
    }

    #[tokio::test]
    async fn test_production_hardening_manager_memory_pressure() {
        let config = ProductionHardeningConfig::default();
        let manager = ProductionHardeningManager::new(config);
        manager.update_memory_usage(1000, 500).await;
        let level = manager.get_memory_pressure_level().await;
        assert_eq!(level, MemoryPressureLevel::Normal);
    }

    #[test]
    fn test_production_hardening_config_serde() {
        let config = ProductionHardeningConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: ProductionHardeningConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.enable_circuit_breakers,
            decoded.enable_circuit_breakers
        );
    }
}
