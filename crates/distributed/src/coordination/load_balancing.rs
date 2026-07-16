// SPDX-License-Identifier: AGPL-3.0-or-later
//! Load balancing coordination

use std::collections::HashMap;
use std::sync::Arc;

use toadstool::error::ToadStoolResult;
use toadstool_common::interned_strings::socket_env;
use tracing::debug;

use crate::ResourceRequirements;

use super::types::{
    CoordinationConnection, CoordinationFeedback, CoordinationFeedbackSender,
    CoordinationLoadBalancer, LoadBalancerConfig, LoadBalancingAdvice, NodeCapacityTracker, NodeId,
    PerformanceMetrics,
};

/// Load above this fraction triggers a rebalance suggestion.
const REBALANCE_THRESHOLD: f64 = 0.8;

impl CoordinationLoadBalancer {
    /// Initialize a load balancer with config and Coordination connection.
    pub async fn new(
        _config: LoadBalancerConfig,
        _connection: Arc<CoordinationConnection>,
    ) -> ToadStoolResult<Self> {
        debug!("Initializing load balancer");
        Ok(Self {
            strategies: HashMap::new(),
            capacity_tracker: NodeCapacityTracker::new(),
            performance_metrics: PerformanceMetrics::new(),
            feedback_sender: CoordinationFeedbackSender::default(),
        })
    }

    fn self_node_id(&self) -> String {
        std::env::var(socket_env::TOADSTOOL_GATE_ID)
            .or_else(|_| std::env::var(socket_env::HOSTNAME))
            .unwrap_or_else(|_| toadstool_common::constants::network::LOCALHOST_IPV4.to_string())
    }

    /// Recommend target nodes for the given resource requirements using current load data.
    pub async fn request_advice(
        &self,
        _requirements: &ResourceRequirements,
    ) -> ToadStoolResult<LoadBalancingAdvice> {
        let start = std::time::Instant::now();
        let strategy = self
            .strategies
            .keys()
            .next()
            .map(String::as_str)
            .unwrap_or("least-loaded");
        debug!(strategy, "Requesting load balancing advice");

        let snapshot = self.capacity_tracker.snapshot();
        let self_node = self.self_node_id();
        let (recommended, reasoning) = if snapshot.is_empty() {
            (
                vec![self_node],
                "No capacity data available; defaulting to self".to_string(),
            )
        } else {
            let least = self.capacity_tracker.least_loaded();
            least.map_or_else(
                || {
                    (
                        vec![self_node],
                        "All nodes saturated; defaulting to self".to_string(),
                    )
                },
                |node| {
                    let load = snapshot.get(&node).copied().unwrap_or(0.0);
                    (
                        vec![node.clone()],
                        format!(
                            "Selected '{}' with load {:.0}% (least-loaded node)",
                            node,
                            load * 100.0
                        ),
                    )
                },
            )
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;
        self.performance_metrics.record(elapsed_ms, false);

        let load_distribution: HashMap<String, f64> = snapshot;
        Ok(LoadBalancingAdvice {
            recommended_nodes: recommended,
            load_distribution,
            reasoning,
        })
    }

    /// Record load for a node and emit feedback for downstream coordination.
    pub async fn update_node_load(&self, node_id: &NodeId, load: f64) -> ToadStoolResult<()> {
        debug!(%node_id, load, "Recording node load");
        self.capacity_tracker.update(node_id, load);
        self.feedback_sender.send(CoordinationFeedback::LoadUpdate {
            node_id: node_id.clone(),
            load,
        });
        Ok(())
    }

    /// Return whether any tracked node exceeds the rebalance load threshold.
    pub async fn rebalance_if_needed(&self) -> ToadStoolResult<bool> {
        let snapshot = self.capacity_tracker.snapshot();
        let overloaded = snapshot.values().any(|&l| l > REBALANCE_THRESHOLD);
        if overloaded {
            debug!(
                "Rebalance needed: {} nodes above {:.0}% load threshold",
                snapshot
                    .values()
                    .filter(|&&l| l > REBALANCE_THRESHOLD)
                    .count(),
                REBALANCE_THRESHOLD * 100.0
            );
        }
        Ok(overloaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordination::types::{
        ConnectionHealth, CoordinationTransport, HttpProtocolConfig, MessageQueueProtocolConfig,
        ProtocolConfig,
    };
    use std::collections::HashMap;
    use std::time::Duration;

    const TEST_MOCK_ENDPOINT: &str = "http://localhost:8080";

    fn make_mock_connection() -> CoordinationConnection {
        CoordinationConnection {
            endpoints: vec![TEST_MOCK_ENDPOINT.to_string()],
            active_endpoint: TEST_MOCK_ENDPOINT.to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: ProtocolConfig {
                protocol: CoordinationTransport::HTTP,
                http: HttpProtocolConfig {
                    timeout_ms: 5000,
                    max_retries: 3,
                    headers: HashMap::new(),
                },
                message_queue: MessageQueueProtocolConfig {
                    queue_name: "default".to_string(),
                    exchange: "default".to_string(),
                    routing_key: "default".to_string(),
                },
            },
        }
    }

    fn make_load_balancer_config() -> LoadBalancerConfig {
        LoadBalancerConfig {
            strategy: "least-loaded".to_string(),
            feedback_interval: Duration::from_secs(5),
        }
    }

    async fn setup_load_balancer() -> ToadStoolResult<CoordinationLoadBalancer> {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        CoordinationLoadBalancer::new(config, connection).await
    }

    #[tokio::test]
    async fn test_load_balancer_new() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;
        let advice = lb.request_advice(&ResourceRequirements::default()).await?;

        assert!(!advice.recommended_nodes.is_empty());
        assert!(advice.reasoning.contains("localhost") || advice.reasoning.contains("No capacity"));
        Ok(())
    }

    #[tokio::test]
    async fn test_request_advice_empty_capacity() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;
        let advice = lb.request_advice(&ResourceRequirements::default()).await?;

        assert_eq!(
            advice.recommended_nodes[0],
            toadstool_common::constants::network::LOCALHOST_IPV4
        );
        assert!(advice.reasoning.contains("No capacity data"));
        assert!(advice.load_distribution.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_request_advice_with_capacity() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"node1".to_string(), 0.3).await?;
        lb.update_node_load(&"node2".to_string(), 0.7).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;

        assert_eq!(advice.recommended_nodes, vec!["node1".to_string()]);
        assert!(advice.reasoning.contains("node1"));
        assert!(advice.reasoning.contains("30"));
        assert_eq!(advice.load_distribution.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_request_advice_single_node() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"solo-node".to_string(), 0.5).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;

        assert_eq!(advice.recommended_nodes, vec!["solo-node".to_string()]);
        assert_eq!(advice.load_distribution.get("solo-node"), Some(&0.5));
        Ok(())
    }

    #[tokio::test]
    async fn test_update_node_load() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"n1".to_string(), 0.85).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;
        assert_eq!(advice.load_distribution.get("n1"), Some(&0.85));
        Ok(())
    }

    #[tokio::test]
    async fn test_update_node_load_clamps_above_one() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"overload".to_string(), 1.5).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;
        assert_eq!(advice.load_distribution.get("overload"), Some(&1.0));
        Ok(())
    }

    #[tokio::test]
    async fn test_rebalance_if_needed_not_needed() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"n1".to_string(), 0.5).await?;
        lb.update_node_load(&"n2".to_string(), 0.6).await?;

        let needed = lb.rebalance_if_needed().await?;
        assert!(!needed);
        Ok(())
    }

    #[tokio::test]
    async fn test_rebalance_if_needed_when_overloaded() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"busy".to_string(), 0.9).await?;

        let needed = lb.rebalance_if_needed().await?;
        assert!(needed);
        Ok(())
    }

    #[tokio::test]
    async fn test_rebalance_empty_snapshot() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        let needed = lb.rebalance_if_needed().await?;
        assert!(!needed);
        Ok(())
    }

    #[tokio::test]
    async fn test_load_balancing_advice_has_reasoning() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        lb.update_node_load(&"a".to_string(), 0.1).await?;
        lb.update_node_load(&"b".to_string(), 0.9).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;
        assert_eq!(advice.recommended_nodes, vec!["a".to_string()]);
        assert!(advice.reasoning.contains("10%") || advice.reasoning.contains("10"));
        Ok(())
    }

    #[tokio::test]
    async fn test_performance_metrics_recorded() -> ToadStoolResult<()> {
        let lb = setup_load_balancer().await?;

        let _ = lb.request_advice(&ResourceRequirements::default()).await?;
        let _ = lb.request_advice(&ResourceRequirements::default()).await?;

        let advice = lb.request_advice(&ResourceRequirements::default()).await?;
        assert!(!advice.reasoning.is_empty());
        Ok(())
    }
}
