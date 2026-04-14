// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pipeline graph -- DAG-based multi-stage compute coordination.
//!
//! Absorbed from `neuralSpring/metalForge/forge/src/graph.rs` (S134).
//!
//! Models multi-stage compute pipelines as directed acyclic graphs where:
//! - **Nodes** are capability-addressed stages (resolved via biomeOS at runtime)
//! - **Edges** are data-flow dependencies between stages
//!
//! Graphs are defined declaratively and executed in topological order.
//! Each stage specifies a `capability` string that toadStool resolves to a
//! primal at runtime, enabling dynamic routing without hardcoded service names.

use std::collections::HashMap;

/// Preferred execution substrate for a pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Substrate {
    /// CPU-only execution.
    CpuOnly,
    /// GPU-only execution.
    GpuOnly,
    /// GPU preferred, CPU fallback.
    GpuPreferred,
    /// Any available substrate.
    Any,
}

/// A single stage in a pipeline graph.
#[derive(Debug, Clone)]
pub struct StageNode {
    /// Unique identifier for this stage (e.g. `"eigensolve"`, `"compile"`).
    pub id: String,
    /// biomeOS capability string (e.g. `"science.eigensolve"`, `"gpu.dispatch"`).
    pub capability: String,
    /// Preferred execution substrate.
    pub substrate: Substrate,
    /// Human-readable label for visualization.
    pub label: String,
}

/// Result of executing a single stage.
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Stage identifier.
    pub stage_id: String,
    /// Whether the stage completed successfully.
    pub success: bool,
    /// Execution time in microseconds.
    pub elapsed_us: f64,
    /// Which substrate was actually used.
    pub actual_substrate: Substrate,
    /// Output data from this stage.
    pub output: StageOutput,
}

/// Output from a completed stage.
#[derive(Debug, Clone)]
pub enum StageOutput {
    /// Scalar result (e.g. entropy value).
    Scalar(f64),
    /// Vector result (e.g. eigenvalues).
    Vector(Vec<f64>),
    /// Named map of values.
    Map(HashMap<String, f64>),
    /// No output (side-effect only).
    Empty,
}

/// Directed acyclic graph of pipeline stages.
///
/// Stages are stored by ID, edges encode data-flow dependencies.
/// [`PipelineGraph::execute_order`] returns a topological sort.
#[derive(Debug, Clone)]
pub struct PipelineGraph {
    /// Pipeline name for logging and provenance.
    pub name: String,
    stages: Vec<StageNode>,
    /// Edges: `(from_id, to_id)` -- `from` must complete before `to`.
    edges: Vec<(String, String)>,
}

impl PipelineGraph {
    /// Create a new empty pipeline graph.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stages: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a stage to the graph.
    pub fn add_stage(&mut self, stage: StageNode) {
        self.stages.push(stage);
    }

    /// Add a dependency edge: `from` must complete before `to`.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push((from.to_string(), to.to_string()));
    }

    /// Number of stages.
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Number of dependency edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get a stage by ID.
    #[must_use]
    pub fn stage(&self, id: &str) -> Option<&StageNode> {
        self.stages.iter().find(|s| s.id == id)
    }

    /// All stages (read-only).
    #[must_use]
    pub fn stages(&self) -> &[StageNode] {
        &self.stages
    }

    /// All edges (read-only).
    #[must_use]
    pub fn edges(&self) -> &[(String, String)] {
        &self.edges
    }

    /// Compute topological execution order via Kahn's algorithm.
    ///
    /// Returns stage IDs in a valid execution order, or `None` if the
    /// graph contains a cycle (which violates the DAG invariant).
    #[must_use]
    pub fn execute_order(&self) -> Option<Vec<String>> {
        let ids: Vec<&str> = self.stages.iter().map(|s| s.id.as_str()).collect();
        let mut in_degree: HashMap<&str, usize> = ids.iter().map(|id| (*id, 0)).collect();
        let mut adjacency: HashMap<&str, Vec<&str>> =
            ids.iter().map(|id| (*id, Vec::new())).collect();

        for (from, to) in &self.edges {
            if let Some(neighbors) = adjacency.get_mut(from.as_str()) {
                neighbors.push(to.as_str());
            }
            if let Some(deg) = in_degree.get_mut(to.as_str()) {
                *deg += 1;
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort_unstable();

        let mut order = Vec::with_capacity(self.stages.len());

        while let Some(node) = queue.pop() {
            order.push(node.to_string());
            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                            queue.sort_unstable();
                        }
                    }
                }
            }
        }

        if order.len() == self.stages.len() {
            Some(order)
        } else {
            None
        }
    }

    /// Validate the graph structure.
    ///
    /// Checks:
    /// 1. All edge endpoints reference existing stages
    /// 2. No duplicate stage IDs
    /// 3. Graph is a DAG (no cycles)
    ///
    /// # Errors
    ///
    /// Returns a description of the first structural issue found.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen_ids = std::collections::HashSet::new();
        for stage in &self.stages {
            if !seen_ids.insert(&stage.id) {
                return Err(format!("duplicate stage ID: {}", stage.id));
            }
        }

        for (from, to) in &self.edges {
            if !seen_ids.contains(from) {
                return Err(format!("edge references unknown stage: {from}"));
            }
            if !seen_ids.contains(to) {
                return Err(format!("edge references unknown stage: {to}"));
            }
        }

        if self.execute_order().is_none() {
            return Err("graph contains a cycle".to_string());
        }

        Ok(())
    }
}

/// Track execution results across all stages of a pipeline.
#[derive(Debug)]
pub struct PipelineExecution {
    /// Pipeline name (from graph).
    pub pipeline_name: String,
    /// Per-stage results in execution order.
    pub results: Vec<StageResult>,
}

impl PipelineExecution {
    /// Create a new execution tracker for a pipeline.
    #[must_use]
    pub fn new(pipeline_name: &str) -> Self {
        Self {
            pipeline_name: pipeline_name.to_string(),
            results: Vec::new(),
        }
    }

    /// Record a stage result.
    pub fn record(&mut self, result: StageResult) {
        self.results.push(result);
    }

    /// Whether all recorded stages passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        !self.results.is_empty() && self.results.iter().all(|r| r.success)
    }

    /// Total execution time in microseconds.
    #[must_use]
    pub fn total_elapsed_us(&self) -> f64 {
        self.results.iter().map(|r| r.elapsed_us).sum()
    }

    /// Count of completed stages.
    #[must_use]
    pub fn completed_count(&self) -> usize {
        self.results.len()
    }

    /// Count of failed stages.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }
}

/// Build the compute-triangle pipeline: discover -> compile -> dispatch.
///
/// This is the canonical toadStool/visualization/compute pipeline.
#[must_use]
pub fn compute_triangle_pipeline() -> PipelineGraph {
    let mut g = PipelineGraph::new("compute triangle");

    g.add_stage(StageNode {
        id: "discover".to_string(),
        capability: "gpu.dispatch".to_string(),
        substrate: Substrate::CpuOnly,
        label: "GPU Discovery (toadStool)".to_string(),
    });
    g.add_stage(StageNode {
        id: "compile".to_string(),
        capability: "shader.compile".to_string(),
        substrate: Substrate::CpuOnly,
        label: "Shader Compile".to_string(),
    });
    g.add_stage(StageNode {
        id: "dispatch".to_string(),
        capability: "gpu.dispatch".to_string(),
        substrate: Substrate::GpuOnly,
        label: "GPU Dispatch".to_string(),
    });

    g.add_edge("discover", "compile");
    g.add_edge("compile", "dispatch");
    g
}

#[cfg(test)]
#[path = "pipeline_graph_tests.rs"]
mod tests;
