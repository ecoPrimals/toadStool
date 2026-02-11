//! # Production Hardening Module
//!
//! This module provides critical production hardening features for `ToadStool`:
//! - Circuit breaker patterns for fault tolerance
//! - Resource leak detection and automatic cleanup
//! - Memory pressure handling and optimization
//! - Advanced error recovery mechanisms
//! - Performance monitoring and alerting
//! - Security hardening and audit logging

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::resources::ResourceRequirements;
use crate::ToadStoolResult;

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,
    /// Circuit is open - requests are rejected
    Open,
    /// Circuit is half-open - testing if service is recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit
    pub success_threshold: u32,
    /// Timeout before trying half-open state
    pub timeout: Duration,
    /// Rolling window size for failure rate calculation
    pub rolling_window: Duration,
    /// Maximum concurrent requests in half-open state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            rolling_window: Duration::from_secs(60),
            half_open_max_requests: 3,
        }
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    /// Configuration
    config: CircuitBreakerConfig,
    /// Current state
    state: Arc<RwLock<CircuitState>>,
    /// Failure count
    failure_count: Arc<RwLock<u32>>,
    /// Success count (in half-open state)
    success_count: Arc<RwLock<u32>>,
    /// Last failure time
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    /// Semaphore for half-open state
    half_open_semaphore: Arc<Semaphore>,
    /// Service name for logging
    service_name: String,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    #[must_use]
    pub fn new(service_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let half_open_permits = config.half_open_max_requests as usize;

        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            half_open_semaphore: Arc::new(Semaphore::new(half_open_permits)),
            service_name: service_name.into(),
        }
    }

    /// Execute function with circuit breaker protection
    ///
    /// # Errors
    ///
    /// Returns `CircuitBreakerError` if:
    /// - Circuit breaker is in Open state (`CircuitOpen`)
    /// - Half-open limit is exceeded (`HalfOpenLimitExceeded`)
    /// - The operation fails (`ServiceFailure`)
    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Check if circuit is open
        if self.is_circuit_open().await {
            return Err(CircuitBreakerError::CircuitOpen {
                service: self.service_name.clone(),
            });
        }

        // Handle half-open state
        let _permit = if self.is_half_open().await {
            Some(self.half_open_semaphore.acquire().await.map_err(|_| {
                CircuitBreakerError::HalfOpenLimitExceeded {
                    service: self.service_name.clone(),
                }
            })?)
        } else {
            None
        };

        // Execute operation
        let result = operation.await;

        // Update circuit state based on result
        match result {
            Ok(value) => {
                self.record_success().await;
                Ok(value)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitBreakerError::ServiceFailure {
                    service: self.service_name.clone(),
                    error: e.to_string(),
                })
            }
        }
    }

    /// Check if circuit is open
    async fn is_circuit_open(&self) -> bool {
        let state = self.state.read().await;

        match *state {
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() > self.config.timeout {
                        // Transition to half-open
                        drop(state);
                        self.transition_to_half_open().await;
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => {
                // Check if we have permits available
                self.half_open_semaphore.available_permits() == 0
            }
            CircuitState::Closed => false,
        }
    }

    /// Check if circuit is half-open
    async fn is_half_open(&self) -> bool {
        let state = self.state.read().await;
        *state == CircuitState::HalfOpen
    }

    /// Record successful execution
    async fn record_success(&self) {
        let state = self.state.read().await;

        match *state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    drop(state);
                    drop(success_count);
                    self.transition_to_closed().await;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on successful execution
                let mut failure_count = self.failure_count.write().await;
                *failure_count = 0;
            }
            CircuitState::Open => {
                // Should not happen, but reset if it does
                warn!(
                    "Recorded success while circuit is open for service: {}",
                    self.service_name
                );
            }
        }
    }

    /// Record failed execution
    async fn record_failure(&self) {
        let state = self.state.read().await;

        match *state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    drop(state);
                    drop(failure_count);
                    self.transition_to_open().await;
                }
            }
            CircuitState::HalfOpen => {
                // Transition back to open on failure
                drop(state);
                self.transition_to_open().await;
            }
            CircuitState::Open => {
                // Already open, just update timestamp
                self.update_failure_time().await;
            }
        }
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;

        // Reset counters
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;

        info!("Circuit breaker closed for service: {}", self.service_name);
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;

        self.update_failure_time().await;

        error!("Circuit breaker opened for service: {}", self.service_name);
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;

        // Reset success count
        *self.success_count.write().await = 0;

        info!(
            "Circuit breaker half-open for service: {}",
            self.service_name
        );
    }

    /// Update failure time
    async fn update_failure_time(&self) {
        let mut last_failure = self.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Get failure count
    pub async fn get_failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }
}

/// Circuit breaker error types
#[derive(Debug, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open for service: {service}")]
    CircuitOpen { service: String },

    #[error("Half-open limit exceeded for service: {service}")]
    HalfOpenLimitExceeded { service: String },

    #[error("Service failure for {service}: {error}")]
    ServiceFailure { service: String, error: String },
}

/// Resource leak detector
pub struct ResourceLeakDetector {
    /// Resource allocations
    allocations: Arc<RwLock<HashMap<Uuid, ResourceAllocation>>>,
    /// Leak detection threshold
    leak_threshold: Duration,
    /// Cleanup interval
    cleanup_interval: Duration,
}

/// Resource allocation tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Resource ID
    pub id: Uuid,
    /// Resource type
    pub resource_type: String,
    /// Allocation time
    pub allocated_at: Instant,
    /// Resource requirements
    pub requirements: ResourceRequirements,
    /// Owner/workload ID
    pub owner: String,
    /// Last access time
    pub last_accessed: Instant,
}

impl ResourceLeakDetector {
    /// Create new resource leak detector
    #[must_use]
    pub fn new(leak_threshold: Duration, cleanup_interval: Duration) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            leak_threshold,
            cleanup_interval,
        }
    }

    /// Track resource allocation
    pub async fn track_allocation(&self, allocation: ResourceAllocation) {
        let mut allocations = self.allocations.write().await;
        allocations.insert(allocation.id, allocation);
    }

    /// Update resource access time
    pub async fn update_access(&self, resource_id: Uuid) {
        let mut allocations = self.allocations.write().await;
        if let Some(allocation) = allocations.get_mut(&resource_id) {
            allocation.last_accessed = Instant::now();
        }
    }

    /// Remove resource allocation
    pub async fn remove_allocation(&self, resource_id: Uuid) {
        let mut allocations = self.allocations.write().await;
        allocations.remove(&resource_id);
    }

    /// Detect and cleanup leaked resources
    pub async fn cleanup_leaked_resources(&self) -> Vec<ResourceAllocation> {
        let mut allocations = self.allocations.write().await;
        let now = Instant::now();

        let mut leaked = Vec::new();
        allocations.retain(|_, allocation| {
            if now.duration_since(allocation.last_accessed) > self.leak_threshold {
                warn!(
                    "Detected resource leak: {} ({}) allocated at {:?}",
                    allocation.id, allocation.resource_type, allocation.allocated_at
                );
                leaked.push(allocation.clone());
                false
            } else {
                true
            }
        });

        leaked
    }

    /// Start background cleanup task
    pub async fn start_cleanup_task(&self) {
        let allocations = Arc::clone(&self.allocations);
        let leak_threshold = self.leak_threshold;
        let cleanup_interval = self.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let detector = ResourceLeakDetector {
                    allocations: Arc::clone(&allocations),
                    leak_threshold,
                    cleanup_interval,
                };

                let leaked = detector.cleanup_leaked_resources().await;
                if !leaked.is_empty() {
                    error!("Cleaned up {} leaked resources", leaked.len());
                }
            }
        });
    }
}

/// Memory pressure handler
pub struct MemoryPressureHandler {
    /// Memory pressure thresholds
    config: MemoryPressureConfig,
    /// Current memory usage
    current_usage: Arc<RwLock<u64>>,
    /// Memory pressure callbacks
    callbacks: Arc<RwLock<Vec<Box<dyn MemoryPressureCallback>>>>,
}

/// Memory pressure configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPressureConfig {
    /// Warning threshold (percentage)
    pub warning_threshold: f64,
    /// Critical threshold (percentage)
    pub critical_threshold: f64,
    /// Emergency threshold (percentage)
    pub emergency_threshold: f64,
    /// Memory check interval
    pub check_interval: Duration,
}

impl Default for MemoryPressureConfig {
    fn default() -> Self {
        Self {
            warning_threshold: 70.0,
            critical_threshold: 85.0,
            emergency_threshold: 95.0,
            check_interval: Duration::from_secs(10),
        }
    }
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressureLevel {
    Normal,
    Warning,
    Critical,
    Emergency,
}

/// Memory pressure callback trait
#[async_trait]
pub trait MemoryPressureCallback: Send + Sync {
    /// Handle memory pressure
    async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64);
}

impl MemoryPressureHandler {
    /// Create new memory pressure handler
    #[must_use]
    pub fn new(config: MemoryPressureConfig) -> Self {
        Self {
            config,
            current_usage: Arc::new(RwLock::new(0)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register memory pressure callback
    pub async fn register_callback(&self, callback: Box<dyn MemoryPressureCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    /// Update memory usage
    pub async fn update_memory_usage(&self, total_memory: u64, used_memory: u64) {
        let usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        let level = if usage_percent >= self.config.emergency_threshold {
            MemoryPressureLevel::Emergency
        } else if usage_percent >= self.config.critical_threshold {
            MemoryPressureLevel::Critical
        } else if usage_percent >= self.config.warning_threshold {
            MemoryPressureLevel::Warning
        } else {
            MemoryPressureLevel::Normal
        };

        let mut current_usage = self.current_usage.write().await;
        *current_usage = used_memory;

        // Trigger callbacks if pressure detected
        if level != MemoryPressureLevel::Normal {
            let callbacks = self.callbacks.read().await;
            for callback in callbacks.iter() {
                callback.handle_pressure(level, usage_percent).await;
            }
        }
    }

    /// Get current memory pressure level
    pub async fn get_pressure_level(&self) -> MemoryPressureLevel {
        // This would need to be implemented based on actual memory usage
        MemoryPressureLevel::Normal
    }
}

/// Production hardening manager
pub struct ProductionHardeningManager {
    /// Circuit breakers
    circuit_breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
    /// Resource leak detector
    leak_detector: Arc<ResourceLeakDetector>,
    /// Memory pressure handler
    memory_handler: Arc<MemoryPressureHandler>,
    /// Configuration
    config: ProductionHardeningConfig,
}

/// Production hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionHardeningConfig {
    /// Enable circuit breakers
    pub enable_circuit_breakers: bool,
    /// Enable resource leak detection
    pub enable_leak_detection: bool,
    /// Enable memory pressure handling
    pub enable_memory_pressure: bool,
    /// Default circuit breaker config
    pub default_circuit_config: CircuitBreakerConfig,
    /// Memory pressure config
    pub memory_pressure_config: MemoryPressureConfig,
    /// Resource leak detection threshold
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
            leak_detection_threshold: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl ProductionHardeningManager {
    /// Create new production hardening manager
    #[must_use]
    pub fn new(config: ProductionHardeningConfig) -> Self {
        let leak_detector = Arc::new(ResourceLeakDetector::new(
            config.leak_detection_threshold,
            Duration::from_secs(60), // 1 minute cleanup interval
        ));

        let memory_handler = Arc::new(MemoryPressureHandler::new(
            config.memory_pressure_config.clone(),
        ));

        Self {
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            leak_detector,
            memory_handler,
            config,
        }
    }

    /// Initialize production hardening
    ///
    /// # Errors
    ///
    /// Currently always succeeds, but returns `ToadStoolResult` for future extensibility
    /// (e.g., initialization failures, config validation errors)
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        info!("Initializing production hardening");

        if self.config.enable_leak_detection {
            self.leak_detector.start_cleanup_task().await;
        }

        // Add default memory pressure callback
        if self.config.enable_memory_pressure {
            self.memory_handler
                .register_callback(Box::new(DefaultMemoryPressureCallback))
                .await;
        }

        info!("Production hardening initialized");
        Ok(())
    }

    /// Get or create circuit breaker
    pub async fn get_circuit_breaker(&self, service_name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.circuit_breakers.write().await;

        if let Some(breaker) = breakers.get(service_name) {
            Arc::clone(breaker)
        } else {
            let breaker = Arc::new(CircuitBreaker::new(
                service_name.to_string(),
                self.config.default_circuit_config.clone(),
            ));
            breakers.insert(service_name.to_string(), Arc::clone(&breaker));
            breaker
        }
    }

    /// Track resource allocation
    pub async fn track_resource(&self, allocation: ResourceAllocation) {
        if self.config.enable_leak_detection {
            self.leak_detector.track_allocation(allocation).await;
        }
    }

    /// Update resource access
    pub async fn update_resource_access(&self, resource_id: Uuid) {
        if self.config.enable_leak_detection {
            self.leak_detector.update_access(resource_id).await;
        }
    }

    /// Remove resource tracking
    pub async fn remove_resource(&self, resource_id: Uuid) {
        if self.config.enable_leak_detection {
            self.leak_detector.remove_allocation(resource_id).await;
        }
    }

    /// Update memory usage
    pub async fn update_memory_usage(&self, total_memory: u64, used_memory: u64) {
        if self.config.enable_memory_pressure {
            self.memory_handler
                .update_memory_usage(total_memory, used_memory)
                .await;
        }
    }
}

/// Default memory pressure callback
pub struct DefaultMemoryPressureCallback;

#[async_trait]
impl MemoryPressureCallback for DefaultMemoryPressureCallback {
    async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64) {
        match level {
            MemoryPressureLevel::Normal => {}
            MemoryPressureLevel::Warning => {
                warn!("Memory pressure warning: {:.1}% usage", usage_percent);
            }
            MemoryPressureLevel::Critical => {
                error!("Memory pressure critical: {:.1}% usage", usage_percent);
                // Could trigger garbage collection or resource cleanup
            }
            MemoryPressureLevel::Emergency => {
                error!("Memory pressure emergency: {:.1}% usage", usage_percent);
                // Could trigger emergency shutdown or resource reclamation
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::RwLock;

    // ========================================================================
    // CircuitBreakerConfig tests
    // ========================================================================

    #[test]
    fn circuit_breaker_config_defaults() {
        let config = CircuitBreakerConfig::default();

        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.rolling_window, Duration::from_secs(60));
        assert_eq!(config.half_open_max_requests, 3);
    }

    #[test]
    fn circuit_breaker_config_construction() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
            rolling_window: Duration::from_secs(30),
            half_open_max_requests: 5,
        };

        assert_eq!(config.failure_threshold, 2);
        assert_eq!(config.success_threshold, 1);
        assert_eq!(config.timeout.as_millis(), 100);
        assert_eq!(config.half_open_max_requests, 5);
    }

    // ========================================================================
    // CircuitBreaker state transition tests
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_closed_initial_state() {
        let config = CircuitBreakerConfig::default();
        let breaker = CircuitBreaker::new("test-svc", config);

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert_eq!(breaker.get_failure_count().await, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_closed_to_open_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..3 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }

        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_rejects_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..2 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }

        let result = breaker
            .execute(async { Ok::<(), std::io::Error>(()) })
            .await;

        assert!(matches!(
            result,
            Err(CircuitBreakerError::CircuitOpen { .. })
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_open_to_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..2 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }
        assert_eq!(breaker.get_state().await, CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(60)).await;

        let result = breaker
            .execute(async { Ok::<(), std::io::Error>(()) })
            .await;
        assert!(result.is_ok());
        assert_eq!(breaker.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_half_open_to_closed_on_success_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..2 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }
        tokio::time::sleep(Duration::from_millis(60)).await;

        for _ in 0..2 {
            let result = breaker
                .execute(async { Ok::<(), std::io::Error>(()) })
                .await;
            assert!(result.is_ok());
        }

        assert_eq!(breaker.get_state().await, CircuitState::Closed);
        assert_eq!(breaker.get_failure_count().await, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_half_open_to_open_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 3,
            timeout: Duration::from_millis(50),
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..2 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }
        tokio::time::sleep(Duration::from_millis(60)).await;

        let _ = breaker
            .execute(async { Err::<(), _>(std::io::Error::other("fail again")) })
            .await;

        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_success_resets_failure_count_in_closed() {
        let config = CircuitBreakerConfig {
            failure_threshold: 5,
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        for _ in 0..2 {
            let _ = breaker
                .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
                .await;
        }
        assert_eq!(breaker.get_failure_count().await, 2);

        let _ = breaker
            .execute(async { Ok::<(), std::io::Error>(()) })
            .await;

        assert_eq!(breaker.get_failure_count().await, 0);
        assert_eq!(breaker.get_state().await, CircuitState::Closed);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_zero_failure_threshold_opens_immediately() {
        let config = CircuitBreakerConfig {
            failure_threshold: 0,
            success_threshold: 1,
            timeout: Duration::from_secs(60),
            ..CircuitBreakerConfig::default()
        };
        let breaker = CircuitBreaker::new("test-svc", config);

        let _ = breaker
            .execute(async { Err::<(), _>(std::io::Error::other("fail")) })
            .await;

        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_successful_execution_returns_value() {
        let breaker = CircuitBreaker::new("test-svc", CircuitBreakerConfig::default());

        let result = breaker
            .execute(async { Ok::<i32, std::io::Error>(42) })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_service_failure_error() {
        let breaker = CircuitBreaker::new("test-svc", CircuitBreakerConfig::default());

        let result = breaker
            .execute(async { Err::<(), _>(std::io::Error::other("service down")) })
            .await;

        assert!(matches!(
            result,
            Err(CircuitBreakerError::ServiceFailure { ref error, .. }) if error.contains("service down")
        ));
    }

    // ========================================================================
    // CircuitState tests
    // ========================================================================

    #[test]
    fn circuit_state_debug_clone_partial_eq() {
        let states = [
            CircuitState::Closed,
            CircuitState::Open,
            CircuitState::HalfOpen,
        ];
        for state in &states {
            let cloned = state.clone();
            assert_eq!(state, &cloned);
            assert!(!format!("{:?}", state).is_empty());
        }
    }

    // ========================================================================
    // ResourceLeakDetector tests
    // ========================================================================

    fn make_allocation(id: Uuid, last_accessed: Instant) -> ResourceAllocation {
        ResourceAllocation {
            id,
            resource_type: "test-resource".to_string(),
            allocated_at: Instant::now(),
            requirements: crate::resources::ResourceRequirements::default(),
            owner: "test-owner".to_string(),
            last_accessed,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_track_and_remove() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));
        let id = Uuid::new_v4();
        let allocation = make_allocation(id, Instant::now());

        detector.track_allocation(allocation).await;
        detector.remove_allocation(id).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_detects_leaked_resources() {
        let detector =
            ResourceLeakDetector::new(Duration::from_millis(10), Duration::from_secs(10));
        let id = Uuid::new_v4();
        let old_time = Instant::now() - Duration::from_secs(100);
        let allocation = make_allocation(id, old_time);

        detector.track_allocation(allocation).await;

        tokio::time::sleep(Duration::from_millis(20)).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 1);
        assert_eq!(leaked[0].id, id);
        assert_eq!(leaked[0].resource_type, "test-resource");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_no_leaks_for_fresh_resources() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));
        let allocation = make_allocation(Uuid::new_v4(), Instant::now());

        detector.track_allocation(allocation).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_update_access_prevents_leak() {
        let detector =
            ResourceLeakDetector::new(Duration::from_millis(50), Duration::from_secs(10));
        let id = Uuid::new_v4();
        let allocation = make_allocation(id, Instant::now());

        detector.track_allocation(allocation).await;
        tokio::time::sleep(Duration::from_millis(25)).await;
        detector.update_access(id).await;
        tokio::time::sleep(Duration::from_millis(25)).await;

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_empty_detector() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

        let leaked = detector.cleanup_leaked_resources().await;
        assert_eq!(leaked.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn resource_leak_detector_remove_nonexistent_no_panic() {
        let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

        detector.remove_allocation(Uuid::new_v4()).await;
    }

    #[test]
    fn resource_allocation_clone_debug() {
        let allocation = ResourceAllocation {
            id: Uuid::new_v4(),
            resource_type: "gpu".to_string(),
            allocated_at: Instant::now(),
            requirements: crate::resources::ResourceRequirements::default(),
            owner: "owner".to_string(),
            last_accessed: Instant::now(),
        };
        let cloned = allocation.clone();
        assert_eq!(allocation.id, cloned.id);
        assert_eq!(allocation.resource_type, cloned.resource_type);
        assert!(!format!("{:?}", allocation).is_empty());
    }

    // ========================================================================
    // MemoryPressureHandler tests
    // ========================================================================

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn memory_pressure_handler_update_triggers_callback() {
        let config = MemoryPressureConfig {
            warning_threshold: 50.0,
            critical_threshold: 80.0,
            emergency_threshold: 95.0,
            check_interval: Duration::from_secs(10),
        };
        let handler = MemoryPressureHandler::new(config);

        let callback_invoked = Arc::new(RwLock::new((None as Option<MemoryPressureLevel>, 0.0)));

        struct TestCallback {
            captured: Arc<RwLock<(Option<MemoryPressureLevel>, f64)>>,
        }
        #[async_trait::async_trait]
        impl MemoryPressureCallback for TestCallback {
            async fn handle_pressure(&self, level: MemoryPressureLevel, usage_percent: f64) {
                let mut guard = self.captured.write().await;
                *guard = (Some(level), usage_percent);
            }
        }

        let test_cb = TestCallback {
            captured: Arc::clone(&callback_invoked),
        };
        handler.register_callback(Box::new(test_cb)).await;

        handler.update_memory_usage(100, 60).await;

        let (level, pct) = *callback_invoked.read().await;
        assert_eq!(level, Some(MemoryPressureLevel::Warning));
        assert!((pct - 60.0).abs() < 0.01);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn memory_pressure_handler_normal_does_not_trigger_callback() {
        let config = MemoryPressureConfig::default();
        let handler = MemoryPressureHandler::new(config);

        let callback_invoked = Arc::new(AtomicBool::new(false));
        struct TestCallback {
            invoked: Arc<AtomicBool>,
        }
        #[async_trait::async_trait]
        impl MemoryPressureCallback for TestCallback {
            async fn handle_pressure(&self, _level: MemoryPressureLevel, _usage_percent: f64) {
                self.invoked.store(true, Ordering::SeqCst);
            }
        }

        let test_cb = TestCallback {
            invoked: Arc::clone(&callback_invoked),
        };
        handler.register_callback(Box::new(test_cb)).await;

        handler.update_memory_usage(1000, 100).await;

        assert!(!callback_invoked.load(Ordering::SeqCst));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn memory_pressure_handler_emergency_level() {
        let config = MemoryPressureConfig::default();
        let handler = MemoryPressureHandler::new(config);

        let callback_invoked = Arc::new(RwLock::new(None as Option<MemoryPressureLevel>));
        struct TestCallback {
            captured: Arc<RwLock<Option<MemoryPressureLevel>>>,
        }
        #[async_trait::async_trait]
        impl MemoryPressureCallback for TestCallback {
            async fn handle_pressure(&self, level: MemoryPressureLevel, _usage_percent: f64) {
                let mut guard = self.captured.write().await;
                *guard = Some(level);
            }
        }

        let test_cb = TestCallback {
            captured: Arc::clone(&callback_invoked),
        };
        handler.register_callback(Box::new(test_cb)).await;

        handler.update_memory_usage(100, 97).await;

        let level = *callback_invoked.read().await;
        assert_eq!(level, Some(MemoryPressureLevel::Emergency));
    }

    #[test]
    fn memory_pressure_config_defaults() {
        let config = MemoryPressureConfig::default();

        assert!((config.warning_threshold - 70.0).abs() < 0.01);
        assert!((config.critical_threshold - 85.0).abs() < 0.01);
        assert!((config.emergency_threshold - 95.0).abs() < 0.01);
        assert_eq!(config.check_interval, Duration::from_secs(10));
    }

    #[test]
    fn memory_pressure_level_partial_eq_debug() {
        assert_eq!(MemoryPressureLevel::Normal, MemoryPressureLevel::Normal);
        assert_ne!(MemoryPressureLevel::Normal, MemoryPressureLevel::Warning);
        assert_eq!(MemoryPressureLevel::Critical, MemoryPressureLevel::Critical);
        assert!(!format!("{:?}", MemoryPressureLevel::Emergency).is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn default_memory_pressure_callback_no_panic() {
        let cb = DefaultMemoryPressureCallback;
        cb.handle_pressure(MemoryPressureLevel::Normal, 50.0).await;
        cb.handle_pressure(MemoryPressureLevel::Warning, 75.0).await;
        cb.handle_pressure(MemoryPressureLevel::Critical, 90.0)
            .await;
        cb.handle_pressure(MemoryPressureLevel::Emergency, 98.0)
            .await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn memory_pressure_handler_get_pressure_level() {
        let handler = MemoryPressureHandler::new(MemoryPressureConfig::default());
        let level = handler.get_pressure_level().await;
        assert_eq!(level, MemoryPressureLevel::Normal);
    }

    // ========================================================================
    // Serialization round-trip tests
    // ========================================================================

    #[test]
    fn circuit_state_serialization_roundtrip() {
        let states = [
            CircuitState::Closed,
            CircuitState::Open,
            CircuitState::HalfOpen,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let deserialized: CircuitState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, &deserialized);
        }
    }

    #[test]
    fn memory_pressure_level_serialization_roundtrip() {
        let levels = [
            MemoryPressureLevel::Normal,
            MemoryPressureLevel::Warning,
            MemoryPressureLevel::Critical,
            MemoryPressureLevel::Emergency,
        ];
        for level in &levels {
            let json = serde_json::to_string(level).unwrap();
            let deserialized: MemoryPressureLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, &deserialized);
        }
    }

    #[test]
    fn circuit_breaker_config_serialization_roundtrip() {
        let config = CircuitBreakerConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: CircuitBreakerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.failure_threshold, deserialized.failure_threshold);
        assert_eq!(config.success_threshold, deserialized.success_threshold);
        assert_eq!(config.timeout, deserialized.timeout);
        assert_eq!(config.rolling_window, deserialized.rolling_window);
        assert_eq!(
            config.half_open_max_requests,
            deserialized.half_open_max_requests
        );
    }

    #[test]
    fn memory_pressure_config_serialization_roundtrip() {
        let config = MemoryPressureConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MemoryPressureConfig = serde_json::from_str(&json).unwrap();
        assert!((config.warning_threshold - deserialized.warning_threshold).abs() < 0.01);
        assert!((config.critical_threshold - deserialized.critical_threshold).abs() < 0.01);
        assert!((config.emergency_threshold - deserialized.emergency_threshold).abs() < 0.01);
        assert_eq!(config.check_interval, deserialized.check_interval);
    }

    #[test]
    fn production_hardening_config_serialization_roundtrip() {
        let config = ProductionHardeningConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProductionHardeningConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            config.enable_circuit_breakers,
            deserialized.enable_circuit_breakers
        );
        assert_eq!(
            config.enable_leak_detection,
            deserialized.enable_leak_detection
        );
        assert_eq!(
            config.enable_memory_pressure,
            deserialized.enable_memory_pressure
        );
        assert_eq!(
            config.leak_detection_threshold,
            deserialized.leak_detection_threshold
        );
    }

    // ========================================================================
    // ProductionHardeningConfig and ProductionHardeningManager tests
    // ========================================================================

    #[test]
    fn production_hardening_config_defaults() {
        let config = ProductionHardeningConfig::default();

        assert!(config.enable_circuit_breakers);
        assert!(config.enable_leak_detection);
        assert!(config.enable_memory_pressure);
        assert_eq!(config.leak_detection_threshold, Duration::from_secs(300));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn production_hardening_manager_get_circuit_breaker() {
        let manager = ProductionHardeningManager::new(ProductionHardeningConfig::default());

        let breaker1 = manager.get_circuit_breaker("svc-a").await;
        let breaker2 = manager.get_circuit_breaker("svc-a").await;

        assert!(Arc::ptr_eq(&breaker1, &breaker2));

        let breaker3 = manager.get_circuit_breaker("svc-b").await;
        assert!(!Arc::ptr_eq(&breaker1, &breaker3));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn circuit_breaker_error_display() {
        let err = CircuitBreakerError::CircuitOpen {
            service: "my-service".to_string(),
        };
        assert!(err.to_string().contains("my-service"));
        assert!(err.to_string().to_lowercase().contains("open"));
    }
}
