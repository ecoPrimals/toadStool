// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{BackoffStrategy, LoadBalancingStrategy};

/// Network load balancer for distributed execution
pub struct NetworkLoadBalancer {
    _strategies: Vec<LoadBalancingStrategy>,
    pub(crate) node_health: Arc<std::sync::RwLock<HashMap<String, NodeHealth>>>,
}

/// Health snapshot for a load-balanced node.
#[derive(Debug, Clone)]
pub struct NodeHealth {
    /// Whether the node is accepting traffic.
    pub healthy: bool,
    /// CPU utilization fraction (0.0–1.0).
    pub cpu_usage: f64,
    /// Memory utilization fraction (0.0–1.0).
    pub memory_usage: f64,
    /// Last observed response time in milliseconds.
    pub response_time_ms: u64,
}

impl NetworkLoadBalancer {
    /// Creates a load balancer with round-robin strategy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _strategies: vec![LoadBalancingStrategy::RoundRobin],
            node_health: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register or update a node's health metrics. Called by Coordination capability discovery.
    pub async fn register_node(&self, node_id: String, health: NodeHealth) {
        self.node_health
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(node_id, health);
    }

    /// Deregister a node (e.g. after health probe failure).
    pub async fn deregister_node(&self, node_id: &str) {
        self.node_health
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(node_id);
    }

    /// Select the least-loaded healthy node. Returns `None` if no remote nodes are registered.
    pub async fn select_node(&self) -> Option<String> {
        let health = self.node_health.read().unwrap_or_else(|e| e.into_inner());
        health
            .iter()
            .filter(|(_, h)| h.healthy)
            .min_by(|(_, a), (_, b)| {
                // Combined load score: 60 % CPU + 40 % memory pressure
                let score_a = 0.6f64.mul_add(a.cpu_usage, 0.4 * a.memory_usage);
                let score_b = 0.6f64.mul_add(b.cpu_usage, 0.4 * b.memory_usage);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| id.clone())
    }

    /// Snapshot of current node health for diagnostics.
    pub async fn node_health_snapshot(&self) -> HashMap<String, NodeHealth> {
        self.node_health
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl Default for NetworkLoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fault tolerance manager for network distribution
pub struct FaultToleranceManager {
    _circuit_breakers: Arc<std::sync::RwLock<HashMap<String, CircuitBreaker>>>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Reserved for fault-tolerance retry policy wiring; read in unit tests until production call sites land"
        )
    )]
    pub(crate) retries: Arc<RetryManager>,
}

/// Circuit breaker for a single target (tracks failures, opens on threshold).
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Current state (closed/open/half-open).
    pub state: CircuitBreakerState,
    /// Consecutive failure count.
    pub failure_count: u32,
    /// Timestamp of last failure for backoff.
    pub last_failure_time: Option<std::time::Instant>,
}

/// Circuit breaker state for fault isolation.
#[derive(Debug, Clone)]
pub enum CircuitBreakerState {
    /// Normal operation; requests flow through.
    Closed,
    /// Failures exceeded threshold; requests fail fast.
    Open,
    /// Probing to see if target recovered.
    HalfOpen,
}

/// Retry policy for transient failures.
pub struct RetryManager {
    /// Maximum retry attempts before giving up.
    pub max_retries: u32,
    /// Backoff strategy (exponential, linear, etc.).
    pub backoff_strategy: BackoffStrategy,
}

impl FaultToleranceManager {
    /// Creates a fault tolerance manager with default circuit breakers and retries.
    #[must_use]
    pub fn new() -> Self {
        const DEFAULT_BACKOFF_BASE_MS: u64 = 1_000;
        const DEFAULT_BACKOFF_MAX_MS: u64 = 30_000;
        Self {
            _circuit_breakers: Arc::new(std::sync::RwLock::new(HashMap::new())),
            retries: Arc::new(RetryManager {
                max_retries: 3,
                backoff_strategy: BackoffStrategy::Exponential {
                    base_ms: DEFAULT_BACKOFF_BASE_MS,
                    max_ms: DEFAULT_BACKOFF_MAX_MS,
                },
            }),
        }
    }
}

impl Default for FaultToleranceManager {
    fn default() -> Self {
        Self::new()
    }
}
