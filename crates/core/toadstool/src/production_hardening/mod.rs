//! Production hardening - circuit breakers, memory pressure, leak detection

mod circuit_breaker;
mod memory_pressure;
mod resource_leak;

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
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
}

impl Default for ProductionHardeningConfig {
    fn default() -> Self {
        Self {
            enable_circuit_breakers: true,
            enable_leak_detection: true,
            enable_memory_pressure: true,
            default_circuit_config: CircuitBreakerConfig::default(),
            memory_pressure_config: MemoryPressureConfig::default(),
            leak_detection_threshold: Duration::from_secs(1800),
        }
    }
}

/// Production hardening manager
pub struct ProductionHardeningManager {
    config: ProductionHardeningConfig,
    circuit_breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
}

impl ProductionHardeningManager {
    #[must_use]
    pub fn new(config: ProductionHardeningConfig) -> Self {
        Self {
            config,
            circuit_breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn get_circuit_breaker(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        let breakers = self.circuit_breakers.read().await;
        breakers.get(service).cloned()
    }

    pub async fn get_or_create_circuit_breaker(&self, service: &str) -> Arc<CircuitBreaker> {
        if let Some(b) = self.get_circuit_breaker(service).await {
            return b;
        }
        let breaker = Arc::new(CircuitBreaker::new(
            service.to_string(),
            self.config.default_circuit_config.clone(),
        ));
        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(service.to_string(), Arc::clone(&breaker));
        breaker
    }
}
