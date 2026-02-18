//! Graph validation errors

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
