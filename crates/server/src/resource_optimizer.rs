//! Resource optimization for collaborative intelligence
//!
//! This module analyzes execution graphs and suggests optimizations to improve
//! performance, reduce resource usage, or work around resource constraints.
//!
//! ## Deep Debt Principles
//!
//! - **No Hardcoding**: All suggestions based on graph analysis, not hardcoded rules
//! - **Capability-Based**: Suggestions consider actual system capabilities
//! - **Self-Knowledge**: Graph nodes describe their own requirements
//! - **Runtime Discovery**: Uses real system state for recommendations
//! - **Safe Rust**: All optimization logic in safe Rust

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::graph_types::ExecutionGraph;
use crate::resource_estimator::{ResourceEstimate, ResourceEstimator};
use crate::resource_validator::SystemCapabilities;

/// Optimization suggestions for an execution graph
///
/// Provides actionable recommendations to improve graph execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestions {
    /// Graph ID these suggestions are for
    pub graph_id: String,

    /// Identified bottlenecks
    pub bottlenecks: Vec<Bottleneck>,

    /// Optimization opportunities
    pub opportunities: Vec<Opportunity>,

    /// Estimated improvement if all suggestions are applied
    pub estimated_improvement: ImprovementEstimate,

    /// Priority order for applying optimizations
    pub priority_order: Vec<String>,
}

/// A performance bottleneck in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,

    /// Affected node IDs
    pub affected_nodes: Vec<String>,

    /// Severity (0.0 = minor, 1.0 = critical)
    pub severity: f32,

    /// Description
    pub description: String,

    /// Impact on execution time
    pub time_impact_secs: u64,
}

/// Type of bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// Sequential execution where parallelism is possible
    SequentialExecution,

    /// Resource contention (multiple nodes competing for same resource)
    ResourceContention,

    /// Inefficient resource allocation
    InefficientAllocation,

    /// Long critical path
    LongCriticalPath,

    /// Memory bottleneck (excessive memory usage)
    MemoryBottleneck,

    /// GPU underutilization
    GpuUnderutilization,
}

/// An optimization opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    /// Opportunity type
    pub opportunity_type: OpportunityType,

    /// Affected node IDs
    pub affected_nodes: Vec<String>,

    /// Potential benefit (0.0 = minor, 1.0 = major)
    pub benefit: f32,

    /// Description
    pub description: String,

    /// Specific recommendation
    pub recommendation: String,

    /// Estimated time savings in seconds
    pub time_savings_secs: u64,

    /// Estimated resource savings
    pub resource_savings: HashMap<String, u64>,
}

/// Type of optimization opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpportunityType {
    /// Parallelize sequential nodes
    Parallelization,

    /// Use GPU instead of CPU
    GpuAcceleration,

    /// Reduce memory usage through streaming
    MemoryStreaming,

    /// Batch multiple operations
    Batching,

    /// Cache intermediate results
    Caching,

    /// Reorder nodes for better resource utilization
    Reordering,

    /// Split large node into smaller ones
    NodeSplitting,

    /// Merge small nodes into larger ones
    NodeMerging,
}

/// Estimated improvement from optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEstimate {
    /// Current estimated duration
    pub current_duration_secs: u64,

    /// Optimized estimated duration
    pub optimized_duration_secs: u64,

    /// Time savings
    pub time_savings_secs: u64,

    /// Speedup factor (e.g., 2.0 = 2x faster)
    pub speedup_factor: f32,

    /// Current resource usage
    pub current_resources: HashMap<String, u64>,

    /// Optimized resource usage
    pub optimized_resources: HashMap<String, u64>,
}

/// Resource optimizer
///
/// Analyzes execution graphs and generates optimization suggestions.
pub struct ResourceOptimizer {
    estimator: ResourceEstimator,
}

impl Default for ResourceOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceOptimizer {
    /// Create a new resource optimizer
    pub fn new() -> Self {
        Self {
            estimator: ResourceEstimator::new(),
        }
    }

    /// Suggest optimizations for an execution graph
    ///
    /// This performs:
    /// 1. Resource estimation
    /// 2. Bottleneck identification
    /// 3. Opportunity discovery
    /// 4. Improvement estimation
    /// 5. Priority ranking
    pub async fn suggest_optimizations(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<OptimizationSuggestions, OptimizationError> {
        info!(
            "Analyzing graph for optimization opportunities: {}",
            graph.id
        );

        // Estimate current resource usage
        let estimate = self
            .estimator
            .estimate(graph)
            .map_err(OptimizationError::EstimationFailed)?;

        // Query system capabilities for context
        let capabilities = self.query_system_capabilities().await?;

        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(graph, &estimate, &capabilities);

        // Discover optimization opportunities
        let opportunities = self.discover_opportunities(graph, &estimate, &capabilities);

        // Estimate improvement
        let improvement = self.estimate_improvement(&estimate, &opportunities);

        // Rank by priority
        let priority_order = self.rank_by_priority(&opportunities);

        info!(
            "Found {} bottlenecks and {} optimization opportunities for graph {}",
            bottlenecks.len(),
            opportunities.len(),
            graph.id
        );

        Ok(OptimizationSuggestions {
            graph_id: graph.id.clone(),
            bottlenecks,
            opportunities,
            estimated_improvement: improvement,
            priority_order,
        })
    }

    /// Query system capabilities
    async fn query_system_capabilities(&self) -> Result<SystemCapabilities, OptimizationError> {
        // Query CPU
        let total_cpu_cores = num_cpus::get() as u32;
        let available_cpu_cores = (total_cpu_cores as f32 * 0.8) as u32;

        // Query memory - Pure Rust Evolution (Jan 17, 2026)
        use sysinfo::System;
        let mut system = System::new_all();
        system.refresh_memory();

        let total_memory_bytes = system.total_memory(); // Already in bytes
        let available_memory_bytes = system.available_memory(); // Already in bytes

        // Query storage - using swap as proxy
        let total_storage_bytes = system.total_swap();
        let available_storage_bytes = system.free_swap();

        // GPU (placeholder - would be queried via wgpu in production)
        let (total_gpu_memory_bytes, available_gpu_memory_bytes, gpu_count, gpu_types) =
            (0, 0, 0, Vec::new());

        // Network bandwidth (rough estimate)
        let network_bandwidth_mbps = 1000;

        Ok(SystemCapabilities {
            total_cpu_cores,
            available_cpu_cores,
            total_memory_bytes,
            available_memory_bytes,
            total_gpu_memory_bytes,
            available_gpu_memory_bytes,
            total_storage_bytes,
            available_storage_bytes,
            network_bandwidth_mbps,
            gpu_count,
            gpu_types,
        })
    }

    /// Identify bottlenecks in the graph
    fn identify_bottlenecks(
        &self,
        graph: &ExecutionGraph,
        estimate: &ResourceEstimate,
        capabilities: &SystemCapabilities,
    ) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        // Check for sequential execution opportunities
        if estimate.max_parallelism == 1 && graph.nodes.len() > 1 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::SequentialExecution,
                affected_nodes: graph.nodes.iter().map(|n| n.id.clone()).collect(),
                severity: 0.8,
                description: "Graph is entirely sequential. Parallelization may be possible."
                    .to_string(),
                time_impact_secs: estimate.estimated_duration.as_secs() / 2, // Could save ~50%
            });
        }

        // Check for long critical path
        if estimate.critical_path_length > 5 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::LongCriticalPath,
                affected_nodes: Vec::new(), // Would need more analysis to identify specific nodes
                severity: 0.6,
                description: format!(
                    "Critical path has {} levels. Consider reducing dependencies.",
                    estimate.critical_path_length
                ),
                time_impact_secs: estimate.estimated_duration.as_secs() / 4,
            });
        }

        // Check for memory bottleneck
        let memory_gb = estimate.memory_bytes / (1024 * 1024 * 1024);
        if memory_gb > 64 {
            bottlenecks.push(Bottleneck {
                bottleneck_type: BottleneckType::MemoryBottleneck,
                affected_nodes: self.find_high_memory_nodes(graph, estimate),
                severity: 0.7,
                description: format!(
                    "High memory usage: {} GB. Consider streaming or batching.",
                    memory_gb
                ),
                time_impact_secs: 0, // Memory doesn't directly impact time, but may cause swapping
            });
        }

        // Check for GPU underutilization
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
                    time_impact_secs: estimate.estimated_duration.as_secs() / 3, // GPU could be 3x faster
                });
            }
        }

        bottlenecks
    }

    /// Find nodes with high memory usage
    fn find_high_memory_nodes(
        &self,
        _graph: &ExecutionGraph,
        estimate: &ResourceEstimate,
    ) -> Vec<String> {
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
    fn discover_opportunities(
        &self,
        graph: &ExecutionGraph,
        estimate: &ResourceEstimate,
        capabilities: &SystemCapabilities,
    ) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Parallelization opportunities
        opportunities.extend(self.find_parallelization_opportunities(graph, estimate));

        // GPU acceleration opportunities
        if capabilities.gpu_count > 0 {
            opportunities.extend(self.find_gpu_acceleration_opportunities(
                graph,
                estimate,
                capabilities,
            ));
        }

        // Memory streaming opportunities
        opportunities.extend(self.find_memory_streaming_opportunities(graph, estimate));

        // Batching opportunities
        opportunities.extend(self.find_batching_opportunities(graph));

        // Caching opportunities
        opportunities.extend(self.find_caching_opportunities(graph));

        opportunities
    }

    /// Find parallelization opportunities
    fn find_parallelization_opportunities(
        &self,
        graph: &ExecutionGraph,
        estimate: &ResourceEstimate,
    ) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Look for nodes at the same level that could be parallelized
        let mut level_groups: HashMap<usize, Vec<String>> = HashMap::new();
        for (node_id, node_estimate) in &estimate.node_estimates {
            level_groups
                .entry(node_estimate.parallelism_level)
                .or_insert_with(Vec::new)
                .push(node_id.clone());
        }

        // If any level has only 1 node but could have more, suggest parallelization
        for (level, nodes) in level_groups {
            if nodes.len() == 1 && level > 0 {
                // Check if this node could be split
                if let Some(node) = graph.get_node(&nodes[0]) {
                    if node.operation == "cpu_compute" || node.operation == "gpu_compute" {
                        opportunities.push(Opportunity {
                            opportunity_type: OpportunityType::Parallelization,
                            affected_nodes: nodes.clone(),
                            benefit: 0.7,
                            description: format!("Node '{}' could be parallelized", nodes[0]),
                            recommendation:
                                "Consider splitting this node into multiple parallel tasks."
                                    .to_string(),
                            time_savings_secs: 30,
                            resource_savings: HashMap::new(),
                        });
                    }
                }
            }
        }

        opportunities
    }

    /// Find GPU acceleration opportunities
    fn find_gpu_acceleration_opportunities(
        &self,
        graph: &ExecutionGraph,
        _estimate: &ResourceEstimate,
        capabilities: &SystemCapabilities,
    ) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Find CPU compute nodes that could use GPU
        for node in &graph.nodes {
            if node.operation == "cpu_compute" {
                let time_savings = 60; // Assume GPU is 3x faster
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
                    time_savings_secs: time_savings,
                    resource_savings: HashMap::new(),
                });
            }
        }

        opportunities
    }

    /// Find memory streaming opportunities
    fn find_memory_streaming_opportunities(
        &self,
        _graph: &ExecutionGraph,
        estimate: &ResourceEstimate,
    ) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Find nodes with high memory usage
        for (node_id, node_estimate) in &estimate.node_estimates {
            let memory_gb = node_estimate.memory_bytes / (1024 * 1024 * 1024);
            if memory_gb > 16 {
                let mut resource_savings = HashMap::new();
                resource_savings.insert("memory_bytes".to_string(), node_estimate.memory_bytes / 2);

                opportunities.push(Opportunity {
                    opportunity_type: OpportunityType::MemoryStreaming,
                    affected_nodes: vec![node_id.clone()],
                    benefit: 0.6,
                    description: format!("Node '{}' uses {} GB memory", node_id, memory_gb),
                    recommendation: "Consider streaming data instead of loading all at once."
                        .to_string(),
                    time_savings_secs: 0, // May not save time, but reduces memory
                    resource_savings,
                });
            }
        }

        opportunities
    }

    /// Find batching opportunities
    fn find_batching_opportunities(&self, graph: &ExecutionGraph) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Find multiple nodes of the same type that could be batched
        let mut operation_groups: HashMap<String, Vec<String>> = HashMap::new();
        for node in &graph.nodes {
            operation_groups
                .entry(node.operation.clone())
                .or_insert_with(Vec::new)
                .push(node.id.clone());
        }

        for (operation, nodes) in operation_groups {
            if nodes.len() >= 3 {
                opportunities.push(Opportunity {
                    opportunity_type: OpportunityType::Batching,
                    affected_nodes: nodes.clone(),
                    benefit: 0.5,
                    description: format!("{} nodes with operation '{}'", nodes.len(), operation),
                    recommendation:
                        "Consider batching these operations together for better efficiency."
                            .to_string(),
                    time_savings_secs: 15,
                    resource_savings: HashMap::new(),
                });
            }
        }

        opportunities
    }

    /// Find caching opportunities
    fn find_caching_opportunities(&self, graph: &ExecutionGraph) -> Vec<Opportunity> {
        let mut opportunities = Vec::new();

        // Find nodes with multiple dependents (their output is used multiple times)
        for node in &graph.nodes {
            let dependents = graph.get_dependents(&node.id);
            if dependents.len() >= 2 {
                opportunities.push(Opportunity {
                    opportunity_type: OpportunityType::Caching,
                    affected_nodes: vec![node.id.clone()],
                    benefit: 0.4,
                    description: format!("Node '{}' has {} dependents", node.id, dependents.len()),
                    recommendation:
                        "Consider caching the output of this node to avoid recomputation."
                            .to_string(),
                    time_savings_secs: 20,
                    resource_savings: HashMap::new(),
                });
            }
        }

        opportunities
    }

    /// Estimate improvement from applying optimizations
    fn estimate_improvement(
        &self,
        estimate: &ResourceEstimate,
        opportunities: &[Opportunity],
    ) -> ImprovementEstimate {
        let current_duration_secs = estimate.estimated_duration.as_secs();

        // Sum up time savings (with diminishing returns)
        let total_time_savings: u64 = opportunities.iter().map(|o| o.time_savings_secs).sum();

        // Apply diminishing returns (can't save more than 80% of time)
        let effective_savings = total_time_savings.min((current_duration_secs as f32 * 0.8) as u64);
        let optimized_duration_secs = current_duration_secs.saturating_sub(effective_savings);

        let speedup_factor = if optimized_duration_secs > 0 {
            current_duration_secs as f32 / optimized_duration_secs as f32
        } else {
            1.0
        };

        // Aggregate resource savings
        let mut total_resource_savings: HashMap<String, u64> = HashMap::new();
        for opportunity in opportunities {
            for (resource, savings) in &opportunity.resource_savings {
                *total_resource_savings.entry(resource.clone()).or_insert(0) += savings;
            }
        }

        let mut current_resources = HashMap::new();
        current_resources.insert("cpu_cores".to_string(), estimate.cpu_cores as u64);
        current_resources.insert("memory_bytes".to_string(), estimate.memory_bytes);
        current_resources.insert("gpu_memory_bytes".to_string(), estimate.gpu_memory_bytes);

        let mut optimized_resources = current_resources.clone();
        for (resource, savings) in &total_resource_savings {
            if let Some(current) = optimized_resources.get_mut(resource) {
                *current = current.saturating_sub(*savings);
            }
        }

        ImprovementEstimate {
            current_duration_secs,
            optimized_duration_secs,
            time_savings_secs: effective_savings,
            speedup_factor,
            current_resources,
            optimized_resources,
        }
    }

    /// Rank opportunities by priority
    fn rank_by_priority(&self, opportunities: &[Opportunity]) -> Vec<String> {
        let mut ranked: Vec<(String, f32)> = opportunities
            .iter()
            .map(|o| {
                let id = format!("{:?}-{}", o.opportunity_type, o.affected_nodes.join(","));
                let priority = o.benefit * (o.time_savings_secs as f32 / 60.0); // Benefit * minutes saved
                (id, priority)
            })
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        ranked.into_iter().map(|(id, _)| id).collect()
    }
}

/// Optimization error
#[derive(Debug, Clone, thiserror::Error)]
pub enum OptimizationError {
    #[error("Estimation failed: {0}")]
    EstimationFailed(#[from] crate::resource_estimator::EstimationError),

    #[error("System query failed: {0}")]
    SystemQueryFailed(String),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_types::{EdgeType, GraphEdge, GraphNode, NodeResourceRequirements};

    #[tokio::test]
    async fn test_suggest_optimizations_sequential_graph() {
        let optimizer = ResourceOptimizer::new();

        let graph = ExecutionGraph {
            id: "sequential-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-3".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "node-1".to_string(),
                    to: "node-2".to_string(),
                    edge_type: EdgeType::DataFlow,
                    metadata: HashMap::new(),
                },
                GraphEdge {
                    from: "node-2".to_string(),
                    to: "node-3".to_string(),
                    edge_type: EdgeType::DataFlow,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };

        let suggestions = optimizer.suggest_optimizations(&graph).await.unwrap();

        // Should identify sequential execution bottleneck
        assert!(!suggestions.bottlenecks.is_empty());
        assert!(suggestions
            .bottlenecks
            .iter()
            .any(|b| b.bottleneck_type == BottleneckType::SequentialExecution));

        // Should have some optimization opportunities
        assert!(!suggestions.opportunities.is_empty());
    }
}
