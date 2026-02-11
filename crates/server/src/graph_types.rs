//! Graph types for collaborative intelligence resource planning
//!
//! These types represent workflow graphs for biomeOS's collaborative intelligence system.
//! They enable human-AI collaboration by providing structured representations of
//! multi-primal workflows for resource estimation and optimization.
//!
//! ## Deep Debt Principles
//!
//! - **No Hardcoding**: Generic graph structure, no primal names hardcoded
//! - **Self-Knowledge**: Nodes describe their own requirements
//! - **Capability-Based**: Resources described by capabilities, not specifics
//! - **Runtime Discovery**: Primal availability checked at runtime
//! - **Type-Safe**: Rust type system prevents invalid graphs
//! - **Error Handling**: Result<T, E> throughout, no unwrap()

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use toadstool::resources::{
    CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};

/// Execution graph representing a complete workflow
///
/// A graph consists of nodes (workload units) connected by edges (dependencies).
/// The graph structure enables parallel execution analysis and resource estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraph {
    /// Unique graph identifier
    pub id: String,

    /// Nodes in the graph (workload units)
    pub nodes: Vec<GraphNode>,

    /// Edges connecting nodes (dependencies)
    pub edges: Vec<GraphEdge>,

    /// Optional metadata (user-provided hints)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ExecutionGraph {
    /// Validate graph structure
    ///
    /// Checks:
    /// - All nodes have unique IDs
    /// - All edges reference valid nodes
    /// - Graph is acyclic (no cycles)
    /// - No self-edges
    pub fn validate(&self) -> Result<(), GraphValidationError> {
        // Check for empty graph
        if self.nodes.is_empty() {
            return Err(GraphValidationError::EmptyGraph);
        }

        // Check for duplicate node IDs
        let mut seen_ids = HashSet::new();
        for node in &self.nodes {
            if !seen_ids.insert(&node.id) {
                return Err(GraphValidationError::DuplicateNodeId(node.id.clone()));
            }
        }

        // Check that all edges reference valid nodes
        for edge in &self.edges {
            if !seen_ids.contains(&edge.from) {
                return Err(GraphValidationError::InvalidEdge {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    reason: format!("Source node '{}' not found", edge.from),
                });
            }
            if !seen_ids.contains(&edge.to) {
                return Err(GraphValidationError::InvalidEdge {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    reason: format!("Target node '{}' not found", edge.to),
                });
            }

            // Check for self-edges
            if edge.from == edge.to {
                return Err(GraphValidationError::SelfEdge(edge.from.clone()));
            }
        }

        // Check for cycles (use DFS-based cycle detection)
        self.check_for_cycles()?;

        Ok(())
    }

    /// Check for cycles in the graph using DFS
    fn check_for_cycles(&self) -> Result<(), GraphValidationError> {
        // Build adjacency list
        let mut adj_list: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj_list.insert(&node.id, Vec::new());
        }
        for edge in &self.edges {
            if let Some(neighbors) = adj_list.get_mut(edge.from.as_str()) {
                neighbors.push(&edge.to);
            }
        }

        // DFS with three colors: white (unvisited), gray (visiting), black (visited)
        let mut color: HashMap<&str, Color> = HashMap::new();
        for node in &self.nodes {
            color.insert(&node.id, Color::White);
        }

        // Visit each unvisited node
        for node in &self.nodes {
            if color[node.id.as_str()] == Color::White {
                if let Err(cycle_path) = self.dfs_visit(&node.id, &adj_list, &mut color) {
                    return Err(GraphValidationError::CycleDetected(cycle_path));
                }
            }
        }

        Ok(())
    }

    /// DFS visit for cycle detection
    fn dfs_visit<'a>(
        &self,
        node: &'a str,
        adj_list: &HashMap<&str, Vec<&'a str>>,
        color: &mut HashMap<&'a str, Color>,
    ) -> Result<(), Vec<String>> {
        color.insert(node, Color::Gray);

        if let Some(neighbors) = adj_list.get(node) {
            for &neighbor in neighbors {
                match color[neighbor] {
                    Color::White => {
                        self.dfs_visit(neighbor, adj_list, color)?;
                    }
                    Color::Gray => {
                        // Back edge found - cycle detected
                        return Err(vec![node.to_string(), neighbor.to_string()]);
                    }
                    Color::Black => {
                        // Already visited, skip
                    }
                }
            }
        }

        color.insert(node, Color::Black);
        Ok(())
    }

    /// Get node by ID
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all nodes that depend on the given node
    pub fn get_dependents(&self, node_id: &str) -> Vec<&GraphNode> {
        let dependent_ids: Vec<&str> = self
            .edges
            .iter()
            .filter(|e| e.from == node_id)
            .map(|e| e.to.as_str())
            .collect();

        self.nodes
            .iter()
            .filter(|n| dependent_ids.contains(&n.id.as_str()))
            .collect()
    }

    /// Get all nodes that this node depends on
    pub fn get_dependencies(&self, node_id: &str) -> Vec<&GraphNode> {
        let dependency_ids: Vec<&str> = self
            .edges
            .iter()
            .filter(|e| e.to == node_id)
            .map(|e| e.from.as_str())
            .collect();

        self.nodes
            .iter()
            .filter(|n| dependency_ids.contains(&n.id.as_str()))
            .collect()
    }
}

/// Color for DFS cycle detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White, // Unvisited
    Gray,  // Visiting (in current DFS path)
    Black, // Visited (completed)
}

/// Graph node representing a single workload unit
///
/// Each node represents a primal operation with resource requirements.
/// Nodes are self-describing - they contain all information needed for
/// resource estimation without external knowledge.
///
/// ## Modern Idiomatic Rust
///
/// Use builder pattern for ergonomic construction:
/// ```rust,ignore
/// let node = GraphNode::builder("my_node", "gpu_compute")
///     .cpu(4.0)
///     .memory_gb(8)
///     .gpu_memory_gb(16)
///     .duration_secs(60)
///     .build();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique node identifier
    pub id: String,

    /// Primal name (e.g., "toadstool", "squirrel", "nestgate")
    /// This is self-knowledge - the node knows which primal it needs
    /// Defaults to "toadstool" if not specified
    #[serde(default = "default_primal")]
    pub primal: String,

    /// Operation type (e.g., "gpu_compute", "cpu_compute", "storage")
    pub operation: String,

    /// Resource requirements for this node
    #[serde(default)]
    pub requirements: NodeResourceRequirements,

    /// Estimated execution duration (type-safe)
    /// Replaces duration_secs in metadata for better ergonomics
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub duration: Option<Duration>,

    /// Optional metadata (workload hints, model size, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_primal() -> String {
    "toadstool".to_string()
}

fn serialize_duration<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match duration {
        Some(d) => serializer.serialize_u64(d.as_secs()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let secs: Option<u64> = Option::deserialize(deserializer)?;
    Ok(secs.map(Duration::from_secs))
}

/// Resource requirements for a graph node
///
/// Uses Option for all fields to allow partial specification.
/// Estimation logic will provide sensible defaults for missing requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeResourceRequirements {
    /// CPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu: Option<CpuRequirements>,

    /// Memory requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryRequirements>,

    /// Storage requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageRequirements>,

    /// GPU requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<GpuRequirements>,

    /// Network requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkRequirements>,
}

/// Dependency edge between nodes
///
/// Edges represent execution dependencies. The graph executor must ensure
/// that all dependencies of a node are completed before starting that node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node ID (dependency)
    pub from: String,

    /// Target node ID (dependent)
    pub to: String,

    /// Edge type (data flow, control flow, or general dependency)
    /// Defaults to Dependency if not specified
    #[serde(default)]
    pub edge_type: EdgeType,

    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl GraphEdge {
    /// Create a simple dependency edge
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::Dependency,
            metadata: HashMap::new(),
        }
    }

    /// Create a data flow edge
    pub fn data_flow(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::DataFlow,
            metadata: HashMap::new(),
        }
    }

    /// Create a control flow edge
    pub fn control(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            edge_type: EdgeType::Control,
            metadata: HashMap::new(),
        }
    }
}

/// Type of dependency edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum EdgeType {
    /// Data flows from source to target (output → input)
    DataFlow,

    /// Control flow - target waits for source to complete
    Control,

    /// General dependency - no specific semantics
    #[default]
    Dependency,
}

/// Graph validation error
#[derive(Debug, Clone, thiserror::Error)]
pub enum GraphValidationError {
    #[error("Graph is empty (no nodes)")]
    EmptyGraph,

    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(String),

    #[error("Invalid edge from '{from}' to '{to}': {reason}")]
    InvalidEdge {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Self-edge detected on node '{0}'")]
    SelfEdge(String),

    #[error("Cycle detected in graph: {}", .0.join(" -> "))]
    CycleDetected(Vec<String>),
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Builder Patterns - Modern Idiomatic Rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

impl GraphNode {
    /// Create a builder for ergonomic node construction
    ///
    /// # Example
    /// ```rust,ignore
    /// let node = GraphNode::builder("my_node", "gpu_compute")
    ///     .cpu(4.0)
    ///     .memory_gb(8)
    ///     .gpu_memory_gb(16)
    ///     .duration_secs(60)
    ///     .build();
    /// ```
    pub fn builder(id: impl Into<String>, operation: impl Into<String>) -> GraphNodeBuilder {
        GraphNodeBuilder::new(id, operation)
    }

    /// Create a simple node with defaults
    pub fn simple(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: "toadstool".to_string(),
            operation: operation.into(),
            requirements: NodeResourceRequirements::default(),
            duration: None,
            metadata: HashMap::new(),
        }
    }
}

/// Builder for GraphNode with fluent API
///
/// Provides ergonomic construction of graph nodes with sensible defaults.
pub struct GraphNodeBuilder {
    id: String,
    primal: String,
    operation: String,
    cpu_cores: Option<f64>,
    memory_bytes: Option<u64>,
    gpu_memory_bytes: Option<u64>,
    storage_bytes: Option<u64>,
    network_bandwidth_mbps: Option<u64>,
    duration: Option<Duration>,
    metadata: HashMap<String, String>,
}

impl GraphNodeBuilder {
    /// Create a new builder
    pub fn new(id: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            primal: "toadstool".to_string(),
            operation: operation.into(),
            cpu_cores: None,
            memory_bytes: None,
            gpu_memory_bytes: None,
            storage_bytes: None,
            network_bandwidth_mbps: None,
            duration: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the primal name
    pub fn primal(mut self, primal: impl Into<String>) -> Self {
        self.primal = primal.into();
        self
    }

    /// Set CPU requirements (in cores)
    pub fn cpu(mut self, cores: f64) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Set memory requirements (in bytes)
    pub fn memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    /// Set memory requirements (in GB, for convenience)
    pub fn memory_gb(mut self, gb: u64) -> Self {
        self.memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set GPU memory requirements (in bytes)
    pub fn gpu_memory(mut self, bytes: u64) -> Self {
        self.gpu_memory_bytes = Some(bytes);
        self
    }

    /// Set GPU memory requirements (in GB, for convenience)
    pub fn gpu_memory_gb(mut self, gb: u64) -> Self {
        self.gpu_memory_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set storage requirements (in bytes)
    pub fn storage(mut self, bytes: u64) -> Self {
        self.storage_bytes = Some(bytes);
        self
    }

    /// Set storage requirements (in GB, for convenience)
    pub fn storage_gb(mut self, gb: u64) -> Self {
        self.storage_bytes = Some(gb * 1024 * 1024 * 1024);
        self
    }

    /// Set network bandwidth requirements (in Mbps)
    pub fn network_bandwidth(mut self, mbps: u64) -> Self {
        self.network_bandwidth_mbps = Some(mbps);
        self
    }

    /// Set estimated duration (as Duration)
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Set estimated duration (in seconds, for convenience)
    pub fn duration_secs(mut self, secs: u64) -> Self {
        self.duration = Some(Duration::from_secs(secs));
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the GraphNode
    pub fn build(self) -> GraphNode {
        let mut requirements = NodeResourceRequirements::default();

        if let Some(cores) = self.cpu_cores {
            requirements.cpu = Some(CpuRequirements {
                min_cores: cores,
                max_cores: None,
                architecture: None,
            });
        }

        if let Some(bytes) = self.memory_bytes {
            requirements.memory = Some(MemoryRequirements {
                min_bytes: bytes,
                max_bytes: None,
            });
        }

        if let Some(bytes) = self.gpu_memory_bytes {
            requirements.gpu = Some(GpuRequirements {
                min_units: 1,
                max_units: None,
                gpu_type: None,
                min_memory_bytes: Some(bytes),
            });
        }

        if let Some(bytes) = self.storage_bytes {
            requirements.storage = Some(StorageRequirements {
                min_bytes: bytes,
                max_bytes: None,
                storage_type: None,
            });
        }

        if let Some(mbps) = self.network_bandwidth_mbps {
            // Convert Mbps to bytes per second (Mbps * 125000)
            let bytes_per_sec = mbps * 125000;
            requirements.network = Some(NetworkRequirements {
                min_bandwidth: Some(bytes_per_sec),
                max_bandwidth: None,
                max_latency_ms: None,
            });
        }

        GraphNode {
            id: self.id,
            primal: self.primal,
            operation: self.operation,
            duration: self.duration,
            requirements,
            metadata: self.metadata,
        }
    }
}

impl ExecutionGraph {
    /// Create a builder for ergonomic graph construction
    pub fn builder(id: impl Into<String>) -> ExecutionGraphBuilder {
        ExecutionGraphBuilder::new(id)
    }

    /// Create a simple graph
    pub fn simple(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Builder for ExecutionGraph with fluent API
pub struct ExecutionGraphBuilder {
    id: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    metadata: HashMap<String, String>,
}

impl ExecutionGraphBuilder {
    /// Create a new builder
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a node
    pub fn node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    /// Add multiple nodes
    pub fn nodes(mut self, nodes: impl IntoIterator<Item = GraphNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    /// Add an edge
    pub fn edge(mut self, edge: GraphEdge) -> Self {
        self.edges.push(edge);
        self
    }

    /// Add a simple dependency edge
    pub fn connect(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push(GraphEdge::new(from, to));
        self
    }

    /// Add multiple edges
    pub fn edges(mut self, edges: impl IntoIterator<Item = GraphEdge>) -> Self {
        self.edges.extend(edges);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Build the ExecutionGraph
    pub fn build(self) -> ExecutionGraph {
        ExecutionGraph {
            id: self.id,
            nodes: self.nodes,
            edges: self.edges,
            metadata: self.metadata,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Unit Tests – Cover production code not reached by integration tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    // ───── Validation ───────────────────────────────────────────────────────

    #[test]
    fn validate_empty_graph() {
        let g = ExecutionGraph::simple("x");
        assert!(matches!(
            g.validate(),
            Err(GraphValidationError::EmptyGraph)
        ));
    }

    #[test]
    fn validate_duplicate_node_id() {
        let g = ExecutionGraph::builder("x")
            .nodes([GraphNode::simple("a", "op"), GraphNode::simple("a", "op2")])
            .build();
        assert!(matches!(
            g.validate(),
            Err(GraphValidationError::DuplicateNodeId(_))
        ));
    }

    #[test]
    fn validate_invalid_edge_source() {
        let g = ExecutionGraph::builder("x")
            .node(GraphNode::simple("b", "op"))
            .connect("missing", "b")
            .build();
        let e = g.validate().unwrap_err();
        match &e {
            GraphValidationError::InvalidEdge { from, reason, .. } => {
                assert_eq!(from, "missing");
                assert!(reason.contains("Source node"));
            }
            _ => panic!("{:?}", e),
        }
    }

    #[test]
    fn validate_invalid_edge_target() {
        let g = ExecutionGraph::builder("x")
            .node(GraphNode::simple("a", "op"))
            .connect("a", "missing")
            .build();
        let e = g.validate().unwrap_err();
        match &e {
            GraphValidationError::InvalidEdge { to, reason, .. } => {
                assert_eq!(to, "missing");
                assert!(reason.contains("Target node"));
            }
            _ => panic!("{:?}", e),
        }
    }

    #[test]
    fn validate_self_edge() {
        let g = ExecutionGraph::builder("x")
            .node(GraphNode::simple("a", "op"))
            .connect("a", "a")
            .build();
        assert!(matches!(
            g.validate(),
            Err(GraphValidationError::SelfEdge(_))
        ));
    }

    #[test]
    fn validate_cycle_two_nodes() {
        let g = ExecutionGraph::builder("x")
            .nodes([GraphNode::simple("a", "op1"), GraphNode::simple("b", "op2")])
            .connect("a", "b")
            .connect("b", "a")
            .build();
        assert!(matches!(
            g.validate(),
            Err(GraphValidationError::CycleDetected(_))
        ));
    }

    #[test]
    fn validate_cycle_three_nodes() {
        let g = ExecutionGraph::builder("x")
            .nodes([
                GraphNode::simple("a", "op1"),
                GraphNode::simple("b", "op2"),
                GraphNode::simple("c", "op3"),
            ])
            .connect("a", "b")
            .connect("b", "c")
            .connect("c", "a")
            .build();
        let e = g.validate().unwrap_err();
        match &e {
            GraphValidationError::CycleDetected(p) => assert_eq!(p.len(), 2),
            _ => panic!("{:?}", e),
        }
    }

    #[test]
    fn validate_valid_single_node() {
        let g = ExecutionGraph::builder("x")
            .node(GraphNode::simple("a", "op"))
            .build();
        assert!(g.validate().is_ok());
    }

    #[test]
    fn validate_valid_dag_diamond() {
        let g = ExecutionGraph::builder("x")
            .nodes([
                GraphNode::simple("a", "op1"),
                GraphNode::simple("b", "op2"),
                GraphNode::simple("c", "op3"),
            ])
            .connect("a", "b")
            .connect("a", "c")
            .connect("c", "b")
            .build();
        assert!(g.validate().is_ok(), "diamond with cross-edge is DAG");
    }

    #[test]
    fn validate_node_without_neighbors_dfs() {
        let g = ExecutionGraph::builder("x")
            .node(GraphNode::simple("sink", "op"))
            .build();
        assert!(g.validate().is_ok());
    }

    // ───── ExecutionGraph methods ───────────────────────────────────────────

    #[test]
    fn get_node_found() {
        let g = ExecutionGraph::builder("g")
            .nodes([GraphNode::simple("a", "op1"), GraphNode::simple("b", "op2")])
            .connect("a", "b")
            .build();
        let n = g.get_node("a").unwrap();
        assert_eq!(n.id, "a");
        assert_eq!(n.operation, "op1");
    }

    #[test]
    fn get_node_not_found() {
        let g = ExecutionGraph::builder("g")
            .node(GraphNode::simple("a", "op"))
            .build();
        assert!(g.get_node("z").is_none());
    }

    #[test]
    fn get_dependencies() {
        let g = ExecutionGraph::builder("g")
            .nodes([
                GraphNode::simple("a", "op1"),
                GraphNode::simple("b", "op2"),
                GraphNode::simple("c", "op3"),
            ])
            .connect("a", "c")
            .connect("b", "c")
            .build();
        let deps = g.get_dependencies("c");
        assert_eq!(deps.len(), 2);
        let ids: Vec<&str> = deps.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"b"));
    }

    #[test]
    fn get_dependents() {
        let g = ExecutionGraph::builder("g")
            .nodes([
                GraphNode::simple("a", "op1"),
                GraphNode::simple("b", "op2"),
                GraphNode::simple("c", "op3"),
            ])
            .connect("a", "b")
            .connect("a", "c")
            .build();
        let dep = g.get_dependents("a");
        assert_eq!(dep.len(), 2);
        let ids: Vec<&str> = dep.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"b"));
        assert!(ids.contains(&"c"));
    }

    #[test]
    fn get_dependents_empty() {
        let g = ExecutionGraph::builder("g")
            .nodes([GraphNode::simple("a", "op1"), GraphNode::simple("b", "op2")])
            .connect("a", "b")
            .build();
        assert!(g.get_dependents("b").is_empty());
    }

    // ───── GraphNode constructors and builder ───────────────────────────────

    #[test]
    fn graph_node_simple() {
        let n = GraphNode::simple("id", "op");
        assert_eq!(n.id, "id");
        assert_eq!(n.primal, "toadstool");
        assert_eq!(n.operation, "op");
        assert!(n.requirements.cpu.is_none());
        assert!(n.duration.is_none());
    }

    #[test]
    fn graph_node_builder_minimal() {
        let n = GraphNode::builder("id", "op").build();
        assert_eq!(n.id, "id");
        assert_eq!(n.primal, "toadstool");
        assert_eq!(n.operation, "op");
    }

    #[test]
    fn graph_node_builder_primal() {
        let n = GraphNode::builder("id", "op").primal("squirrel").build();
        assert_eq!(n.primal, "squirrel");
    }

    #[test]
    fn graph_node_builder_cpu() {
        let n = GraphNode::builder("id", "op").cpu(4.0).build();
        assert_eq!(n.requirements.cpu.as_ref().unwrap().min_cores, 4.0);
    }

    #[test]
    fn graph_node_builder_memory() {
        let n = GraphNode::builder("id", "op")
            .memory(2 * 1024 * 1024 * 1024)
            .build();
        assert_eq!(
            n.requirements.memory.as_ref().unwrap().min_bytes,
            2_u64 << 30
        );
    }

    #[test]
    fn graph_node_builder_memory_gb() {
        let n = GraphNode::builder("id", "op").memory_gb(8).build();
        assert_eq!(
            n.requirements.memory.as_ref().unwrap().min_bytes,
            8 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn graph_node_builder_gpu_memory() {
        let n = GraphNode::builder("id", "op")
            .gpu_memory(16 * 1024 * 1024 * 1024)
            .build();
        assert_eq!(
            n.requirements.gpu.as_ref().unwrap().min_memory_bytes,
            Some(16 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn graph_node_builder_gpu_memory_gb() {
        let n = GraphNode::builder("id", "op").gpu_memory_gb(24).build();
        assert_eq!(
            n.requirements.gpu.as_ref().unwrap().min_memory_bytes,
            Some(24 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn graph_node_builder_storage() {
        let n = GraphNode::builder("id", "op")
            .storage(500 * 1024 * 1024)
            .build();
        assert_eq!(
            n.requirements.storage.as_ref().unwrap().min_bytes,
            500 * 1024 * 1024
        );
    }

    #[test]
    fn graph_node_builder_storage_gb() {
        let n = GraphNode::builder("id", "op").storage_gb(100).build();
        assert_eq!(
            n.requirements.storage.as_ref().unwrap().min_bytes,
            100 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn graph_node_builder_network_bandwidth() {
        let n = GraphNode::builder("id", "op")
            .network_bandwidth(1000)
            .build();
        assert_eq!(
            n.requirements.network.as_ref().unwrap().min_bandwidth,
            Some(1000 * 125000)
        );
    }

    #[test]
    fn graph_node_builder_duration() {
        let n = GraphNode::builder("id", "op")
            .duration(Duration::from_secs(90))
            .build();
        assert_eq!(n.duration, Some(Duration::from_secs(90)));
    }

    #[test]
    fn graph_node_builder_duration_secs() {
        let n = GraphNode::builder("id", "op").duration_secs(120).build();
        assert_eq!(n.duration, Some(Duration::from_secs(120)));
    }

    #[test]
    fn graph_node_builder_metadata() {
        let n = GraphNode::builder("id", "op")
            .metadata("k", "v")
            .metadata("k2", "v2")
            .build();
        assert_eq!(n.metadata.get("k"), Some(&"v".to_string()));
        assert_eq!(n.metadata.get("k2"), Some(&"v2".to_string()));
    }

    #[test]
    fn graph_node_builder_full() {
        let n = GraphNode::builder("n", "gpu_compute")
            .primal("nestgate")
            .cpu(8.0)
            .memory_gb(32)
            .gpu_memory_gb(80)
            .storage_gb(500)
            .network_bandwidth(10_000)
            .duration_secs(3600)
            .metadata("model", "llama")
            .build();
        assert_eq!(n.primal, "nestgate");
        assert_eq!(n.requirements.cpu.as_ref().unwrap().min_cores, 8.0);
        assert_eq!(
            n.requirements.memory.as_ref().unwrap().min_bytes,
            32 * 1024 * 1024 * 1024
        );
        assert_eq!(
            n.requirements.gpu.as_ref().unwrap().min_memory_bytes,
            Some(80 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            n.requirements.storage.as_ref().unwrap().min_bytes,
            500 * 1024 * 1024 * 1024
        );
        assert_eq!(
            n.requirements.network.as_ref().unwrap().min_bandwidth,
            Some(10_000 * 125000)
        );
        assert_eq!(n.duration, Some(Duration::from_secs(3600)));
        assert_eq!(n.metadata.get("model"), Some(&"llama".to_string()));
    }

    // ───── ExecutionGraph constructors and builder ──────────────────────────

    #[test]
    fn execution_graph_simple() {
        let g = ExecutionGraph::simple("empty");
        assert_eq!(g.id, "empty");
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn execution_graph_builder() {
        let g = ExecutionGraph::builder("my-graph")
            .node(GraphNode::simple("n1", "op1"))
            .nodes([
                GraphNode::simple("n2", "op2"),
                GraphNode::simple("n3", "op3"),
            ])
            .connect("n1", "n2")
            .edge(GraphEdge::data_flow("n2", "n3"))
            .edges([GraphEdge::control("n1", "n3")])
            .metadata("key", "value")
            .build();
        assert_eq!(g.id, "my-graph");
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 3);
        assert_eq!(g.metadata.get("key"), Some(&"value".to_string()));
        assert!(g.validate().is_ok());
    }

    // ───── GraphEdge constructors ──────────────────────────────────────────

    #[test]
    fn graph_edge_new() {
        let e = GraphEdge::new("a", "b");
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
        assert_eq!(e.edge_type, EdgeType::Dependency);
    }

    #[test]
    fn graph_edge_data_flow() {
        let e = GraphEdge::data_flow("x", "y");
        assert_eq!(e.edge_type, EdgeType::DataFlow);
    }

    #[test]
    fn graph_edge_control() {
        let e = GraphEdge::control("x", "y");
        assert_eq!(e.edge_type, EdgeType::Control);
    }

    #[test]
    fn graph_edge_with_string() {
        let e = GraphEdge::new(String::from("a"), String::from("b"));
        assert_eq!(e.from, "a");
        assert_eq!(e.to, "b");
    }

    // ───── EdgeType ────────────────────────────────────────────────────────

    #[test]
    fn edge_type_default() {
        let et: EdgeType = Default::default();
        assert_eq!(et, EdgeType::Dependency);
    }

    // ───── NodeResourceRequirements ────────────────────────────────────────

    #[test]
    fn node_resource_requirements_default() {
        let r = NodeResourceRequirements::default();
        assert!(r.cpu.is_none());
        assert!(r.memory.is_none());
        assert!(r.storage.is_none());
        assert!(r.gpu.is_none());
        assert!(r.network.is_none());
    }

    // ───── Serialization ───────────────────────────────────────────────────

    #[test]
    fn serialize_graph_roundtrip() {
        let g = ExecutionGraph::builder("g")
            .nodes([
                GraphNode::builder("n1", "op").duration_secs(60).build(),
                GraphNode::simple("n2", "op2"),
            ])
            .connect("n1", "n2")
            .build();
        let json = serde_json::to_string(&g).unwrap();
        let r: ExecutionGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(g.id, r.id);
        assert_eq!(g.nodes.len(), r.nodes.len());
        assert_eq!(g.edges.len(), r.edges.len());
    }

    #[test]
    fn serialize_node_duration_some() {
        let n = GraphNode::builder("n", "op").duration_secs(300).build();
        let json = serde_json::to_string(&n).unwrap();
        assert!(json.contains("300"));
        let r: GraphNode = serde_json::from_str(&json).unwrap();
        assert_eq!(r.duration, Some(Duration::from_secs(300)));
    }

    #[test]
    fn serialize_node_duration_none_omitted() {
        let n = GraphNode::simple("n", "op");
        let json = serde_json::to_string(&n).unwrap();
        assert!(!json.contains("duration"));
        let r: GraphNode = serde_json::from_str(&json).unwrap();
        assert!(r.duration.is_none());
    }

    #[test]
    fn deserialize_node_default_primal() {
        let json = r#"{"id":"n","operation":"op"}"#;
        let n: GraphNode = serde_json::from_str(json).unwrap();
        assert_eq!(n.primal, "toadstool");
    }

    #[test]
    fn deserialize_node_duration_null() {
        let json = r#"{"id":"n","operation":"op","duration":null}"#;
        let n: GraphNode = serde_json::from_str(json).unwrap();
        assert!(n.duration.is_none());
    }

    #[test]
    fn serialize_edge_roundtrip() {
        let e = GraphEdge::data_flow("a", "b");
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("data_flow"));
        let r: GraphEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(e.from, r.from);
        assert_eq!(e.to, r.to);
        assert_eq!(e.edge_type, r.edge_type);
    }

    #[test]
    fn serialize_edge_type_snake_case() {
        let json = serde_json::to_string(&EdgeType::DataFlow).unwrap();
        assert_eq!(json, "\"data_flow\"");
        let r: EdgeType = serde_json::from_str(&json).unwrap();
        assert_eq!(r, EdgeType::DataFlow);
    }

    #[test]
    fn serialize_execution_graph_missing_metadata_default() {
        let json = r#"{"id":"g1","nodes":[],"edges":[]}"#;
        let g: ExecutionGraph = serde_json::from_str(json).unwrap();
        assert!(g.metadata.is_empty());
    }

    // ───── Display / Debug ─────────────────────────────────────────────────

    #[test]
    fn graph_validation_error_display() {
        assert_eq!(
            GraphValidationError::EmptyGraph.to_string(),
            "Graph is empty (no nodes)"
        );
        assert!(GraphValidationError::DuplicateNodeId("x".into())
            .to_string()
            .contains("Duplicate node ID"));
        assert!(GraphValidationError::SelfEdge("n".into())
            .to_string()
            .contains("Self-edge"));
        assert!(GraphValidationError::InvalidEdge {
            from: "a".into(),
            to: "b".into(),
            reason: "reason".into(),
        }
        .to_string()
        .contains("Invalid edge"));
        assert!(
            GraphValidationError::CycleDetected(vec!["a".into(), "b".into()])
                .to_string()
                .contains("Cycle detected")
        );
    }

    #[test]
    fn graph_validation_error_debug() {
        let _ = format!("{:?}", GraphValidationError::EmptyGraph);
    }

    #[test]
    fn debug_execution_graph() {
        let g = ExecutionGraph::simple("g");
        let _ = format!("{:?}", g);
    }

    #[test]
    fn debug_graph_node() {
        let n = GraphNode::simple("n", "op");
        let _ = format!("{:?}", n);
    }

    #[test]
    fn debug_graph_edge() {
        let e = GraphEdge::new("a", "b");
        let _ = format!("{:?}", e);
    }

    #[test]
    fn debug_node_resource_requirements() {
        let r = NodeResourceRequirements::default();
        let _ = format!("{:?}", r);
    }

    // ───── Clone ───────────────────────────────────────────────────────────

    #[test]
    fn clone_execution_graph() {
        let g = ExecutionGraph::builder("g")
            .node(GraphNode::simple("a", "op"))
            .build();
        let c = g.clone();
        assert_eq!(g.id, c.id);
        assert_eq!(g.nodes.len(), c.nodes.len());
    }

    #[test]
    fn clone_graph_node() {
        let n = GraphNode::builder("n", "op").cpu(2.0).build();
        let c = n.clone();
        assert_eq!(n.id, c.id);
        assert_eq!(c.requirements.cpu.as_ref().unwrap().min_cores, 2.0);
    }

    #[test]
    fn clone_graph_edge() {
        let e = GraphEdge::data_flow("a", "b");
        let c = e.clone();
        assert_eq!(e.from, c.from);
        assert_eq!(e.edge_type, c.edge_type);
    }
}
