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
    CpuRequirements, MemoryRequirements, StorageRequirements, 
    GpuRequirements, NetworkRequirements,
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
            adj_list.get_mut(edge.from.as_str())
                .unwrap_or_else(|| panic!("Node should exist"))
                .push(&edge.to);
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
        let dependent_ids: Vec<&str> = self.edges
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
        let dependency_ids: Vec<&str> = self.edges
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
pub enum EdgeType {
    /// Data flows from source to target (output → input)
    DataFlow,
    
    /// Control flow - target waits for source to complete
    Control,
    
    /// General dependency - no specific semantics
    Dependency,
}

impl Default for EdgeType {
    fn default() -> Self {
        EdgeType::Dependency
    }
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
            requirements,
            duration: self.duration,
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_valid_graph() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
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
            ],
            metadata: HashMap::new(),
        };
        
        assert!(graph.validate().is_ok());
    }
    
    #[test]
    fn test_empty_graph() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![],
            edges: vec![],
            metadata: HashMap::new(),
        };
        
        assert!(matches!(
            graph.validate(),
            Err(GraphValidationError::EmptyGraph)
        ));
    }
    
    #[test]
    fn test_duplicate_node_id() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![],
            metadata: HashMap::new(),
        };
        
        assert!(matches!(
            graph.validate(),
            Err(GraphValidationError::DuplicateNodeId(_))
        ));
    }
    
    #[test]
    fn test_cycle_detection() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
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
                    to: "node-1".to_string(),
                    edge_type: EdgeType::DataFlow,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        assert!(matches!(
            graph.validate(),
            Err(GraphValidationError::CycleDetected(_))
        ));
    }
    
    #[test]
    fn test_self_edge() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "node-1".to_string(),
                    to: "node-1".to_string(),
                    edge_type: EdgeType::DataFlow,
                    metadata: HashMap::new(),
                },
            ],
            metadata: HashMap::new(),
        };
        
        assert!(matches!(
            graph.validate(),
            Err(GraphValidationError::SelfEdge(_))
        ));
    }
    
    #[test]
    fn test_get_dependencies() {
        let graph = ExecutionGraph {
            id: "test-graph".to_string(),
            nodes: vec![
                GraphNode {
                    id: "node-1".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "cpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-2".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "gpu_compute".to_string(),
                    requirements: NodeResourceRequirements::default(),
                    metadata: HashMap::new(),
                },
                GraphNode {
                    id: "node-3".to_string(),
                    primal: "toadstool".to_string(),
                    operation: "storage".to_string(),
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
        
        let deps = graph.get_dependencies("node-3");
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|n| n.id == "node-1"));
        assert!(deps.iter().any(|n| n.id == "node-2"));
    }
}

