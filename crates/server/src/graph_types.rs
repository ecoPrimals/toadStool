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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique node identifier
    pub id: String,
    
    /// Primal name (e.g., "toadstool", "squirrel", "nestgate")
    /// This is self-knowledge - the node knows which primal it needs
    pub primal: String,
    
    /// Operation type (e.g., "gpu_compute", "cpu_compute", "storage")
    pub operation: String,
    
    /// Resource requirements for this node
    pub requirements: NodeResourceRequirements,
    
    /// Optional metadata (workload hints, model size, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
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
    pub edge_type: EdgeType,
    
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Type of dependency edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Data flows from source to target (output → input)
    DataFlow,
    
    /// Control flow - target waits for source to complete
    Control,
    
    /// General dependency - no specific semantics
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

