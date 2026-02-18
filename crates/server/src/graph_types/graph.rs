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

    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

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
