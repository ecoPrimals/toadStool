//! Production hardening — circuit breakers, memory pressure, resource leak detection.

mod circuit_breaker;
mod memory_pressure;
mod resource_leak;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
};
pub use memory_pressure::{
    DefaultMemoryPressureCallback, MemoryPressureCallback, MemoryPressureConfig,
    MemoryPressureHandler, MemoryPressureLevel,
};
pub use resource_leak::{ResourceAllocation, ResourceLeakDetector};

/// Production hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionHardeningConfig {
    pub enable_circuit_breakers: bool,
    pub enable_leak_detection: bool,
    pub enable_memory_pressure: bool,
    pub default_circuit_config: CircuitBreakerConfig,
    pub memory_pressure_config: MemoryPressureConfig,
    pub leak_detection_threshold: Duration,
    pub leak_cleanup_interval: Duration,
}

impl Default for ProductionHardeningConfig {
    fn default() -> Self {
        Self {
            enable_circuit_breakers: true,
            enable_leak_detection: true,
            enable_memory_pressure: true,
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
/// handler. Callers initialise once with [`initialize`], then use the delegation
/// methods to interact with each sub-system.
pub struct ProductionHardeningManager {
    config: ProductionHardeningConfig,
    circuit_breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
    leak_detector: Arc<ResourceLeakDetector>,
    memory_handler: Arc<MemoryPressureHandler>,
}

impl ProductionHardeningManager {
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
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            leak_detector,
            memory_handler,
        }
    }

    /// Start background tasks: resource-leak cleanup loop and memory-pressure
    /// monitoring. Idempotent — safe to call multiple times.
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.config.enable_leak_detection {
            self.leak_detector.start_cleanup_task().await;
        }
        if self.config.enable_memory_pressure {
            let callback = Box::new(DefaultMemoryPressureCallback);
            self.memory_handler.register_callback(callback).await;
        }
        Ok(())
    }

    // ── Circuit breaker API ────────────────────────────────────────────────────

    /// Retrieve an existing circuit breaker, or auto-create one using the
    /// default config. Callers always receive a ready-to-use breaker.
    pub async fn get_circuit_breaker(&self, service: &str) -> Arc<CircuitBreaker> {
        {
            let breakers = self.circuit_breakers.read().await;
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
            .await
            .insert(service.to_string(), Arc::clone(&breaker));
        breaker
    }

    /// Look up an existing circuit breaker without creating one.
    pub async fn find_circuit_breaker(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        self.circuit_breakers.read().await.get(service).cloned()
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
