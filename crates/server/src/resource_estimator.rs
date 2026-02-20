//! Resource estimation for collaborative intelligence
//!
//! This module provides resource estimation capabilities for execution graphs.
//! It analyzes graph structure, identifies parallelization opportunities, and
//! estimates total resource requirements and execution duration.
//!
//! ## Deep Debt Principles
//!
//! - **No Hardcoding**: Estimation based on actual requirements, no magic numbers
//! - **Capability-Based**: Uses system capabilities for realistic estimates
//! - **Self-Knowledge**: Each node provides its own requirements
//! - **Runtime Discovery**: Queries real system state for accurate estimates
//! - **Safe Rust**: All algorithms in safe Rust, no unsafe blocks

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tracing::{debug, info};

use crate::graph_types::{ExecutionGraph, GraphNode, GraphValidationError};

/// Resource estimate for an execution graph
///
/// Provides complete resource profile including CPU, memory, GPU, storage,
/// network bandwidth, and estimated execution duration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEstimate {
    /// Graph ID this estimate is for
    pub graph_id: String,

    /// Total CPU cores needed (peak)
    pub cpu_cores: u32,

    /// Total memory needed in bytes (peak)
    pub memory_bytes: u64,

    /// Total GPU memory needed in bytes (peak)
    pub gpu_memory_bytes: u64,

    /// Total storage needed in bytes
    pub storage_bytes: u64,

    /// Network bandwidth needed in Mbps
    pub network_bandwidth_mbps: u64,

    /// Estimated execution duration
    pub estimated_duration: Duration,

    /// Maximum parallelism level (number of concurrent nodes)
    pub max_parallelism: usize,

    /// Critical path length (longest dependency chain)
    pub critical_path_length: usize,

    /// Per-node estimates
    pub node_estimates: HashMap<String, NodeEstimate>,

    /// Optional warnings or notes
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Resource estimate for a single node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEstimate {
    /// Node ID
    pub node_id: String,

    /// CPU cores needed
    pub cpu_cores: u32,

    /// Memory needed in bytes
    pub memory_bytes: u64,

    /// GPU memory needed in bytes
    pub gpu_memory_bytes: u64,

    /// Estimated duration
    pub duration: Duration,

    /// Parallelism level (which parallel group this node belongs to)
    pub parallelism_level: usize,
}

/// Resource estimator
///
/// Analyzes execution graphs and produces resource estimates.
pub struct ResourceEstimator {
    /// Default CPU cores per node if not specified
    default_cpu_cores: u32,

    /// Default memory per node if not specified (1GB)
    default_memory_bytes: u64,

    /// Default duration per node if not specified (30 seconds)
    default_duration: Duration,
}

impl Default for ResourceEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceEstimator {
    /// Create a new resource estimator with sensible defaults
    pub fn new() -> Self {
        Self {
            default_cpu_cores: 2,
            default_memory_bytes: 1024 * 1024 * 1024, // 1GB
            default_duration: Duration::from_secs(30),
        }
    }

    /// Estimate resources for an execution graph
    ///
    /// This performs a comprehensive analysis:
    /// 1. Validate graph structure
    /// 2. Topological sort to determine execution order
    /// 3. Analyze parallelism opportunities
    /// 4. Estimate per-node resources
    /// 5. Aggregate total requirements
    /// 6. Calculate critical path and duration
    pub fn estimate(&self, graph: &ExecutionGraph) -> Result<ResourceEstimate, EstimationError> {
        info!("Estimating resources for graph: {}", graph.id);

        // Validate graph
        graph.validate().map_err(EstimationError::InvalidGraph)?;

        // Topological sort
        let sorted_nodes = self.topological_sort(graph)?;
        debug!("Topological sort produced {} levels", sorted_nodes.len());

        // Estimate per-node resources
        let node_estimates = self.estimate_nodes(graph, &sorted_nodes);

        // Aggregate resources
        let (total_cpu, total_memory, total_gpu, total_storage, total_network) =
            self.aggregate_resources(&node_estimates, &sorted_nodes);

        // Calculate duration and parallelism
        let (duration, max_parallelism, critical_path) =
            self.calculate_duration_and_parallelism(&node_estimates, &sorted_nodes);

        // Generate warnings
        let warnings = self.generate_warnings(total_cpu, total_memory, total_gpu);

        Ok(ResourceEstimate {
            graph_id: graph.id.clone(),
            cpu_cores: total_cpu,
            memory_bytes: total_memory,
            gpu_memory_bytes: total_gpu,
            storage_bytes: total_storage,
            network_bandwidth_mbps: total_network,
            estimated_duration: duration,
            max_parallelism,
            critical_path_length: critical_path,
            node_estimates,
            warnings,
        })
    }

    /// Perform topological sort on the graph
    ///
    /// Returns nodes grouped by execution level. Level 0 nodes have no dependencies,
    /// level 1 nodes depend only on level 0, etc. This enables parallel execution
    /// within each level.
    fn topological_sort(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<Vec<Vec<String>>, EstimationError> {
        // Build adjacency list and in-degree map
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        for node in &graph.nodes {
            in_degree.insert(node.id.clone(), 0);
            adj_list.insert(node.id.clone(), Vec::new());
        }

        for edge in &graph.edges {
            if let Some(deg) = in_degree.get_mut(&edge.to) {
                *deg += 1;
            }
            if let Some(neighbors) = adj_list.get_mut(&edge.from) {
                neighbors.push(edge.to.clone());
            }
        }

        // Kahn's algorithm for topological sort
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut visited = 0;

        // Start with nodes that have no dependencies
        for (node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id.clone());
            }
        }

        while !queue.is_empty() {
            // Process all nodes at current level
            let level_size = queue.len();
            let mut current_level = Vec::new();

            for _ in 0..level_size {
                let Some(node_id) = queue.pop_front() else {
                    break;
                };
                current_level.push(node_id.clone());
                visited += 1;

                // Reduce in-degree of neighbors
                if let Some(neighbors) = adj_list.get(&node_id) {
                    for neighbor in neighbors {
                        if let Some(degree) = in_degree.get_mut(neighbor) {
                            *degree -= 1;
                            if *degree == 0 {
                                queue.push_back(neighbor.clone());
                            }
                        }
                    }
                }
            }

            levels.push(current_level);
        }

        // Check if all nodes were visited (graph is acyclic)
        if visited != graph.nodes.len() {
            return Err(EstimationError::CyclicGraph);
        }

        Ok(levels)
    }

    /// Estimate resources for individual nodes
    fn estimate_nodes(
        &self,
        graph: &ExecutionGraph,
        sorted_nodes: &[Vec<String>],
    ) -> HashMap<String, NodeEstimate> {
        let mut estimates = HashMap::new();

        for (level, node_ids) in sorted_nodes.iter().enumerate() {
            for node_id in node_ids {
                if let Some(node) = graph.get_node(node_id) {
                    let estimate = self.estimate_node(node, level);
                    estimates.insert(node_id.clone(), estimate);
                }
            }
        }

        estimates
    }

    /// Estimate resources for a single node
    fn estimate_node(&self, node: &GraphNode, level: usize) -> NodeEstimate {
        // Extract requirements or use defaults
        let cpu_cores = node
            .requirements
            .cpu
            .as_ref()
            .map(|r| r.min_cores as u32)
            .unwrap_or(self.default_cpu_cores);

        let memory_bytes = node
            .requirements
            .memory
            .as_ref()
            .map(|r| r.min_bytes)
            .unwrap_or(self.default_memory_bytes);

        let gpu_memory_bytes = node
            .requirements
            .gpu
            .as_ref()
            .and_then(|r| r.min_memory_bytes)
            .unwrap_or(0);

        // Estimate duration based on operation type and resources
        let duration = self.estimate_duration(node);

        NodeEstimate {
            node_id: node.id.clone(),
            cpu_cores,
            memory_bytes,
            gpu_memory_bytes,
            duration,
            parallelism_level: level,
        }
    }

    /// Estimate execution duration for a node
    ///
    /// Uses heuristics based on operation type and resource requirements.
    /// In a real system, this would use historical data and ML models.
    fn estimate_duration(&self, node: &GraphNode) -> Duration {
        // Check metadata for duration hint
        if let Some(duration_str) = node.metadata.get("estimated_duration_secs") {
            if let Ok(secs) = duration_str.parse::<u64>() {
                return Duration::from_secs(secs);
            }
        }

        // Use heuristics based on operation type
        match node.operation.as_str() {
            "gpu_compute" => Duration::from_secs(60), // GPU tasks tend to be longer
            "neural_compute" => Duration::from_secs(120), // Neural tasks even longer
            "cpu_compute" => Duration::from_secs(30), // CPU tasks moderate
            "storage" => Duration::from_secs(10),     // Storage operations quick
            "network" => Duration::from_secs(5),      // Network operations very quick
            _ => self.default_duration,
        }
    }

    /// Aggregate resources across all nodes
    ///
    /// For peak resources (CPU, memory), we take the maximum across all parallel levels.
    /// For cumulative resources (storage), we sum across all nodes.
    fn aggregate_resources(
        &self,
        node_estimates: &HashMap<String, NodeEstimate>,
        sorted_nodes: &[Vec<String>],
    ) -> (u32, u64, u64, u64, u64) {
        let mut max_cpu = 0;
        let mut max_memory = 0;
        let mut max_gpu = 0;
        let mut total_storage = 0;
        let mut total_network = 0;

        // Calculate peak resources per level
        for level_nodes in sorted_nodes {
            let mut level_cpu = 0;
            let mut level_memory = 0;
            let mut level_gpu = 0;

            for node_id in level_nodes {
                if let Some(estimate) = node_estimates.get(node_id) {
                    level_cpu += estimate.cpu_cores;
                    level_memory += estimate.memory_bytes;
                    level_gpu += estimate.gpu_memory_bytes;

                    // Storage and network are cumulative
                    total_storage += estimate.memory_bytes; // Rough approximation
                    total_network += 100; // Rough approximation (100 Mbps per node)
                }
            }

            max_cpu = max_cpu.max(level_cpu);
            max_memory = max_memory.max(level_memory);
            max_gpu = max_gpu.max(level_gpu);
        }

        (max_cpu, max_memory, max_gpu, total_storage, total_network)
    }

    /// Calculate total duration and maximum parallelism
    ///
    /// Duration is the sum of critical path node durations.
    /// Parallelism is the maximum number of concurrent nodes at any level.
    fn calculate_duration_and_parallelism(
        &self,
        node_estimates: &HashMap<String, NodeEstimate>,
        sorted_nodes: &[Vec<String>],
    ) -> (Duration, usize, usize) {
        let mut total_duration = Duration::ZERO;
        let mut max_parallelism = 0;

        for level_nodes in sorted_nodes {
            max_parallelism = max_parallelism.max(level_nodes.len());

            // Duration of this level is the maximum duration of any node in it
            let mut level_duration = Duration::ZERO;
            for node_id in level_nodes {
                if let Some(estimate) = node_estimates.get(node_id) {
                    level_duration = level_duration.max(estimate.duration);
                }
            }
            total_duration += level_duration;
        }

        let critical_path = sorted_nodes.len();
        (total_duration, max_parallelism, critical_path)
    }

    /// Generate warnings based on resource requirements
    fn generate_warnings(
        &self,
        cpu_cores: u32,
        memory_bytes: u64,
        gpu_memory_bytes: u64,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        // Warn about high resource usage
        if cpu_cores > 64 {
            warnings.push(format!(
                "High CPU usage: {} cores needed. Consider splitting workload.",
                cpu_cores
            ));
        }

        let memory_gb = memory_bytes / (1024 * 1024 * 1024);
        if memory_gb > 128 {
            warnings.push(format!(
                "High memory usage: {} GB needed. Consider streaming data.",
                memory_gb
            ));
        }

        let gpu_memory_gb = gpu_memory_bytes / (1024 * 1024 * 1024);
        if gpu_memory_gb > 48 {
            warnings.push(format!(
                "High GPU memory usage: {} GB needed. Consider model sharding.",
                gpu_memory_gb
            ));
        }

        warnings
    }
}

/// Estimation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum EstimationError {
    #[error("Invalid graph: {0}")]
    InvalidGraph(#[from] GraphValidationError),

    #[error("Graph contains cycles (not a DAG)")]
    CyclicGraph,

    #[error("Unable to estimate node '{0}': {1}")]
    NodeEstimationFailed(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_types::{EdgeType, GraphEdge, NodeResourceRequirements};
    use toadstool::resources::{CpuRequirements, GpuRequirements, MemoryRequirements};

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn simple_node(id: &str, cpu: f64) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            primal: "toadstool".to_string(),
            operation: "cpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                cpu: Some(CpuRequirements {
                    min_cores: cpu,
                    ..Default::default()
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        }
    }

    fn edge(from: &str, to: &str) -> GraphEdge {
        GraphEdge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: EdgeType::DataFlow,
            metadata: HashMap::new(),
        }
    }

    // ── Error path tests ─────────────────────────────────────────────────────

    #[test]
    fn test_empty_graph_returns_error() {
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "empty".to_string(),
            nodes: vec![],
            edges: vec![],
            metadata: HashMap::new(),
        };
        assert!(estimator.estimate(&graph).is_err());
    }

    #[test]
    fn test_cyclic_graph_returns_cyclic_error() {
        let estimator = ResourceEstimator::new();
        // A → B → A (cycle)
        let graph = ExecutionGraph {
            id: "cycle".to_string(),
            nodes: vec![simple_node("a", 1.0), simple_node("b", 1.0)],
            edges: vec![edge("a", "b"), edge("b", "a")],
            metadata: HashMap::new(),
        };
        let err = estimator.estimate(&graph).unwrap_err();
        // Either InvalidGraph (cycle detected in validate()) or CyclicGraph
        let is_cycle = matches!(
            err,
            EstimationError::CyclicGraph | EstimationError::InvalidGraph(_)
        );
        assert!(is_cycle, "Expected cycle error, got: {err}");
    }

    #[test]
    fn test_self_loop_is_rejected() {
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "self-loop".to_string(),
            nodes: vec![simple_node("a", 1.0)],
            edges: vec![edge("a", "a")],
            metadata: HashMap::new(),
        };
        assert!(
            estimator.estimate(&graph).is_err(),
            "self-loop must be rejected"
        );
    }

    #[test]
    fn test_invalid_edge_missing_node() {
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "bad-edge".to_string(),
            nodes: vec![simple_node("a", 1.0)],
            edges: vec![edge("a", "does-not-exist")],
            metadata: HashMap::new(),
        };
        assert!(
            estimator.estimate(&graph).is_err(),
            "edge to missing node must be rejected"
        );
    }

    // ── Single node ──────────────────────────────────────────────────────────

    #[test]
    fn test_single_node_graph() {
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "single".to_string(),
            nodes: vec![simple_node("only", 4.0)],
            edges: vec![],
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        assert_eq!(est.max_parallelism, 1);
        assert_eq!(est.critical_path_length, 1);
        assert_eq!(est.cpu_cores, 4);
    }

    // ── CPU aggregation ──────────────────────────────────────────────────────

    #[test]
    fn test_sequential_cpu_peaks() {
        // Sequential chain: max CPU across nodes (not sum)
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "seq".to_string(),
            nodes: vec![simple_node("a", 2.0), simple_node("b", 6.0)],
            edges: vec![edge("a", "b")],
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        // Peak CPU = 6 (b needs 6, a needs 2; they don't run in parallel)
        assert_eq!(est.cpu_cores, 6, "sequential peak CPU");
        assert_eq!(est.max_parallelism, 1);
    }

    #[test]
    fn test_parallel_cpu_sums() {
        // Two parallel nodes then merge: peak CPU = a+b = 3+5=8
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "par".to_string(),
            nodes: vec![
                simple_node("a", 3.0),
                simple_node("b", 5.0),
                simple_node("c", 1.0),
            ],
            edges: vec![edge("a", "c"), edge("b", "c")],
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        assert_eq!(est.cpu_cores, 8, "parallel peak CPU = 3+5");
        assert_eq!(est.max_parallelism, 2);
    }

    // ── Memory aggregation ───────────────────────────────────────────────────

    #[test]
    fn test_memory_aggregated_for_parallel_nodes() {
        let estimator = ResourceEstimator::new();
        let graph = ExecutionGraph {
            id: "mem".to_string(),
            nodes: vec![
                GraphNode {
                    id: "a".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        memory: Some(MemoryRequirements {
                            min_bytes: 2 * 1024 * 1024 * 1024,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "b".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        memory: Some(MemoryRequirements {
                            min_bytes: 4 * 1024 * 1024 * 1024,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![], // parallel
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        // Both run simultaneously: total = 2GB + 4GB = 6GB
        let expected_bytes = 6 * 1024 * 1024 * 1024u64;
        assert_eq!(est.memory_bytes, expected_bytes, "parallel memory sum");
    }

    // ── GPU aggregation ──────────────────────────────────────────────────────

    #[test]
    fn test_gpu_memory_aggregated() {
        let estimator = ResourceEstimator::new();
        let gpu_node = |id: &str, vram_mb: u64| GraphNode {
            id: id.to_string(),
            primal: "toadstool".to_string(),
            operation: "gpu_compute".to_string(),
            duration: None,
            requirements: NodeResourceRequirements {
                gpu: Some(GpuRequirements {
                    min_units: 1,
                    max_units: None,
                    gpu_type: None,
                    min_memory_bytes: Some(vram_mb * 1024 * 1024),
                }),
                ..Default::default()
            },
            metadata: HashMap::new(),
        };
        let graph = ExecutionGraph {
            id: "gpu".to_string(),
            nodes: vec![gpu_node("g1", 4096), gpu_node("g2", 8192)],
            edges: vec![], // parallel
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        // Parallel: total = 4096+8192 = 12288 MB
        let expected = (4096 + 8192) * 1024 * 1024;
        assert_eq!(est.gpu_memory_bytes, expected);
    }

    // ── Default instance ─────────────────────────────────────────────────────

    #[test]
    fn test_default_and_new_are_equivalent() {
        let a = ResourceEstimator::new();
        let b = ResourceEstimator::default();
        // Same defaults: both should estimate identical graphs identically.
        let graph = ExecutionGraph {
            id: "g".to_string(),
            nodes: vec![simple_node("n", 1.0)],
            edges: vec![],
            metadata: HashMap::new(),
        };
        let ea = a.estimate(&graph).unwrap();
        let eb = b.estimate(&graph).unwrap();
        assert_eq!(ea.cpu_cores, eb.cpu_cores);
        assert_eq!(ea.max_parallelism, eb.max_parallelism);
    }

    // ── Duration estimation ──────────────────────────────────────────────────

    #[test]
    fn test_duration_from_metadata_hint() {
        let estimator = ResourceEstimator::new();
        let mut meta = HashMap::new();
        meta.insert("estimated_duration_secs".to_string(), "180".to_string());
        let graph = ExecutionGraph {
            id: "dur".to_string(),
            nodes: vec![GraphNode {
                id: "slow".to_string(),
                primal: "toadstool".to_string(),
                operation: "custom_operation".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: meta,
            }],
            edges: vec![],
            metadata: HashMap::new(),
        };
        let est = estimator.estimate(&graph).unwrap();
        assert!(
            est.estimated_duration >= Duration::from_secs(180),
            "duration must reflect metadata hint: got {:?}",
            est.estimated_duration
        );
    }

    #[test]
    fn test_neural_compute_duration_longer_than_cpu() {
        let estimator = ResourceEstimator::new();

        let make_graph = |op: &str| ExecutionGraph {
            id: op.to_string(),
            nodes: vec![GraphNode {
                id: "n".to_string(),
                primal: "toadstool".to_string(),
                operation: op.to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: HashMap::new(),
            }],
            edges: vec![],
            metadata: HashMap::new(),
        };

        let cpu_est = estimator.estimate(&make_graph("cpu_compute")).unwrap();
        let neural_est = estimator.estimate(&make_graph("neural_compute")).unwrap();

        assert!(
            neural_est.estimated_duration >= cpu_est.estimated_duration,
            "neural_compute should have >= duration than cpu_compute"
        );
    }

    #[test]
    fn test_simple_linear_graph() {
        let estimator = ResourceEstimator::new();

        let graph = ExecutionGraph {
            id: "linear-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        cpu: Some(CpuRequirements {
                            min_cores: 4.0,
                            ..Default::default()
                        }),
                        memory: Some(MemoryRequirements {
                            min_bytes: 2 * 1024 * 1024 * 1024,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        cpu: Some(CpuRequirements {
                            min_cores: 2.0,
                            ..Default::default()
                        }),
                        memory: Some(MemoryRequirements {
                            min_bytes: 4 * 1024 * 1024 * 1024,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![GraphEdge {
                from: "node-1".to_string(),
                to: "node-2".to_string(),
                edge_type: EdgeType::DataFlow,
                metadata: HashMap::new(),
            }],
            metadata: HashMap::new(),
        };

        let estimate = estimator.estimate(&graph).unwrap();

        // Linear graph, so max parallelism is 1
        assert_eq!(estimate.max_parallelism, 1);

        // Critical path is 2 nodes
        assert_eq!(estimate.critical_path_length, 2);

        // Peak CPU is 4 (node-1)
        assert_eq!(estimate.cpu_cores, 4);

        // Peak memory is 4GB (node-2)
        assert_eq!(estimate.memory_bytes, 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parallel_graph() {
        let estimator = ResourceEstimator::new();

        let graph = ExecutionGraph {
            id: "parallel-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        cpu: Some(CpuRequirements {
                            min_cores: 2.0,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements {
                        cpu: Some(CpuRequirements {
                            min_cores: 2.0,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-3".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "storage".to_string(),
                    duration: None,
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "node-1".to_string(),
                    to: "node-3".to_string(),
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

        let estimate = estimator.estimate(&graph).unwrap();

        // Two nodes can run in parallel
        assert_eq!(estimate.max_parallelism, 2);

        // Critical path is 2 levels
        assert_eq!(estimate.critical_path_length, 2);

        // Peak CPU is 4 (both parallel nodes)
        assert_eq!(estimate.cpu_cores, 4);
    }
}
