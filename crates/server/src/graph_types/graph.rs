//! Execution graph and validation logic

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::edges::GraphEdge;
use super::errors::GraphValidationError;
use super::nodes::GraphNode;

/// Execution graph representing a complete workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGraph {
    pub id: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

impl ExecutionGraph {
    /// Validate graph structure (nodes, edges, no cycles)
    ///
    /// # Errors
    ///
    /// Returns error if graph is empty, has duplicate node IDs, invalid edges, self-edges, or cycles.
    pub fn validate(&self) -> Result<(), GraphValidationError> {
        if self.nodes.is_empty() {
            return Err(GraphValidationError::EmptyGraph);
        }

        let mut seen_ids = HashSet::new();
        for node in &self.nodes {
            if !seen_ids.insert(&node.id) {
                return Err(GraphValidationError::DuplicateNodeId(node.id.clone()));
            }
        }

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
            if edge.from == edge.to {
                return Err(GraphValidationError::SelfEdge(edge.from.clone()));
            }
        }

        self.check_for_cycles()?;
        Ok(())
    }

    fn check_for_cycles(&self) -> Result<(), GraphValidationError> {
        let mut adj_list: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            adj_list.insert(&node.id, Vec::new());
        }
        for edge in &self.edges {
            if let Some(neighbors) = adj_list.get_mut(edge.from.as_str()) {
                neighbors.push(&edge.to);
            }
        }

        let mut color: HashMap<&str, Color> = HashMap::new();
        for node in &self.nodes {
            color.insert(&node.id, Color::White);
        }

        for node in &self.nodes {
            if color[node.id.as_str()] == Color::White {
                if let Err(cycle_path) = self.dfs_visit(&node.id, &adj_list, &mut color) {
                    return Err(GraphValidationError::CycleDetected(cycle_path));
                }
            }
        }

        Ok(())
    }

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
                        return Err(vec![node.to_string(), neighbor.to_string()]);
                    }
                    Color::Black => {}
                }
            }
        }

        color.insert(node, Color::Black);
        Ok(())
    }

    #[must_use]
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    #[must_use]
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

    #[must_use]
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

    pub fn builder(id: impl Into<String>) -> ExecutionGraphBuilder {
        ExecutionGraphBuilder::new(id)
    }

    pub fn simple(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Builder for ExecutionGraph
pub struct ExecutionGraphBuilder {
    id: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    metadata: HashMap<String, String>,
}

impl ExecutionGraphBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn nodes(mut self, nodes: impl IntoIterator<Item = GraphNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    pub fn edge(mut self, edge: GraphEdge) -> Self {
        self.edges.push(edge);
        self
    }

    pub fn connect(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push(GraphEdge::new(from, to));
        self
    }

    pub fn edges(mut self, edges: impl IntoIterator<Item = GraphEdge>) -> Self {
        self.edges.extend(edges);
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

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
    use crate::graph_types::GraphNode;

    fn node(id: &str) -> GraphNode {
        GraphNode::simple(id, "cpu_compute")
    }

    #[test]
    fn test_execution_graph_simple() {
        let graph = ExecutionGraph::simple("g1");
        assert_eq!(graph.id, "g1");
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_validate_empty_graph() {
        let graph = ExecutionGraph::simple("empty");
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::EmptyGraph));
    }

    #[test]
    fn test_validate_duplicate_node_id() {
        let graph = ExecutionGraph::builder("dup")
            .nodes([node("a"), node("a")])
            .build();
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::DuplicateNodeId(id) if id == "a"));
    }

    #[test]
    fn test_validate_invalid_edge_source() {
        let graph = ExecutionGraph::builder("bad-edge")
            .nodes([node("a")])
            .connect("missing", "a")
            .build();
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::InvalidEdge { .. }));
    }

    #[test]
    fn test_validate_invalid_edge_target() {
        let graph = ExecutionGraph::builder("bad-edge")
            .nodes([node("a")])
            .connect("a", "missing")
            .build();
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::InvalidEdge { .. }));
    }

    #[test]
    fn test_validate_self_edge() {
        let graph = ExecutionGraph::builder("self")
            .nodes([node("a")])
            .connect("a", "a")
            .build();
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::SelfEdge(id) if id == "a"));
    }

    #[test]
    fn test_validate_cycle() {
        let graph = ExecutionGraph::builder("cycle")
            .nodes([node("a"), node("b")])
            .connect("a", "b")
            .connect("b", "a")
            .build();
        let err = graph.validate().unwrap_err();
        assert!(matches!(err, GraphValidationError::CycleDetected(_)));
    }

    #[test]
    fn test_validate_valid_single_node() {
        let graph = ExecutionGraph::builder("ok").nodes([node("only")]).build();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_linear() {
        let graph = ExecutionGraph::builder("linear")
            .nodes([node("a"), node("b"), node("c")])
            .connect("a", "b")
            .connect("b", "c")
            .build();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_get_node() {
        let graph = ExecutionGraph::builder("g")
            .nodes([node("x"), node("y")])
            .build();
        assert_eq!(graph.get_node("x").map(|n| n.id.as_str()), Some("x"));
        assert!(graph.get_node("z").is_none());
    }

    #[test]
    fn test_get_dependents() {
        let graph = ExecutionGraph::builder("g")
            .nodes([node("a"), node("b"), node("c")])
            .connect("a", "b")
            .connect("a", "c")
            .build();
        let deps = graph.get_dependents("a");
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_get_dependencies() {
        let graph = ExecutionGraph::builder("g")
            .nodes([node("a"), node("b"), node("c")])
            .connect("a", "c")
            .connect("b", "c")
            .build();
        let deps = graph.get_dependencies("c");
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_execution_graph_builder() {
        let graph = ExecutionGraph::builder("built")
            .node(node("n1"))
            .connect("n1", "n2")
            .metadata("k", "v")
            .build();
        assert_eq!(graph.id, "built");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.metadata.get("k").map(String::as_str), Some("v"));
    }

    #[test]
    fn test_graph_validation_error_display() {
        let err = GraphValidationError::EmptyGraph;
        assert!(err.to_string().contains("empty"));
    }
}
