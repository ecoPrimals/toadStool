// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{BackoffStrategy, LoadBalancingStrategy};

/// Network load balancer for distributed execution
pub struct NetworkLoadBalancer {
    _strategies: Vec<LoadBalancingStrategy>,
    node_health: Arc<tokio::sync::RwLock<HashMap<String, NodeHealth>>>,
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
            node_health: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register or update a node's health metrics. Called by Songbird capability discovery.
    pub async fn register_node(&self, node_id: String, health: NodeHealth) {
        self.node_health.write().await.insert(node_id, health);
    }

    /// Deregister a node (e.g. after health probe failure).
    pub async fn deregister_node(&self, node_id: &str) {
        self.node_health.write().await.remove(node_id);
    }

    /// Select the least-loaded healthy node. Returns `None` if no remote nodes are registered.
    pub async fn select_node(&self) -> Option<String> {
        let health = self.node_health.read().await;
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
        self.node_health.read().await.clone()
    }
}

impl Default for NetworkLoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fault tolerance manager for network distribution
pub struct FaultToleranceManager {
    _circuit_breakers: Arc<tokio::sync::RwLock<HashMap<String, CircuitBreaker>>>,
    #[allow(dead_code, reason = "used in tests")]
    retries: Arc<RetryManager>,
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
            _circuit_breakers: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
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

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "test values are exact literals")]
mod tests {
    use super::*;

    #[test]
    fn test_load_balancer_creation() {
        let lb = NetworkLoadBalancer::new();
        assert!(lb.node_health.try_read().is_ok());
    }

    #[test]
    fn test_load_balancer_default() {
        let lb = NetworkLoadBalancer::default();
        assert!(lb.node_health.try_read().is_ok());
    }

    #[test]
    fn test_node_health_creation() {
        let health = NodeHealth {
            healthy: true,
            cpu_usage: 45.5,
            memory_usage: 60.2,
            response_time_ms: 150,
        };

        assert!(health.healthy);
        assert_eq!(health.cpu_usage, 45.5);
        assert_eq!(health.memory_usage, 60.2);
        assert_eq!(health.response_time_ms, 150);
    }

    #[test]
    fn test_node_health_clone() {
        let health = NodeHealth {
            healthy: true,
            cpu_usage: 30.0,
            memory_usage: 50.0,
            response_time_ms: 100,
        };

        let cloned = health.clone();
        assert_eq!(cloned.healthy, health.healthy);
        assert_eq!(cloned.cpu_usage, health.cpu_usage);
        assert_eq!(cloned.memory_usage, health.memory_usage);
    }

    #[test]
    fn test_node_health_debug() {
        let health = NodeHealth {
            healthy: false,
            cpu_usage: 90.0,
            memory_usage: 95.0,
            response_time_ms: 500,
        };

        let debug_str = format!("{:?}", health);
        assert!(debug_str.contains("NodeHealth"));
    }

    #[test]
    fn test_fault_tolerance_manager_creation() {
        let manager = FaultToleranceManager::new();
        assert_eq!(manager.retries.max_retries, 3);
    }

    #[test]
    fn test_fault_tolerance_manager_default() {
        let manager = FaultToleranceManager::default();
        assert_eq!(manager.retries.max_retries, 3);
    }

    #[test]
    fn test_circuit_breaker_states() {
        let closed = CircuitBreaker {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            last_failure_time: None,
        };

        let open = CircuitBreaker {
            state: CircuitBreakerState::Open,
            failure_count: 5,
            last_failure_time: Some(std::time::Instant::now()),
        };

        let half_open = CircuitBreaker {
            state: CircuitBreakerState::HalfOpen,
            failure_count: 2,
            last_failure_time: None,
        };

        assert_eq!(closed.failure_count, 0);
        assert_eq!(open.failure_count, 5);
        assert_eq!(half_open.failure_count, 2);
    }

    #[test]
    fn test_circuit_breaker_clone() {
        let breaker = CircuitBreaker {
            state: CircuitBreakerState::Closed,
            failure_count: 3,
            last_failure_time: None,
        };

        let cloned = breaker.clone();
        assert_eq!(cloned.failure_count, 3);
    }

    #[test]
    fn test_circuit_breaker_state_debug() {
        let state = CircuitBreakerState::Closed;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Closed"));
    }

    #[test]
    fn test_retry_manager() {
        let manager = RetryManager {
            max_retries: 5,
            backoff_strategy: BackoffStrategy::Exponential {
                base_ms: 500,
                max_ms: 10000,
            },
        };

        assert_eq!(manager.max_retries, 5);
    }

    #[test]
    fn test_backoff_strategy_exponential() {
        let strategy = BackoffStrategy::Exponential {
            base_ms: 1000,
            max_ms: 30000,
        };

        if let BackoffStrategy::Exponential { base_ms, max_ms } = strategy {
            assert_eq!(base_ms, 1000);
            assert_eq!(max_ms, 30000);
        } else {
            panic!("Expected Exponential backoff");
        }
    }

    #[test]
    fn test_node_health_healthy_state() {
        let healthy = NodeHealth {
            healthy: true,
            cpu_usage: 25.0,
            memory_usage: 40.0,
            response_time_ms: 50,
        };

        let unhealthy = NodeHealth {
            healthy: false,
            cpu_usage: 95.0,
            memory_usage: 98.0,
            response_time_ms: 1000,
        };

        assert!(healthy.healthy);
        assert!(!unhealthy.healthy);
        assert!(healthy.cpu_usage < unhealthy.cpu_usage);
        assert!(healthy.response_time_ms < unhealthy.response_time_ms);
    }

    #[test]
    fn test_circuit_breaker_with_failure() {
        let breaker = CircuitBreaker {
            state: CircuitBreakerState::Open,
            failure_count: 10,
            last_failure_time: Some(std::time::Instant::now()),
        };

        assert_eq!(breaker.failure_count, 10);
        assert!(breaker.last_failure_time.is_some());
    }

    #[test]
    fn test_circuit_breaker_debug() {
        let breaker = CircuitBreaker {
            state: CircuitBreakerState::HalfOpen,
            failure_count: 1,
            last_failure_time: None,
        };

        let debug_str = format!("{:?}", breaker);
        assert!(debug_str.contains("CircuitBreaker"));
    }

    #[tokio::test]
    async fn test_register_and_select_node() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "node-a".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                response_time_ms: 50,
            },
        )
        .await;

        let selected = lb.select_node().await;
        assert_eq!(selected, Some("node-a".to_string()));
    }

    #[tokio::test]
    async fn test_select_node_empty_returns_none() {
        let lb = NetworkLoadBalancer::new();
        assert!(lb.select_node().await.is_none());
    }

    #[tokio::test]
    async fn test_select_least_loaded_node() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "heavy".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 90.0,
                memory_usage: 85.0,
                response_time_ms: 300,
            },
        )
        .await;
        lb.register_node(
            "light".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 10.0,
                memory_usage: 20.0,
                response_time_ms: 50,
            },
        )
        .await;

        let selected = lb.select_node().await;
        assert_eq!(selected, Some("light".to_string()));
    }

    #[tokio::test]
    async fn test_select_skips_unhealthy_nodes() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "down".into(),
            NodeHealth {
                healthy: false,
                cpu_usage: 5.0,
                memory_usage: 5.0,
                response_time_ms: 10,
            },
        )
        .await;
        lb.register_node(
            "up".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 50.0,
                memory_usage: 50.0,
                response_time_ms: 100,
            },
        )
        .await;

        let selected = lb.select_node().await;
        assert_eq!(selected, Some("up".to_string()));
    }

    #[tokio::test]
    async fn test_all_unhealthy_returns_none() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "down1".into(),
            NodeHealth {
                healthy: false,
                cpu_usage: 5.0,
                memory_usage: 5.0,
                response_time_ms: 10,
            },
        )
        .await;
        lb.register_node(
            "down2".into(),
            NodeHealth {
                healthy: false,
                cpu_usage: 10.0,
                memory_usage: 10.0,
                response_time_ms: 20,
            },
        )
        .await;

        assert!(lb.select_node().await.is_none());
    }

    #[tokio::test]
    async fn test_deregister_node() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "node-a".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                response_time_ms: 50,
            },
        )
        .await;
        assert!(lb.select_node().await.is_some());

        lb.deregister_node("node-a").await;
        assert!(lb.select_node().await.is_none());
    }

    #[tokio::test]
    async fn test_node_health_snapshot() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "a".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 10.0,
                memory_usage: 20.0,
                response_time_ms: 30,
            },
        )
        .await;
        lb.register_node(
            "b".into(),
            NodeHealth {
                healthy: false,
                cpu_usage: 80.0,
                memory_usage: 90.0,
                response_time_ms: 500,
            },
        )
        .await;

        let snapshot = lb.node_health_snapshot().await;
        assert_eq!(snapshot.len(), 2);
        assert!(snapshot.get("a").is_some_and(|h| h.healthy));
        assert!(snapshot.get("b").is_some_and(|h| !h.healthy));
    }

    #[tokio::test]
    async fn test_register_updates_existing_node() {
        let lb = NetworkLoadBalancer::new();
        lb.register_node(
            "node-a".into(),
            NodeHealth {
                healthy: true,
                cpu_usage: 20.0,
                memory_usage: 30.0,
                response_time_ms: 50,
            },
        )
        .await;
        lb.register_node(
            "node-a".into(),
            NodeHealth {
                healthy: false,
                cpu_usage: 95.0,
                memory_usage: 99.0,
                response_time_ms: 1000,
            },
        )
        .await;

        let snapshot = lb.node_health_snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.get("node-a").is_some_and(|h| !h.healthy));
    }
}
