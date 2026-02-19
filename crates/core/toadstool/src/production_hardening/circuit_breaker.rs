//! Circuit breaker pattern for fault tolerance.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::Instant;
use tracing::{error, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub rolling_window: Duration,
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
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    half_open_semaphore: Arc<Semaphore>,
    service_name: String,
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

impl CircuitBreaker {
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

    pub async fn execute<F, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        if self.is_circuit_open().await {
            return Err(CircuitBreakerError::CircuitOpen {
                service: self.service_name.clone(),
            });
        }

        let _permit = if self.is_half_open().await {
            Some(self.half_open_semaphore.acquire().await.map_err(|_| {
                CircuitBreakerError::HalfOpenLimitExceeded {
                    service: self.service_name.clone(),
                }
            })?)
        } else {
            None
        };

        match operation.await {
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

    async fn is_circuit_open(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitState::Open => {
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() > self.config.timeout {
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
            CircuitState::HalfOpen => self.half_open_semaphore.available_permits() == 0,
            CircuitState::Closed => false,
        }
    }

    async fn is_half_open(&self) -> bool {
        *self.state.read().await == CircuitState::HalfOpen
    }

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
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {
                warn!(
                    "Recorded success while circuit is open for service: {}",
                    self.service_name
                );
            }
        }
    }

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
                drop(state);
                self.transition_to_open().await;
            }
            CircuitState::Open => {
                self.update_failure_time().await;
            }
        }
    }

    async fn transition_to_closed(&self) {
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;
        info!("Circuit breaker closed for service: {}", self.service_name);
    }

    async fn transition_to_open(&self) {
        *self.state.write().await = CircuitState::Open;
        self.update_failure_time().await;
        error!("Circuit breaker opened for service: {}", self.service_name);
    }

    async fn transition_to_half_open(&self) {
        *self.state.write().await = CircuitState::HalfOpen;
        *self.success_count.write().await = 0;
        info!(
            "Circuit breaker half-open for service: {}",
            self.service_name
        );
    }

    async fn update_failure_time(&self) {
        *self.last_failure_time.write().await = Some(Instant::now());
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    pub async fn get_failure_count(&self) -> u32 {
        *self.failure_count.read().await
    }
}
