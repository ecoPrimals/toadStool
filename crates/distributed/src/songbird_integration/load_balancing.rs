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
