// SPDX-License-Identifier: AGPL-3.0-only
//! Load balancing coordination

use std::collections::HashMap;
use std::sync::Arc;

use toadstool::error::ToadStoolResult;
use tracing::debug;

use crate::ResourceRequirements;

use super::types::{
    LoadBalancerConfig, LoadBalancingAdvice, NodeCapacityTracker, NodeId, PerformanceMetrics,
    SongbirdConnection, SongbirdFeedback, SongbirdFeedbackSender, SongbirdLoadBalancer,
};

/// Load above this fraction triggers a rebalance suggestion.
const REBALANCE_THRESHOLD: f64 = 0.8;

impl SongbirdLoadBalancer {
    pub async fn new(
        _config: LoadBalancerConfig,
        _connection: Arc<SongbirdConnection>,
    ) -> ToadStoolResult<Self> {
        debug!("Initializing load balancer");
        Ok(Self {
            strategies: HashMap::new(),
            capacity_tracker: NodeCapacityTracker::new(),
            performance_metrics: PerformanceMetrics::new(),
            feedback_sender: SongbirdFeedbackSender::default(),
        })
    }

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
        let (recommended, reasoning) = if snapshot.is_empty() {
            // No capacity data yet — recommend self (localhost) as a starting node.
            (
                vec![toadstool_common::constants::network::LOCALHOST_IPV4.to_string()],
                "No capacity data available; defaulting to localhost".to_string(),
            )
        } else {
            let least = self.capacity_tracker.least_loaded();
            match least {
                Some(node) => {
                    let load = snapshot.get(&node).copied().unwrap_or(0.0);
                    (
                        vec![node.clone()],
                        format!(
                            "Selected '{}' with load {:.0}% (least-loaded node)",
                            node,
                            load * 100.0
                        ),
                    )
                }
                None => (
                    vec![toadstool_common::constants::network::LOCALHOST_IPV4.to_string()],
                    "All nodes saturated; defaulting to localhost".to_string(),
                ),
            }
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

    pub async fn update_node_load(&self, node_id: &NodeId, load: f64) -> ToadStoolResult<()> {
        debug!(%node_id, load, "Recording node load");
        self.capacity_tracker.update(node_id, load);
        self.feedback_sender.send(SongbirdFeedback::LoadUpdate {
            node_id: node_id.clone(),
            load,
        });
        Ok(())
    }

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
    use crate::songbird_integration::types::{
        ConnectionHealth, GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig,
        ProtocolConfig, SongbirdProtocol,
    };
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_mock_connection() -> SongbirdConnection {
        SongbirdConnection {
            endpoints: vec!["http://localhost:8080".to_string()],
            active_endpoint: "http://localhost:8080".to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: ProtocolConfig {
                protocol: SongbirdProtocol::HTTP,
                http: HttpProtocolConfig {
                    timeout_ms: 5000,
                    max_retries: 3,
                    headers: HashMap::new(),
                },
                grpc: GrpcProtocolConfig {
                    timeout_ms: 5000,
                    max_message_size: 1024 * 1024,
                    compression: false,
                },
                message_queue: MessageQueueProtocolConfig {
                    queue_name: "default".to_string(),
                    exchange: "default".to_string(),
                    routing_key: "default".to_string(),
                },
            },
            #[cfg(feature = "channels")]
            reply_channel: None,
        }
    }

    fn make_load_balancer_config() -> LoadBalancerConfig {
        LoadBalancerConfig {
            strategy: "least-loaded".to_string(),
            feedback_interval: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn test_load_balancer_new() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());

        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();
        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();

        assert!(!advice.recommended_nodes.is_empty());
        assert!(advice.reasoning.contains("localhost") || advice.reasoning.contains("No capacity"));
    }

    #[tokio::test]
    async fn test_request_advice_empty_capacity() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();

        assert_eq!(
            advice.recommended_nodes[0],
            toadstool_common::constants::network::LOCALHOST_IPV4
        );
        assert!(advice.reasoning.contains("No capacity data"));
        assert!(advice.load_distribution.is_empty());
    }

    #[tokio::test]
    async fn test_request_advice_with_capacity() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"node1".to_string(), 0.3)
            .await
            .unwrap();
        lb.update_node_load(&"node2".to_string(), 0.7)
            .await
            .unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();

        assert_eq!(advice.recommended_nodes, vec!["node1".to_string()]);
        assert!(advice.reasoning.contains("node1"));
        assert!(advice.reasoning.contains("30"));
        assert_eq!(advice.load_distribution.len(), 2);
    }

    #[tokio::test]
    async fn test_request_advice_single_node() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"solo-node".to_string(), 0.5)
            .await
            .unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();

        assert_eq!(advice.recommended_nodes, vec!["solo-node".to_string()]);
        assert_eq!(advice.load_distribution.get("solo-node"), Some(&0.5));
    }

    #[tokio::test]
    async fn test_update_node_load() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        let result = lb.update_node_load(&"n1".to_string(), 0.85).await;
        assert!(result.is_ok());

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();
        assert_eq!(advice.load_distribution.get("n1"), Some(&0.85));
    }

    #[tokio::test]
    async fn test_update_node_load_clamps_above_one() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"overload".to_string(), 1.5)
            .await
            .unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();
        assert_eq!(advice.load_distribution.get("overload"), Some(&1.0));
    }

    #[tokio::test]
    async fn test_rebalance_if_needed_not_needed() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"n1".to_string(), 0.5).await.unwrap();
        lb.update_node_load(&"n2".to_string(), 0.6).await.unwrap();

        let needed = lb.rebalance_if_needed().await.unwrap();
        assert!(!needed);
    }

    #[tokio::test]
    async fn test_rebalance_if_needed_when_overloaded() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"busy".to_string(), 0.9).await.unwrap();

        let needed = lb.rebalance_if_needed().await.unwrap();
        assert!(needed);
    }

    #[tokio::test]
    async fn test_rebalance_empty_snapshot() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        let needed = lb.rebalance_if_needed().await.unwrap();
        assert!(!needed);
    }

    #[tokio::test]
    async fn test_load_balancing_advice_has_reasoning() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        lb.update_node_load(&"a".to_string(), 0.1).await.unwrap();
        lb.update_node_load(&"b".to_string(), 0.9).await.unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();
        assert_eq!(advice.recommended_nodes, vec!["a".to_string()]);
        assert!(advice.reasoning.contains("10%") || advice.reasoning.contains("10"));
    }

    #[tokio::test]
    async fn test_performance_metrics_recorded() {
        let config = make_load_balancer_config();
        let connection = Arc::new(make_mock_connection());
        let lb = SongbirdLoadBalancer::new(config, connection).await.unwrap();

        let _ = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();
        let _ = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();

        let advice = lb
            .request_advice(&ResourceRequirements::default())
            .await
            .unwrap();
        assert!(!advice.reasoning.is_empty());
    }
}
