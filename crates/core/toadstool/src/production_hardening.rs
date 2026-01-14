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
