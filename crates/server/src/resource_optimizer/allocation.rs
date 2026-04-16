// SPDX-License-Identifier: AGPL-3.0-or-later
//! Allocation strategies: bottleneck identification and opportunity discovery

use std::collections::HashMap;

use crate::graph_types::ExecutionGraph;
use crate::resource_estimator::ResourceEstimate;
use crate::resource_validator::SystemCapabilities;

use super::types::{Bottleneck, BottleneckType, Opportunity, OpportunityType};

/// Identify bottlenecks in the graph
#[must_use]
pub fn identify_bottlenecks(
    graph: &ExecutionGraph,
    estimate: &ResourceEstimate,
    capabilities: &SystemCapabilities,
) -> Vec<Bottleneck> {
    let mut bottlenecks = Vec::new();

    if estimate.max_parallelism == 1 && graph.nodes.len() > 1 {
        bottlenecks.push(Bottleneck {
            bottleneck_type: BottleneckType::SequentialExecution,
            affected_nodes: graph.nodes.iter().map(|n| n.id.clone()).collect(),
            severity: 0.8,
            description: "Graph is entirely sequential. Parallelization may be possible."
                .to_string(),
            time_impact_secs: estimate.estimated_duration.as_secs() / 2,
        });
    }

    if estimate.critical_path_length > 5 {
        bottlenecks.push(Bottleneck {
            bottleneck_type: BottleneckType::LongCriticalPath,
            affected_nodes: Vec::new(),
            severity: 0.6,
            description: format!(
                "Critical path has {} levels. Consider reducing dependencies.",
                estimate.critical_path_length
            ),
            time_impact_secs: estimate.estimated_duration.as_secs() / 4,
        });
    }

    let memory_gb = estimate.memory_bytes / (1024 * 1024 * 1024);
    if memory_gb > 64 {
        bottlenecks.push(Bottleneck {
            bottleneck_type: BottleneckType::MemoryBottleneck,
            affected_nodes: find_high_memory_nodes(estimate),
            severity: 0.7,
            description: format!(
                "High memory usage: {memory_gb} GB. Consider streaming or batching."
            ),
            time_impact_secs: 0,
        });
    }

    if capabilities.gpu_count > 0 && estimate.gpu_memory_bytes == 0 {
        let cpu_nodes: Vec<String> = graph
            .nodes
            .iter()
            .filter(|n| n.operation == "cpu_compute")
            .map(|n| n.id.clone())
            .collect();
        if !cpu_nodes.is_empty() {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::GpuUnderutilization,
                affected_nodes: cpu_nodes,
                severity: 0.5,
                description: format!(
                    "GPU available ({} GPUs) but not used. Consider GPU acceleration.",
                    capabilities.gpu_count
                ),
                time_impact_secs: estimate.estimated_duration.as_secs() / 3,
            });
        }
    }

    bottlenecks
}

fn find_high_memory_nodes(estimate: &ResourceEstimate) -> Vec<String> {
    let mut high_memory_nodes = Vec::new();
    for (node_id, node_estimate) in &estimate.node_estimates {
        let memory_gb = node_estimate.memory_bytes / (1024 * 1024 * 1024);
        if memory_gb > 16 {
            high_memory_nodes.push(node_id.clone());
        }
    }
    high_memory_nodes
}

/// Discover optimization opportunities
#[must_use]
pub fn discover_opportunities(
    graph: &ExecutionGraph,
    estimate: &ResourceEstimate,
    capabilities: &SystemCapabilities,
) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    opportunities.extend(find_parallelization_opportunities(graph, estimate));
    if capabilities.gpu_count > 0 {
        opportunities.extend(find_gpu_acceleration_opportunities(graph, capabilities));
    }
    opportunities.extend(find_memory_streaming_opportunities(estimate));
    opportunities.extend(find_batching_opportunities(graph));
    opportunities.extend(find_caching_opportunities(graph));
    opportunities
}

fn find_parallelization_opportunities(
    graph: &ExecutionGraph,
    estimate: &ResourceEstimate,
) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    let mut level_groups: HashMap<usize, Vec<String>> = HashMap::new();
    for (node_id, node_estimate) in &estimate.node_estimates {
        level_groups
            .entry(node_estimate.parallelism_level)
            .or_default()
            .push(node_id.clone());
    }
    for (level, nodes) in level_groups {
        if nodes.len() == 1 && level > 0 {
            let first_node_id = nodes[0].clone();
            if let Some(node) = graph.get_node(&first_node_id)
                && (node.operation == "cpu_compute" || node.operation == "gpu_compute")
            {
                opportunities.push(Opportunity {
                    opportunity_type: OpportunityType::Parallelization,
                    affected_nodes: nodes,
                    benefit: 0.7,
                    description: format!("Node '{first_node_id}' could be parallelized"),
                    recommendation: "Consider splitting this node into multiple parallel tasks."
                        .to_string(),
                    time_savings_secs: 30,
                    resource_savings: HashMap::new(),
                });
            }
        }
    }
    opportunities
}

fn find_gpu_acceleration_opportunities(
    graph: &ExecutionGraph,
    capabilities: &SystemCapabilities,
) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    for node in &graph.nodes {
        if node.operation == "cpu_compute" {
            opportunities.push(Opportunity {
                opportunity_type: OpportunityType::GpuAcceleration,
                affected_nodes: vec![node.id.clone()],
                benefit: 0.8,
                description: format!("Node '{}' could use GPU acceleration", node.id),
                recommendation: format!(
                    "Consider moving this workload to GPU. {} GPU(s) available: {}",
                    capabilities.gpu_count,
                    capabilities.gpu_types.join(", ")
                ),
                time_savings_secs: 60,
                resource_savings: HashMap::new(),
            });
        }
    }
    opportunities
}

fn find_memory_streaming_opportunities(estimate: &ResourceEstimate) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    for (node_id, node_estimate) in &estimate.node_estimates {
        let memory_gb = node_estimate.memory_bytes / (1024 * 1024 * 1024);
        if memory_gb > 16 {
            let mut resource_savings = HashMap::new();
            resource_savings.insert("memory_bytes".to_string(), node_estimate.memory_bytes / 2);
            opportunities.push(Opportunity {
                opportunity_type: OpportunityType::MemoryStreaming,
                affected_nodes: vec![node_id.clone()],
                benefit: 0.6,
                description: format!("Node '{node_id}' uses {memory_gb} GB memory"),
                recommendation: "Consider streaming data instead of loading all at once."
                    .to_string(),
                time_savings_secs: 0,
                resource_savings,
            });
        }
    }
    opportunities
}

fn find_batching_opportunities(graph: &ExecutionGraph) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    let mut operation_groups: HashMap<String, Vec<String>> = HashMap::new();
    for node in &graph.nodes {
        operation_groups
            .entry(node.operation.clone())
            .or_default()
            .push(node.id.clone());
    }
    for (operation, nodes) in operation_groups {
        if nodes.len() >= 3 {
            let node_count = nodes.len();
            opportunities.push(Opportunity {
                opportunity_type: OpportunityType::Batching,
                affected_nodes: nodes,
                benefit: 0.5,
                description: format!("{node_count} nodes with operation '{operation}'"),
                recommendation: "Consider batching these operations together for better efficiency."
                    .to_string(),
                time_savings_secs: 15,
                resource_savings: HashMap::new(),
            });
        }
    }
    opportunities
}

fn find_caching_opportunities(graph: &ExecutionGraph) -> Vec<Opportunity> {
    let mut opportunities = Vec::new();
    for node in &graph.nodes {
        let dependents = graph.get_dependents(&node.id);
        if dependents.len() >= 2 {
            opportunities.push(Opportunity {
                opportunity_type: OpportunityType::Caching,
                affected_nodes: vec![node.id.clone()],
                benefit: 0.4,
                description: format!("Node '{}' has {} dependents", node.id, dependents.len()),
                recommendation: "Consider caching the output of this node to avoid recomputation."
                    .to_string(),
                time_savings_secs: 20,
                resource_savings: HashMap::new(),
            });
        }
    }
    opportunities
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::graph_types::{ExecutionGraph, GraphNode};
    use crate::resource_estimator::{ResourceEstimate, ResourceEstimator};
    use crate::resource_validator::SystemCapabilities;

    use super::{discover_opportunities, identify_bottlenecks};
    use crate::resource_optimizer::types::{BottleneckType, OpportunityType};

    fn caps(gpu_count: usize) -> SystemCapabilities {
        SystemCapabilities {
            total_cpu_cores: 16,
            available_cpu_cores: 8,
            total_memory_bytes: 64 * 1024 * 1024 * 1024,
            available_memory_bytes: 32 * 1024 * 1024 * 1024,
            total_gpu_memory_bytes: 8 * 1024 * 1024 * 1024,
            available_gpu_memory_bytes: 6 * 1024 * 1024 * 1024,
            total_storage_bytes: 256 * 1024 * 1024 * 1024,
            available_storage_bytes: 128 * 1024 * 1024 * 1024,
            network_bandwidth_mbps: 1000,
            gpu_count,
            gpu_types: if gpu_count > 0 {
                vec!["TestGPU".into()]
            } else {
                vec![]
            },
        }
    }

    #[test]
    fn empty_graph_yields_no_opportunities() {
        let graph = ExecutionGraph::simple("empty");
        let estimate = ResourceEstimate {
            graph_id: graph.id.clone(),
            cpu_cores: 0,
            memory_bytes: 0,
            gpu_memory_bytes: 0,
            storage_bytes: 0,
            network_bandwidth_mbps: 0,
            estimated_duration: Duration::ZERO,
            max_parallelism: 0,
            critical_path_length: 0,
            node_estimates: HashMap::new(),
            warnings: vec![],
        };
        let opps = discover_opportunities(&graph, &estimate, &caps(0));
        assert!(opps.is_empty());
    }

    #[test]
    fn sequential_two_node_graph_triggers_sequential_execution_bottleneck() {
        let graph = ExecutionGraph::builder("seq")
            .nodes([
                GraphNode::simple("a", "cpu_compute"),
                GraphNode::simple("b", "cpu_compute"),
            ])
            .connect("a", "b")
            .build();
        let estimator = ResourceEstimator::new();
        let estimate = estimator.estimate(&graph).unwrap();
        let bottlenecks = identify_bottlenecks(&graph, &estimate, &caps(0));
        assert!(
            bottlenecks
                .iter()
                .any(|b| b.bottleneck_type == BottleneckType::SequentialExecution)
        );
    }

    #[test]
    fn diamond_graph_has_parallel_fan_and_caching_on_shared_predecessor() {
        let graph = ExecutionGraph::builder("diamond")
            .nodes([
                GraphNode::simple("root", "cpu_compute"),
                GraphNode::simple("left", "cpu_compute"),
                GraphNode::simple("right", "cpu_compute"),
                GraphNode::simple("join", "cpu_compute"),
            ])
            .connect("root", "left")
            .connect("root", "right")
            .connect("left", "join")
            .connect("right", "join")
            .build();
        let estimator = ResourceEstimator::new();
        let estimate = estimator.estimate(&graph).unwrap();
        assert_eq!(estimate.max_parallelism, 2);
        assert_eq!(estimate.critical_path_length, 3);

        let opps = discover_opportunities(&graph, &estimate, &caps(0));
        assert!(opps.iter().any(|o| {
            o.opportunity_type == OpportunityType::Caching
                && o.affected_nodes == vec!["root".to_string()]
        }));
    }
}
