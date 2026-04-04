// SPDX-License-Identifier: AGPL-3.0-only
//! Core resource estimation logic
//!
//! Analyzes execution graphs and produces resource estimates using topological
//! sort, per-node estimation, and aggregation.

use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use tracing::{debug, info};

use crate::graph_types::{ExecutionGraph, GraphNode};

use super::types::{EstimationError, NodeEstimate, ResourceEstimate};

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
    /// # Errors
    ///
    /// Returns error if graph validation fails or topological sort detects a cycle.
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
    fn topological_sort(
        &self,
        graph: &ExecutionGraph,
    ) -> Result<Vec<Vec<String>>, EstimationError> {
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

        let mut queue: VecDeque<String> = VecDeque::new();
        let mut levels: Vec<Vec<String>> = Vec::new();
        let mut visited = 0;

        for (node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id.clone());
            }
        }

        while !queue.is_empty() {
            let level_size = queue.len();
            let mut current_level = Vec::new();

            for _ in 0..level_size {
                let Some(node_id) = queue.pop_front() else {
                    break;
                };
                current_level.push(node_id.clone());
                visited += 1;

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

        if visited != graph.nodes.len() {
            return Err(EstimationError::CyclicGraph);
        }

        Ok(levels)
    }

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

    fn estimate_node(&self, node: &GraphNode, level: usize) -> NodeEstimate {
        let cpu_cores = node
            .requirements
            .cpu
            .as_ref()
            .map(|r| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "truncation acceptable for this conversion"
                )]
                u32::try_from(r.min_cores.round() as i64).unwrap_or(self.default_cpu_cores)
            })
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

    fn estimate_duration(&self, node: &GraphNode) -> Duration {
        if let Some(duration_str) = node.metadata.get("estimated_duration_secs")
            && let Ok(secs) = duration_str.parse::<u64>()
        {
            return Duration::from_secs(secs);
        }

        match node.operation.as_str() {
            "gpu_compute" => Duration::from_secs(60),
            "neural_compute" => Duration::from_secs(120),
            "cpu_compute" => Duration::from_secs(30),
            "storage" => Duration::from_secs(10),
            "network" => Duration::from_secs(5),
            _ => self.default_duration,
        }
    }

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

        for level_nodes in sorted_nodes {
            let mut level_cpu = 0;
            let mut level_memory = 0;
            let mut level_gpu = 0;

            for node_id in level_nodes {
                if let Some(estimate) = node_estimates.get(node_id) {
                    level_cpu += estimate.cpu_cores;
                    level_memory += estimate.memory_bytes;
                    level_gpu += estimate.gpu_memory_bytes;
                    total_storage += estimate.memory_bytes;
                    total_network += 100;
                }
            }

            max_cpu = max_cpu.max(level_cpu);
            max_memory = max_memory.max(level_memory);
            max_gpu = max_gpu.max(level_gpu);
        }

        (max_cpu, max_memory, max_gpu, total_storage, total_network)
    }

    fn calculate_duration_and_parallelism(
        &self,
        node_estimates: &HashMap<String, NodeEstimate>,
        sorted_nodes: &[Vec<String>],
    ) -> (Duration, usize, usize) {
        let mut total_duration = Duration::ZERO;
        let mut max_parallelism = 0;

        for level_nodes in sorted_nodes {
            max_parallelism = max_parallelism.max(level_nodes.len());

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

    fn generate_warnings(
        &self,
        cpu_cores: u32,
        memory_bytes: u64,
        gpu_memory_bytes: u64,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        if cpu_cores > 64 {
            warnings.push(format!(
                "High CPU usage: {cpu_cores} cores needed. Consider splitting workload."
            ));
        }

        let memory_gb = memory_bytes / (1024 * 1024 * 1024);
        if memory_gb > 128 {
            warnings.push(format!(
                "High memory usage: {memory_gb} GB needed. Consider streaming data."
            ));
        }

        let gpu_memory_gb = gpu_memory_bytes / (1024 * 1024 * 1024);
        if gpu_memory_gb > 48 {
            warnings.push(format!(
                "High GPU memory usage: {gpu_memory_gb} GB needed. Consider model sharding."
            ));
        }

        warnings
    }
}
